import { useState, useEffect, useRef, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { useToastError } from '@/hooks/useToastError';
import { resolveBackendErrorMessage } from '@/lib/backendError';
import { logger } from '@/lib/logger';
import {
  cleanupStagedFile,
  copyStagedFileToDest,
  isUriPath,
  prepareStagedDownloadPath,
} from '@/lib/mobileFileTransfer';
import { invokeCommand as invoke } from '@/lib/ipcClient';

import { useAuthStore } from '@/stores/authStore';
import { Info, FolderOpen, Lock, GitCompare } from 'lucide-react';
import { useExportEstimate } from '@/hooks/useExportEstimate';
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
    () => [
      {
        icon: Info,
        title: t('common:guide_export_import_title', { defaultValue: 'Export & Import Guide' }),
        steps: [
          {
            icon: FolderOpen,
            title: t('common:guide_export_import_step1_title', { defaultValue: 'Select Scope' }),
            description:
              t('common:guide_export_import_step1_desc', { defaultValue: 'Choose the pages, objects, and tags you want to export. You can also include attachments, preferences, and behavioral data.' }),
          },
          {
            icon: Lock,
            title: t('common:guide_export_import_step2_title', { defaultValue: 'Set Password' }),
            description:
              t('common:guide_export_import_step2_desc', { defaultValue: 'Exports are encrypted with a password you provide. Keep the password safe — you will need it to import the package later.' }),
          },
          {
            icon: GitCompare,
            title: t('common:guide_export_import_step3_title', { defaultValue: 'Import & Strategy' }),
            description:
              t('common:guide_export_import_step3_desc', { defaultValue: 'When importing, preview the package and choose how to handle duplicate objects: skip existing, overwrite, or decide per object.' }),
          },
        ],
        helpLinks: [
          {
            title: t('common:guide_help_export_import', { defaultValue: 'Export & Import' }),
            description:
              t('common:guide_help_export_import_desc', { defaultValue: 'Encrypted export and import of your vault data' }),
            href: '/help?id=export_import',
          },
        ],
      },
    ],
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
  const [exportPassword, setExportPassword] = useState('');
  const [exportPasswordConfirm, setExportPasswordConfirm] = useState('');
  const [exportHint, setExportHint] = useState('');
  const [savePath, setSavePath] = useState<string | null>(null);
  const [isExporting, setIsExporting] = useState(false);
  const [showHintWarning, setShowHintWarning] = useState(false);
  const skipHintCheckRef = useRef(false);
  const [showWeakPasswordWarning, setShowWeakPasswordWarning] = useState(false);
  const skipWeakPasswordCheckRef = useRef(false);
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

  // P034: 组件卸载时清空密码 state（JS 堆不可清零，尽早缩短驻留窗口）
  useEffect(() => {
    return () => {
      setExportPassword('');
      setExportPasswordConfirm('');
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

  // Export handler
  const handleExport = async () => {
    if (totalSelected === 0 || !exportPassword || !savePath) return;

    if (exportPassword !== exportPasswordConfirm) {
      onError(new Error(t('settings:password_mismatch')), '');
      return;
    }

    // 检查 1: 密码提示词包含密码内容 → 软警告
    if (!skipHintCheckRef.current && exportHint && exportPassword.length >= 3) {
      const pwLower = exportPassword.toLowerCase();
      const hintLower = exportHint.toLowerCase();
      let hintContainsPassword = false;
      for (let i = 0; i <= pwLower.length - 3; i++) {
        if (hintLower.includes(pwLower.slice(i, i + 3))) {
          hintContainsPassword = true;
          break;
        }
      }
      if (hintContainsPassword) {
        setShowHintWarning(true);
        return;
      }
    }

    // 检查 2: 密码安全性低（不足 8 位）→ 软警告
    if (!skipWeakPasswordCheckRef.current && exportPassword.length < 8) {
      setShowWeakPasswordWarning(true);
      return;
    }

    setIsExporting(true);
    let stagedExportPath: string | null = null;
    try {
      let targetSavePath = savePath;
      // Android 保存对话框返回 content:// URI，Rust 无法直接写入，需要先写到缓存再中转
      if (savePath && isUriPath(savePath)) {
        stagedExportPath = await prepareStagedDownloadPath('solosoul_export.solosoul');
        targetSavePath = stagedExportPath;
      }

      const exportedPath = await invoke<string>('export_execute', {
        accountId: accountId,
        req: {
          scope: {
            selectedPageIds: Array.from(selectedPageIds),
            selectedObjectIds: Array.from(selectedObjectIds),
            selectedTags: Array.from(selectedTags),
            includeAttachments,
            selectedAttachmentIds: Array.from(selectedAttachmentIds),
            includePreferences,
            includeBehavioral,
          },
          password: exportPassword,
          passwordHint: exportHint || null,
          savePath: targetSavePath,
        },
      });

      if (stagedExportPath && savePath) {
        await copyStagedFileToDest(exportedPath, savePath);
      }

      // 导出成功后重置 skip ref，下次导出重新检查
      skipHintCheckRef.current = false;
      skipWeakPasswordCheckRef.current = false;
      // P034: 导出成功后立即清空密码 state（JS 堆不可清零，尽早缩短驻留窗口）
      setExportPassword('');
      setExportPasswordConfirm('');
      onSuccess(t('settings:export_success'));
    } catch (e) {
      onError(new Error(resolveBackendErrorMessage(e)), t('common:export_failed'));
    } finally {
      if (stagedExportPath) {
        await cleanupStagedFile(stagedExportPath);
      }
      setIsExporting(false);
    }
  };

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
