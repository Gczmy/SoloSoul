import { useState, useEffect, useRef, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { useToastError } from '@/hooks/useToastError';
import { resolveBackendErrorMessage } from '@/lib/backendError';
import { logger } from '@/lib/logger';
import { invokeCommand as invoke } from '@/lib/ipcClient';

import { useAuthStore } from '@/stores/authStore';
import { useExportEstimate } from '@/hooks/useExportEstimate';
import { buildExportImportGuidePages } from './exportImportGuidePages';
import { useExportExecution } from './useExportExecution';
import { useExportScope } from '@/hooks/useExportScope';
import { useImportState } from '@/hooks/useImportState';
import { prefetchRegistry } from '@/lib/prefetch/registry';
import { usePrefetchData } from '@/lib/prefetch/usePrefetchData';
import type { ExportImportTabKey } from '@/components/settings/ExportImportTabBar';
import type { CloudTargetInfo } from '@/types/exportImport';

type TabKey = ExportImportTabKey;

/**
 * 导出/导入页的全部编排逻辑（P046 拆分：数据 hook）。
 * Tab 状态、导出范围/密码/路径/估算、导入流程（委托 useImportState）、
 * 导出执行（含 Android content:// URI 中转）均收敛于此；
 * ExportImportPage 组件退化为纯展示组合层。
 */
export function useExportImportPage() {
  const navigate = useNavigate();
  const { onError, onSuccess } = useToastError();
  const { t, i18n } = useTranslation(['settings', 'common']);
  const accountId = useAuthStore((s) => s.currentAccount?.id ?? '');

  const [tab, setTab] = useState<TabKey>('export');

  // Phase 1 云打包：检测到的云盘同步目录（桌面端；移动端恒为空，走 SAF 选择器）。
  const [cloudTargets, setCloudTargets] = useState<CloudTargetInfo[]>([]);
  useEffect(() => {
    let cancelled = false;
    invoke<CloudTargetInfo[]>('cloud_targets_detect')
      .then((targets) => {
        if (!cancelled) setCloudTargets(targets ?? []);
      })
      .catch((err) => {
        logger.warn('[export] cloud_targets_detect failed:', err);
        if (!cancelled) setCloudTargets([]);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const exportImportGuidePages = useMemo(
    () => buildExportImportGuidePages(t),
    [t],
  );

  // Export state
  const [includeAttachments, setIncludeAttachments] = useState(false);
  const {
    selectedPageIds,
    selectedObjectIds,
    expandedPages,
    selectedAttachmentIds,
    objectAttachments,
    expandedObjects,
    togglePage,
    toggleObject,
    toggleObjectExpanded,
    toggleAttachment,
    toggleExpandedPage,
    totalSelected,
    loadSelectedAttachments,
    bulkSelect,
  } = useExportScope({ accountId, includeAttachments });
  const [selectedTags, setSelectedTags] = useState<Set<string>>(new Set());
  const [includePreferences, setIncludePreferences] = useState(false);
  const [includeBehavioral, setIncludeBehavioral] = useState(false);

  // Load scope tree — Prefetch Runtime：登录后后台预热，进入页面直接渲染；
  // 冷启动兜底现场加载；导入成功后 reloadScope() 强制刷新缓存。
  const {
    data: scopeTree,
    error: scopeStoreError,
    reload: reloadScope,
  } = usePrefetchData(prefetchRegistry.exportScope, { enabled: !!accountId });
  // 引用稳定：scopeTree 不变时复用同一数组，避免 hasSensitiveData/allTags 的 useMemo 每帧重算。
  const pageGroups = useMemo(() => scopeTree ?? [], [scopeTree]);
  // N-11: 加载失败态与「无数据」区分——失败时渲染错误占位 + 重试，不渲染空导出树。
  const scopeLoaded = scopeTree !== null || scopeStoreError !== null;
  const scopeError = scopeStoreError ? resolveBackendErrorMessage(scopeStoreError) : null;
  // 加载失败经 store.error 补 toast（原 loadScope 行为保持）。
  useEffect(() => {
    if (scopeStoreError) {
      // P120: 失败不得静默——用户看到空导出范围会误以为数据丢失。
      logger.warn('[ExportImportPage] Load export scope tree failed:', scopeStoreError);
      onError(
        new Error(scopeStoreError),
        t('settings:export_scope_load_failed', { defaultValue: '导出范围加载失败，请重试' }),
      );
    }
  }, [scopeStoreError, onError, t]);

  // Import state — 全部迁移至 useImportState hook（P013/3）
  const {
    importPath,
    importPreview,
    importPw,
    decryptedPreview,
    isPreviewing,
    isDecrypting,
    isImporting,
    importStrategy,
    importSelections,
    showStrategySelector,
    importSelectedPageIds,
    importSelectedAttachmentIds,
    importExpandedPages,
    importExpandedObjects,
    objectConflictStrategies,
    importTotalSelected,
    setImportPreview,
    setDecryptedPreview,
    setImportPw,
    setShowStrategySelector,
    setImportStrategy,
    onPreview: handlePreviewImport,
    onDecrypt: handleDecryptPreview,
    onImport: handleImport,
    onSetImportPath: handleSetImportPath,
    onToggleSelection: toggleImportSelection,
    onToggleImportPage: toggleImportPage,
    onToggleImportAttachment: toggleImportAttachment,
    onToggleExpandedImportPage: toggleExpandedImportPage,
    onToggleImportObjectExpanded: toggleImportObjectExpanded,
    onSelectAllImport: handleSelectAllImport,
    onSetObjectConflictStrategy: handleSetObjectConflictStrategy,
  } = useImportState({
    accountId,
    onError,
    onSuccess,
    t,
    i18n,
    reloadScope,
  });

  // P034: 组件卸载时清空导入密码 state（导出密码由 useExportExecution 自清理）
  useEffect(() => {
    return () => {
      setImportPw('');
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps -- setState 引用稳定，仅需挂载时注册一次
  }, []);

  // Page / object / attachment toggles — managed via useExportScope

  // 当 includeAttachments 从 false 切为 true 时，为已选对象批量加载附件（Bug 修复）
  const prevIncludeAttachmentsRef = useRef(includeAttachments);
  useEffect(() => {
    if (includeAttachments && !prevIncludeAttachmentsRef.current && totalSelected > 0) {
      loadSelectedAttachments();
    }
    prevIncludeAttachmentsRef.current = includeAttachments;
  }, [includeAttachments, totalSelected, loadSelectedAttachments]);

  // Export estimate via dedicated hook (P062 fix)
  const scopeState = useMemo(
    () => ({
      selectedPageIds,
      selectedObjectIds,
      selectedTags,
      includeAttachments,
      selectedAttachmentIds,
      includePreferences,
      includeBehavioral,
    }),
    [
      selectedPageIds,
      selectedObjectIds,
      selectedTags,
      includeAttachments,
      selectedAttachmentIds,
      includePreferences,
      includeBehavioral,
    ],
  );

  // P009：导出执行（密码/警告/Android URI 中转）拆至独立 hook
  const {
    exportPassword,
    setExportPassword,
    exportPasswordConfirm,
    setExportPasswordConfirm,
    exportHint,
    setExportHint,
    savePath,
    setSavePath,
    isExporting,
    showHintWarning,
    setShowHintWarning,
    showWeakPasswordWarning,
    setShowWeakPasswordWarning,
    skipHintCheckRef,
    skipWeakPasswordCheckRef,
    handleExport,
  } = useExportExecution({
    accountId,
    cloudTargets,
    scope: scopeState,
    totalSelected,
  });

  const { estimate: exportEstimate, estimating } = useExportEstimate(
    accountId,
    scopeState,
    totalSelected,
  );

  // Has sensitive data?
  const hasSensitiveData = useMemo(() => {
    for (const group of pageGroups) {
      for (const obj of group.objects) {
        if (
          selectedObjectIds.has(obj.id) &&
          (obj.sensitivityLevel === 'sensitive' || obj.sensitivityLevel === 'critical')
        ) {
          return true;
        }
      }
    }
    return false;
  }, [pageGroups, selectedObjectIds]);

  // All unique tags from selected objects
  const allTags = useMemo(() => {
    const tagSet = new Set<string>();
    for (const group of pageGroups) {
      for (const obj of group.objects) {
        if (selectedObjectIds.has(obj.id) && obj.tags) {
          for (const tag of obj.tags) {
            if (tag) tagSet.add(tag);
          }
        }
      }
    }
    return Array.from(tagSet);
  }, [pageGroups, selectedObjectIds]);

  return {
    t,
    navigate,
    accountId,
    exportImportGuidePages,
    // tab
    tab,
    setTab,
    // scope
    scopeLoaded,
    scopeError,
    reloadScope,
    pageGroups,
    // export selection (useExportScope)
    selectedPageIds,
    selectedObjectIds,
    expandedPages,
    selectedAttachmentIds,
    objectAttachments,
    expandedObjects,
    totalSelected,
    togglePage,
    toggleObject,
    toggleObjectExpanded,
    toggleAttachment,
    toggleExpandedPage,
    bulkSelect,
    // export form state
    exportPassword,
    setExportPassword,
    exportPasswordConfirm,
    setExportPasswordConfirm,
    exportHint,
    setExportHint,
    savePath,
    setSavePath,
    cloudTargets,
    isExporting,
    showHintWarning,
    setShowHintWarning,
    showWeakPasswordWarning,
    setShowWeakPasswordWarning,
    selectedTags,
    setSelectedTags,
    includeAttachments,
    setIncludeAttachments,
    includePreferences,
    setIncludePreferences,
    includeBehavioral,
    setIncludeBehavioral,
    exportEstimate,
    estimating,
    hasSensitiveData,
    allTags,
    handleExport,
    // import (useImportState)
    importPath,
    importPreview,
    importPw,
    decryptedPreview,
    isPreviewing,
    isDecrypting,
    isImporting,
    importStrategy,
    importSelections,
    showStrategySelector,
    importSelectedPageIds,
    importSelectedAttachmentIds,
    importExpandedPages,
    importExpandedObjects,
    objectConflictStrategies,
    importTotalSelected,
    handleSetImportPath,
    setImportPreview,
    setDecryptedPreview,
    setImportPw,
    setShowStrategySelector,
    setImportStrategy,
    handlePreviewImport,
    handleDecryptPreview,
    handleImport,
    toggleImportSelection,
    toggleImportPage,
    toggleImportAttachment,
    toggleExpandedImportPage,
    toggleImportObjectExpanded,
    handleSelectAllImport,
    handleSetObjectConflictStrategy,
    // warning-skip refs（onSetShowHintWarningAndExport / onSetWeakPasswordExport 闭包用）
    skipHintCheckRef,
    skipWeakPasswordCheckRef,
  };
}
