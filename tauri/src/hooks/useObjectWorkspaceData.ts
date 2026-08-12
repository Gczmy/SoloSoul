import { useState, useEffect, useCallback, useRef, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { useObjectStore, type ObjectSummary, type ObjectData } from '@/stores/objectStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { useTemplateStore } from '@/stores/templateStore';
import { useAuthStore } from '@/stores/authStore';
import type { DeprecatedField } from '@/lib/templateSync';
import { DEBOUNCE_DELAY_MS } from '@/lib/constants';
import { logger } from '@/lib/logger';
import { useWorkspacePasswordGuard } from './useWorkspacePasswordGuard';
import { useTemplateFieldMeta } from './useTemplateFieldMeta';
import { useWorkspaceTemplateSync } from './useWorkspaceTemplateSync';

// P011: 对象卡片列表分页大小。注意：分页仅优化 DOM 挂载，
// snapshot/attachment 计数 IPC 仍对全量 visibleObjects 批量请求（单次 batch，成本可控）。
const OBJECT_PAGE_SIZE = 50;

export interface UseObjectWorkspaceDataOptions {
  pageId?: string;
  sectionFilter: string;
  detailObjectId: string | null;
}

/** ObjectWorkspacePage 的完整数据层与业务逻辑（W005 再拆后为组合层）。 */
export function useObjectWorkspaceData({
  pageId,
  sectionFilter,
  detailObjectId,
}: UseObjectWorkspaceDataOptions) {
  const { t } = useTranslation(['common', 'navigation', 'editor']);
  const accountId = useAuthStore((s) => s.currentAccount?.id);

  const [searchQuery, setSearchQuery] = useState('');
  const [debouncedSearchQuery, setDebouncedSearchQuery] = useState('');
  const [, setDeletingId] = useState<string | null>(null);
  const [confirmDelete, setConfirmDelete] = useState<{ id: string; name: string } | null>(null);
  const [confirmPageDelete, setConfirmPageDelete] = useState(false);
  const [historyObj, setHistoryObj] = useState<{
    id: string;
    name: string;
    typeId: string;
    templateId?: string;
  } | null>(null);
  const [snapshotCounts, setSnapshotCounts] = useState<Record<string, number>>({});
  const [attachmentObjId, setAttachmentObjId] = useState<string | null>(null);
  const [attachmentCounts, setAttachmentCounts] = useState<Record<string, number>>({});
  const [detailObj, setDetailObj] = useState<(ObjectSummary | ObjectData) | null>(null);

  // 历史字段查看器状态
  const [deprecatedViewer, setDeprecatedViewer] = useState<{
    objectId: string;
    objectName: string;
  } | null>(null);
  const [deprecatedFields, setDeprecatedFields] = useState<DeprecatedField[]>([]);

  // P010: 字段级 selector 订阅，避免整店订阅导致任何 store 变化都触发整页重渲染。
  const objects = useObjectStore((s) => s.objects);
  const isLoading = useObjectStore((s) => s.isLoading);
  const error = useObjectStore((s) => s.error);
  const loadObjects = useObjectStore((s) => s.loadObjects);
  const deleteObject = useObjectStore((s) => s.deleteObject);
  const previewSyncTemplate = useObjectStore((s) => s.previewSyncTemplate);
  const applySyncTemplate = useObjectStore((s) => s.applySyncTemplate);
  const ignoreTemplateSync = useObjectStore((s) => s.ignoreTemplateSync);
  const loadDeprecatedFields = useObjectStore((s) => s.loadDeprecatedFields);

  // P011: 对象卡片列表分页「加载更多」，避免数百个对象一次全量挂载。
  const [visibleLimit, setVisibleLimit] = useState(OBJECT_PAGE_SIZE);
  // 搜索词或页面变化时重置分页游标。
  useEffect(() => {
    setVisibleLimit(OBJECT_PAGE_SIZE);
  }, [debouncedSearchQuery, pageId, sectionFilter]);

  const customPages = useSettingsStore((s) => s.settings.customPages);
  const activeCustomPages = customPages.filter((p) => !p.deletedAt);
  const removeCustomPage = useSettingsStore((s) => s.removeCustomPage);
  // P010: 分字段 selector 订阅，避免整店订阅导致 store 任何变化触发整页重渲染。
  const userTemplates = useTemplateStore((s) => s.templates);
  const loadUserTemplates = useTemplateStore((s) => s.loadTemplates);
  const abortRef = useRef<AbortController | null>(null);

  // 组件卸载时清理 store 中的陈旧对象，防止重新挂载时首帧闪烁上个页面的内容。
  useEffect(() => {
    return () => {
      useObjectStore.setState({ objects: [] });
    };
  }, []);

  useEffect(() => {
    loadUserTemplates().catch((err) => logger.warn('[Workspace] Load templates failed:', err));
  }, [loadUserTemplates]);

  // Open object detail modal directly when navigated with ?objectId=... (e.g. from search)
  useEffect(() => {
    if (!detailObjectId || !accountId) return;
    invoke('object_get', { objectId: detailObjectId })
      .then((obj) => setDetailObj(obj as (typeof visibleObjects)[number]))
      .catch((err) => logger.warn('[Workspace] Fetch object detail failed:', err));
  }, [detailObjectId, accountId]);

  const customPage = pageId ? customPages.find((p) => p.id === pageId) : null;

  const resolveCollectionLabel = useCallback(
    (typeId: string) => {
      if (['identity', 'travel', 'financial', 'professional', 'document'].includes(typeId)) {
        return t(`navigation:${typeId}`);
      }
      const cp = customPages.find((p) => p.id === typeId);
      return cp?.name || typeId;
    },
    [t, customPages],
  );

  const activeCategoryLabel = sectionFilter
    ? t(`navigation:${sectionFilter}`, sectionFilter)
    : null;

  // P013/5: 密码守卫（详情面板/历史查看器共用）
  const {
    showPwDialog,
    setShowPwDialog,
    pwResolveRef,
    bioAvailable,
    passwordHint,
    passwordVerify,
    verifyVaultPassword,
    handleBiometricUnlock,
  } = useWorkspacePasswordGuard(accountId);

  // P013/5: 模板字段元数据查找（敏感度/废弃/显示名）
  const { getFieldSensitivity, isFieldDeprecated, getFieldName } = useTemplateFieldMeta(
    userTemplates,
  );

  useEffect(() => {
    if (accountId) {
      if (pageId) {
        loadObjects(accountId, { parentId: pageId });
      } else {
        loadObjects(accountId, sectionFilter ? { typeId: sectionFilter } : undefined);
      }
    }
  }, [accountId, sectionFilter, pageId, loadObjects]);

  // Debounce searchQuery to avoid high-frequency IPC calls on every keystroke
  useEffect(() => {
    const timer = setTimeout(() => setDebouncedSearchQuery(searchQuery), DEBOUNCE_DELAY_MS);
    return () => clearTimeout(timer);
  }, [searchQuery]);

  const visibleObjects = useMemo(
    () =>
      objects.filter(
        (obj) =>
          obj.typeId !== 'page' &&
          obj.typeId !== 'unknown' &&
          obj.name.toLowerCase().includes(debouncedSearchQuery.toLowerCase()),
      ),
    [objects, debouncedSearchQuery],
  );

  const snapshotReqRef = useRef(0);

  // Load snapshot counts for visible objects
  useEffect(() => {
    const ids = visibleObjects.map((o) => o.id);
    if (ids.length === 0) return;
    const reqId = ++snapshotReqRef.current;
    let mounted = true;
    invoke<Record<string, number>>('snapshot_count_batch', { objectIds: ids })
      .then((counts) => {
        if (!mounted || snapshotReqRef.current !== reqId) return; // stale response, discard
        // Ensure every visible object has a snapshot count (default 0)
        const full: Record<string, number> = {};
        for (const id of ids) full[id] = counts[id] ?? 0;
        setSnapshotCounts(full);
      })
      .catch((err) => {
        if (!mounted || snapshotReqRef.current !== reqId) return; // stale error, discard
        logger.warn('[Workspace] Snapshot count batch failed:', err);
      });
    return () => {
      mounted = false;
    };
  }, [visibleObjects]);

  // Load attachment counts for visible objects
  const refreshAttachmentCounts = useCallback(() => {
    abortRef.current?.abort();
    const controller = new AbortController();
    abortRef.current = controller;
    const ids = visibleObjects.map((o) => o.id);
    if (ids.length === 0) {
      return () => controller.abort();
    }
    invoke<Record<string, number>>('attachment_count_batch', { objectIds: ids })
      .then((counts) => {
        if (!controller.signal.aborted) setAttachmentCounts(counts);
      })
      .catch((err) => logger.warn('[Workspace] Attachment count batch failed:', err));
    return () => controller.abort();
  }, [visibleObjects]);

  useEffect(() => {
    return refreshAttachmentCounts();
  }, [refreshAttachmentCounts]);

  const newObjectUrl = pageId
    ? `/editor?parentId=${pageId}`
    : `/editor${sectionFilter ? `?section=${sectionFilter}` : ''}`;

  const handleDelete = async (objectId: string) => {
    setConfirmDelete(null);
    setDeletingId(objectId);
    try {
      await deleteObject(objectId);
    } finally {
      setDeletingId(null);
    }
  };

  // 模板同步流程域（useWorkspaceTemplateSync 收敛：syncDialog/dismissConfirm/
  // 指纹映射/语义复核 + 五个同步 handler）
  const templateSync = useWorkspaceTemplateSync({
    accountId,
    pageId,
    sectionFilter,
    detailObj,
    setDetailObj,
    userTemplates,
    loadObjects,
    previewSyncTemplate,
    applySyncTemplate,
    ignoreTemplateSync,
  });

  const handleViewDeprecatedFields = useCallback(
    async (objectId: string, objectName: string) => {
      if (!accountId) return;
      setDeprecatedViewer({ objectId, objectName });
      try {
        const fields = await loadDeprecatedFields(accountId, objectId);
        setDeprecatedFields(fields);
      } catch (err) {
        logger.warn('[Workspace] Load deprecated fields failed:', err);
        setDeprecatedFields([]);
      }
    },
    [accountId, loadDeprecatedFields],
  );

  return {
    t,
    OBJECT_PAGE_SIZE,
    accountId,
    objects,
    isLoading,
    error,
    searchQuery,
    setSearchQuery,
    visibleObjects,
    visibleLimit,
    setVisibleLimit,
    snapshotCounts,
    attachmentCounts,
    customPage,
    activeCategoryLabel,
    resolveCollectionLabel,
    newObjectUrl,
    userTemplates,
    customPages,
    activeCustomPages,
    removeCustomPage,
    confirmDelete,
    setConfirmDelete,
    confirmPageDelete,
    setConfirmPageDelete,
    historyObj,
    setHistoryObj,
    attachmentObjId,
    setAttachmentObjId,
    detailObj,
    setDetailObj,
    deprecatedViewer,
    setDeprecatedViewer,
    deprecatedFields,
    setDeprecatedFields,
    showPwDialog,
    setShowPwDialog,
    pwResolveRef,
    bioAvailable,
    passwordHint,
    passwordVerify,
    verifyVaultPassword,
    handleBiometricUnlock,
    getFieldSensitivity,
    isFieldDeprecated,
    getFieldName,
    refreshAttachmentCounts,
    handleDelete,
    handleViewDeprecatedFields,
    // 模板同步流程域（useWorkspaceTemplateSync 展开）
    ...templateSync,
  };
}
