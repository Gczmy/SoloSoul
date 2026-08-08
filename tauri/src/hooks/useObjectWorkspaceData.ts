import { useState, useEffect, useCallback, useRef, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { useObjectStore, type ObjectSummary, type ObjectData } from '@/stores/objectStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { useTemplateStore } from '@/stores/templateStore';
import { useAuthStore } from '@/stores/authStore';
import {
  objectNeedsSync,
  resolveSemanticNeedsSync,
  type TemplateSyncResult,
  type DeprecatedField,
} from '@/lib/templateSync';
import { DEBOUNCE_DELAY_MS } from '@/lib/constants';
import { logger } from '@/lib/logger';
import { useWorkspacePasswordGuard } from './useWorkspacePasswordGuard';
import { useTemplateFieldMeta } from './useTemplateFieldMeta';

// P011: 对象卡片列表分页大小。注意：分页仅优化 DOM 挂载，
// snapshot/attachment 计数 IPC 仍对全量 visibleObjects 批量请求（单次 batch，成本可控）。
const OBJECT_PAGE_SIZE = 50;

export interface UseObjectWorkspaceDataOptions {
  pageId?: string;
  sectionFilter: string;
  detailObjectId: string | null;
}

/** ObjectWorkspacePage 的完整数据层与业务逻辑。 */
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

  // 模板指纹映射：仅在模板列表变化时异步计算一次，避免切换页面时批量重算导致闪烁。
  const [templateHashMap, setTemplateHashMap] = useState<Map<string, string>>(new Map());

  // 模板同步确认弹窗状态
  const [syncDialog, setSyncDialog] = useState<{
    objectId: string;
    objectName: string;
    result: TemplateSyncResult | null;
    loading: boolean;
  } | null>(null);

  // 忽略模板更新二次确认弹窗状态
  const [dismissConfirm, setDismissConfirm] = useState<{
    objectId: string;
    objectName: string;
    latestHash: string;
  } | null>(null);

  // 历史字段查看器状态
  const [deprecatedViewer, setDeprecatedViewer] = useState<{
    objectId: string;
    objectName: string;
  } | null>(null);
  const [deprecatedFields, setDeprecatedFields] = useState<DeprecatedField[]>([]);

  // 同步成功后刷新当前打开的详情对象，避免本地 state 保留旧的 templateHash 导致提示条继续显示。
  const refreshDetailObjAfterSync = useCallback(
    async (objectId: string) => {
      if (!accountId || detailObj?.id !== objectId) return;
      try {
        const obj = await invoke<ObjectData | null>('object_get', {
          accountId: accountId,
          objectId: objectId,
        });
        if (obj) setDetailObj(obj);
      } catch (err) {
        logger.warn('[Workspace] Refresh detail object after sync failed:', err);
      }
    },
    [accountId, detailObj?.id],
  );

  // 模板同步确认弹窗打开期间，对应对象的提示条应临时隐藏，避免被弹窗遮罩盖住。
  const [syncDialogOpenForObjectId, setSyncDialogOpenForObjectId] = useState<string | null>(null);

  // 详情面板模板同步：hash 初判 + 语义复核
  const detailHashNeedsSync =
    !!detailObj &&
    !!templateHashMap &&
    syncDialogOpenForObjectId !== detailObj.id &&
    objectNeedsSync(detailObj, templateHashMap);
  const [detailSemanticNeedsSync, setDetailSemanticNeedsSync] = useState(false);
  useEffect(() => {
    if (!detailHashNeedsSync || !detailObj || !accountId) {
      setDetailSemanticNeedsSync(false);
      return;
    }
    let cancelled = false;
    resolveSemanticNeedsSync(accountId, detailObj.id).then((needed) => {
      if (!cancelled) setDetailSemanticNeedsSync(needed);
    });
    return () => {
      cancelled = true;
    };
  }, [detailHashNeedsSync, detailObj, accountId]);

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

  // 仅在模板列表变化时计算指纹映射，页面切换时复用，避免同步提示条闪烁。
  useEffect(() => {
    if (!accountId || userTemplates.length === 0) {
      setTemplateHashMap(new Map());
      return;
    }
    let cancelled = false;
    invoke<Record<string, string>>('template_hash_map', { accountId: accountId })
      .then((map) => {
        if (cancelled) return;
        setTemplateHashMap(new Map(Object.entries(map)));
      })
      .catch((err) => {
        logger.warn('[Workspace] Load template hash map failed:', err);
        if (!cancelled) setTemplateHashMap(new Map());
      });
    return () => {
      cancelled = true;
    };
  }, [accountId, userTemplates]);

  // 同步/忽略模板后主动刷新指纹映射，防止模板列表 state 未变化导致提示条继续显示。
  const refreshTemplateHashMap = useCallback(async () => {
    if (!accountId) return;
    try {
      const map = await invoke<Record<string, string>>('template_hash_map', { accountId: accountId });
      setTemplateHashMap(new Map(Object.entries(map)));
    } catch (err) {
      logger.warn('[Workspace] Refresh template hash map failed:', err);
    }
  }, [accountId]);

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

  const handleStartSync = useCallback(
    async (objectId: string, objectName: string) => {
      if (!accountId) return;
      setSyncDialogOpenForObjectId(objectId);
      setSyncDialog({ objectId, objectName, result: null, loading: true });
      try {
        const result = await previewSyncTemplate(accountId, objectId);
        if (!result.hasChanges) {
          // 无实际字段差异时直接应用同步（仅刷新 template_hash），避免提示条反复出现。
          setSyncDialog(null);
          setSyncDialogOpenForObjectId(null);
          await applySyncTemplate(accountId, objectId);
          if (pageId) {
            await loadObjects(accountId, { parentId: pageId });
          } else {
            await loadObjects(
              accountId,
              sectionFilter ? { typeId: sectionFilter } : undefined,
            );
          }
          await refreshDetailObjAfterSync(objectId);
          await refreshTemplateHashMap();
          return;
        }
        setSyncDialog((prev) => (prev ? { ...prev, result, loading: false } : null));
      } catch (err) {
        logger.warn('[Workspace] Preview sync failed:', err);
        setSyncDialog(null);
        setSyncDialogOpenForObjectId(null);
      }
    },
    [
      accountId,
      previewSyncTemplate,
      applySyncTemplate,
      loadObjects,
      pageId,
      sectionFilter,
      refreshDetailObjAfterSync,
      refreshTemplateHashMap,
    ],
  );

  const handleConfirmSync = useCallback(async () => {
    if (!syncDialog || !accountId) return;
    setSyncDialog((prev) => (prev ? { ...prev, loading: true } : null));
    try {
      await applySyncTemplate(accountId, syncDialog.objectId);
      setSyncDialog(null);
      setSyncDialogOpenForObjectId(null);
      // 同步成功后对象 fingerprint 已更新；刷新对象列表。
      if (pageId) {
        await loadObjects(accountId, { parentId: pageId });
      } else {
        await loadObjects(accountId, sectionFilter ? { typeId: sectionFilter } : undefined);
      }
      await refreshDetailObjAfterSync(syncDialog.objectId);
      await refreshTemplateHashMap();
    } catch (err) {
      logger.warn('[Workspace] Apply sync failed:', err);
      setSyncDialog((prev) => (prev ? { ...prev, loading: false } : null));
    }
  }, [
    syncDialog,
    accountId,
    applySyncTemplate,
    loadObjects,
    pageId,
    sectionFilter,
    refreshDetailObjAfterSync,
    refreshTemplateHashMap,
  ]);

  const handleDismissSync = useCallback(
    async (objectId: string, latestHash?: string) => {
      if (!latestHash) return;
      try {
        await ignoreTemplateSync(objectId, latestHash);
        // 后端已持久化 ignoredTemplateHash；刷新列表与指纹映射使提示条立即消失。
        if (accountId) {
          if (pageId) {
            await loadObjects(accountId, { parentId: pageId });
          } else {
            await loadObjects(
              accountId,
              sectionFilter ? { typeId: sectionFilter } : undefined,
            );
          }
          await refreshTemplateHashMap();
        }
      } catch (err) {
        logger.warn('[Workspace] Ignore template sync failed:', err);
      }
    },
    [ignoreTemplateSync, loadObjects, accountId, pageId, sectionFilter, refreshTemplateHashMap],
  );

  const handleRequestDismissSync = useCallback(
    (objectId: string, objectName: string, latestHash?: string) => {
      if (!latestHash) return;
      setSyncDialogOpenForObjectId(objectId);
      setDismissConfirm({ objectId, objectName, latestHash });
    },
    [],
  );

  const handleConfirmDismissSync = useCallback(() => {
    if (!dismissConfirm) return;
    handleDismissSync(dismissConfirm.objectId, dismissConfirm.latestHash);
    setDismissConfirm(null);
    setSyncDialogOpenForObjectId(null);
  }, [dismissConfirm, handleDismissSync]);

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
    templateHashMap,
    syncDialogOpenForObjectId,
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
    detailHashNeedsSync,
    detailSemanticNeedsSync,
    syncDialog,
    setSyncDialog,
    setSyncDialogOpenForObjectId,
    dismissConfirm,
    setDismissConfirm,
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
    handleStartSync,
    handleConfirmSync,
    handleConfirmDismissSync,
    handleRequestDismissSync,
    handleViewDeprecatedFields,
  };
}
