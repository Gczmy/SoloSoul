import { useState, useEffect, useCallback, useRef } from 'react';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { invokeTypedCommand } from '@/lib/typedIpc';
import { toObjectDataView } from '@/lib/objectViewModel';
import {
  objectNeedsSync,
  resolveSemanticNeedsSync,
  type TemplateSyncResult,
} from '@/lib/templateSync';
import { logger } from '@/lib/logger';
import { backendErrorLogDetails } from '@/lib/backendErrorWire';
import type { ObjectData, ObjectSummary } from '@/stores/objectStore';
import type { UserTemplate } from '@/types/template';

export interface UseWorkspaceTemplateSyncOptions {
  accountId?: string;
  pageId?: string;
  sectionFilter: string;
  /** 当前打开的详情对象（hash 初判依赖；刷新由父层 setDetailObj 执行）。 */
  detailObj: (ObjectSummary | ObjectData) | null;
  setDetailObj: (obj: (ObjectSummary | ObjectData) | null) => void;
  /** 模板列表（指纹映射仅在模板变化时重算）。 */
  userTemplates: UserTemplate[];
  loadObjects: (
    accountId: string,
    opts?: { parentId?: string; typeId?: string },
  ) => Promise<unknown>;
  previewSyncTemplate: (accountId: string, objectId: string) => Promise<TemplateSyncResult>;
  applySyncTemplate: (accountId: string, objectId: string) => Promise<unknown>;
  ignoreTemplateSync: (objectId: string, latestHash: string) => Promise<unknown>;
}

/**
 * 对象工作区的模板同步流程域（W005 拆分：数据 hook）。
 * 同步/忽略确认弹窗状态、模板指纹映射（hash 初判 + 语义复核）、同步后刷新
 * 与五个同步 handler 收敛于此；父 hook 仅注入 store actions 并展开返回值。
 */
export function useWorkspaceTemplateSync({
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
}: UseWorkspaceTemplateSyncOptions) {
  // 模板指纹映射：仅在模板列表变化时异步计算一次，避免切换页面时批量重算导致闪烁。
  const [templateHashMap, setTemplateHashMap] = useState<Map<string, string>>(new Map());
  const templateHashVersion = useRef(0);

  // 模板同步确认弹窗状态
  const [syncDialog, setSyncDialog] = useState<{
    objectId: string;
    objectName: string;
    result: TemplateSyncResult | null;
    loading: boolean;
  } | null>(null);
  const syncPreviewVersion = useRef(0);

  // 忽略模板更新二次确认弹窗状态
  const [dismissConfirm, setDismissConfirm] = useState<{
    objectId: string;
    objectName: string;
    latestHash: string;
  } | null>(null);

  // 同步成功后刷新当前打开的详情对象，避免本地 state 保留旧的 templateHash 导致提示条继续显示。
  const refreshDetailObjAfterSync = useCallback(
    async (objectId: string) => {
      if (!accountId || detailObj?.id !== objectId) return;
      try {
        const obj = await invokeTypedCommand('object_get', {
          accountId: accountId,
          objectId: objectId,
        });
        if (obj) setDetailObj(toObjectDataView(obj));
      } catch (err) {
        logger.warn(
          '[Workspace] Refresh detail object after sync failed:',
          backendErrorLogDetails(err),
        );
      }
    },
    [accountId, detailObj?.id, setDetailObj],
  );

  // 模板同步确认弹窗打开期间，对应对象的提示条应临时隐藏，避免被弹窗遮罩盖住。
  const [syncDialogOpenForObjectId, setSyncDialogOpenForObjectId] = useState<string | null>(null);
  useEffect(() => {
    setSyncDialog(null);
    setSyncDialogOpenForObjectId(null);
    setDismissConfirm(null);
    return () => {
      syncPreviewVersion.current += 1;
    };
  }, [accountId]);

  const handleCancelSync = useCallback(() => {
    syncPreviewVersion.current += 1;
    setSyncDialog(null);
    setSyncDialogOpenForObjectId(null);
  }, []);

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

  // 仅在模板列表变化时计算指纹映射，页面切换时复用，避免同步提示条闪烁。
  useEffect(() => {
    const version = ++templateHashVersion.current;
    if (!accountId || userTemplates.length === 0) {
      setTemplateHashMap(new Map());
      return;
    }
    let cancelled = false;
    invoke<Record<string, string>>('template_hash_map', { accountId: accountId })
      .then((map) => {
        if (cancelled || version !== templateHashVersion.current) return;
        setTemplateHashMap(new Map(Object.entries(map)));
      })
      .catch((err) => {
        if (cancelled || version !== templateHashVersion.current) return;
        logger.warn('[Workspace] Load template hash map failed:', err);
        setTemplateHashMap(new Map());
      });
    return () => {
      cancelled = true;
      templateHashVersion.current += 1;
    };
  }, [accountId, userTemplates]);

  // 同步/忽略模板后主动刷新指纹映射，防止模板列表 state 未变化导致提示条继续显示。
  const refreshTemplateHashMap = useCallback(async () => {
    if (!accountId) return;
    const version = ++templateHashVersion.current;
    try {
      const map = await invoke<Record<string, string>>('template_hash_map', {
        accountId: accountId,
      });
      if (version !== templateHashVersion.current) return;
      setTemplateHashMap(new Map(Object.entries(map)));
    } catch (err) {
      if (version === templateHashVersion.current) {
        logger.warn('[Workspace] Refresh template hash map failed:', err);
      }
    }
  }, [accountId]);

  const handleStartSync = useCallback(
    async (objectId: string, objectName: string) => {
      if (!accountId) return;
      const version = ++syncPreviewVersion.current;
      setSyncDialogOpenForObjectId(objectId);
      setSyncDialog({ objectId, objectName, result: null, loading: true });
      try {
        const result = await previewSyncTemplate(accountId, objectId);
        if (version !== syncPreviewVersion.current) return;
        if (!result.hasChanges) {
          // 无实际字段差异时直接应用同步（仅刷新 template_hash），避免提示条反复出现。
          setSyncDialog(null);
          setSyncDialogOpenForObjectId(null);
          await applySyncTemplate(accountId, objectId);
          if (version !== syncPreviewVersion.current) return;
          if (pageId) {
            await loadObjects(accountId, { parentId: pageId });
          } else {
            await loadObjects(accountId, sectionFilter ? { typeId: sectionFilter } : undefined);
          }
          if (version !== syncPreviewVersion.current) return;
          await refreshDetailObjAfterSync(objectId);
          if (version !== syncPreviewVersion.current) return;
          await refreshTemplateHashMap();
          return;
        }
        setSyncDialog((prev) =>
          prev?.objectId === objectId ? { ...prev, result, loading: false } : prev,
        );
      } catch (err) {
        if (version !== syncPreviewVersion.current) return;
        logger.warn('[Workspace] Preview sync failed:', backendErrorLogDetails(err));
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
    const { objectId } = syncDialog;
    const version = syncPreviewVersion.current;
    setSyncDialog((prev) => (prev?.objectId === objectId ? { ...prev, loading: true } : prev));
    try {
      await applySyncTemplate(accountId, objectId);
      if (version !== syncPreviewVersion.current) return;
      setSyncDialog(null);
      setSyncDialogOpenForObjectId(null);
      // 同步成功后对象 fingerprint 已更新；刷新对象列表。
      if (pageId) {
        await loadObjects(accountId, { parentId: pageId });
      } else {
        await loadObjects(accountId, sectionFilter ? { typeId: sectionFilter } : undefined);
      }
      if (version !== syncPreviewVersion.current) return;
      await refreshDetailObjAfterSync(objectId);
      if (version !== syncPreviewVersion.current) return;
      await refreshTemplateHashMap();
    } catch (err) {
      if (version !== syncPreviewVersion.current) return;
      logger.warn('[Workspace] Apply sync failed:', backendErrorLogDetails(err));
      setSyncDialog((prev) => (prev?.objectId === objectId ? { ...prev, loading: false } : prev));
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
            await loadObjects(accountId, sectionFilter ? { typeId: sectionFilter } : undefined);
          }
          await refreshTemplateHashMap();
        }
      } catch (err) {
        logger.warn('[Workspace] Ignore template sync failed:', backendErrorLogDetails(err));
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

  return {
    templateHashMap,
    syncDialogOpenForObjectId,
    syncDialog,
    setSyncDialogOpenForObjectId,
    dismissConfirm,
    setDismissConfirm,
    detailHashNeedsSync,
    detailSemanticNeedsSync,
    handleStartSync,
    handleCancelSync,
    handleConfirmSync,
    handleRequestDismissSync,
    handleConfirmDismissSync,
  };
}
