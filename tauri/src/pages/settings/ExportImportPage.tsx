import { motion } from 'framer-motion';
import { useState, useEffect, useCallback, useRef, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { AppShell } from '@/components/layout/AppShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { useToastError } from '@/hooks/useToastError';
import { resolveBackendErrorMessage } from '@/lib/backendError';
import { logger } from '@/lib/logger';
import { PageGuideButton } from '@/components/guide/PageGuideButton';
import {
  cleanupStagedFile,
  copyStagedFileToDest,
  isUriPath,
  prepareStagedDownloadPath,
} from '@/lib/mobileFileTransfer';
import { invokeCommand as invoke } from '@/lib/ipcClient';

import { useAuthStore } from '@/stores/authStore';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { ICON_SIZE } from '@/lib/constants';
import { ExportSection } from '@/components/export/ExportSection';
import { ImportSection } from '@/components/import/ImportSection';
import { ExportImportTabBar } from '@/components/settings/ExportImportTabBar';
import { useExportEstimate } from '@/hooks/useExportEstimate';
import { useExportScope } from '@/hooks/useExportScope';
import { useImportState } from '@/hooks/useImportState';
import { Info, FolderOpen, Lock, GitCompare } from 'lucide-react';
import type { PageGroup } from '@/types/exportImport';

type TabKey = 'export' | 'import';

export function ExportImportPage() {
  const navigate = useNavigate();
  const { onError, onSuccess } = useToastError();
  const { t, i18n } = useTranslation(['settings', 'common']);
  const accountId = useAuthStore((s) => s.currentAccount?.id ?? '');

  const [tab, setTab] = useState<TabKey>('export');

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
  const [pageGroups, setPageGroups] = useState<PageGroup[]>([]);
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

  // Load scope tree
  const [scopeLoaded, setScopeLoaded] = useState(false);
  // N-11: 加载失败态与「无数据」区分——失败时渲染错误占位 + 重试，不渲染空导出树。
  const [scopeError, setScopeError] = useState<string | null>(null);
  const loadScope = useCallback(() => {
    if (!accountId) return;
    setScopeLoaded(false);
    setScopeError(null);
    invoke<PageGroup[]>('export_get_scope_tree', { accountId: accountId })
      .then((groups) => {
        setPageGroups(groups);
        setScopeLoaded(true);
      })
      .catch((err) => {
        // P120: 失败不得静默——用户看到空导出范围会误以为数据丢失。
        logger.warn('[ExportImportPage] Load export scope tree failed:', err);
        const message = resolveBackendErrorMessage(err);
        onError(
          new Error(message),
          t('settings:export_scope_load_failed', { defaultValue: '导出范围加载失败，请重试' }),
        );
        setPageGroups([]);
        setScopeError(message);
        setScopeLoaded(true);
      });
  }, [accountId, onError, t]);

  useEffect(() => {
    loadScope();
  }, [loadScope]);

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
    reloadScope: loadScope,
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

  return (
    <AppShell
      title={t('settings:export_import')}
      onBack={() => navigate('/settings')}
      actions={<PageGuideButton pages={exportImportGuidePages} />}
    >
      <PageContainer variant="medium" gap="default">

        <ExportImportTabBar tab={tab} onChange={setTab} />

        {tab === 'export' && scopeLoaded && scopeError ? (
          // N-11: 加载失败态与「空数据」同态问题——失败时显示错误占位 + 重试，
          // 不再渲染空导出树（用户误以为数据丢失）。
          <Card style={{ padding: '48px 24px', textAlign: 'center' }}>
            <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 12 }}>
              <Info size={ICON_SIZE['2xl']} style={{ opacity: 0.4, color: 'var(--text-tertiary)' }} />
              <p style={{ fontSize: 'var(--text-body)', color: 'var(--text-secondary)' }}>
                {t('settings:export_scope_load_failed', { defaultValue: '导出范围加载失败，请重试' })}
              </p>
              <p
                style={{
                  fontSize: 'var(--text-caption)',
                  color: 'var(--text-tertiary)',
                  maxWidth: 420,
                  wordBreak: 'break-word',
                }}
              >
                {scopeError}
              </p>
              <Button variant="primary" size="sm" onClick={loadScope}>
                {t('common:retry')}
              </Button>
            </div>
          </Card>
        ) : tab === 'export' && scopeLoaded ? (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ duration: 0.2 }}
            style={{ display: 'flex', flexDirection: 'column', gap: 'var(--page-gap)' }}
          >
            <ExportSection
              pageGroups={pageGroups}
              selectedPageIds={selectedPageIds}
              selectedObjectIds={selectedObjectIds}
              expandedPages={expandedPages}
              exportPassword={exportPassword}
              exportPasswordConfirm={exportPasswordConfirm}
              exportHint={exportHint}
              savePath={savePath}
              isExporting={isExporting}
              showHintWarning={showHintWarning}
              selectedTags={selectedTags}
              includeAttachments={includeAttachments}
              selectedAttachmentIds={selectedAttachmentIds}
              objectAttachments={objectAttachments}
              expandedObjects={expandedObjects}
              includePreferences={includePreferences}
              includeBehavioral={includeBehavioral}
              exportEstimate={exportEstimate}
              estimating={estimating}
              hasSensitiveData={hasSensitiveData}
              allTags={allTags}
              totalSelected={totalSelected}
              onTogglePage={togglePage}
              onToggleObject={toggleObject}
              onToggleObjectExpanded={toggleObjectExpanded}
              onToggleAttachment={toggleAttachment}
              onSetExportPassword={setExportPassword}
              onSetExportPasswordConfirm={setExportPasswordConfirm}
              onSetExportHint={setExportHint}
              onSetSavePath={setSavePath}
              onExport={handleExport}
              onSetShowHintWarning={setShowHintWarning}
              onSetSelectedTags={(updater) => setSelectedTags(updater)}
              onSetIncludeAttachments={setIncludeAttachments}
              onSetIncludePreferences={setIncludePreferences}
              onSetIncludeBehavioral={setIncludeBehavioral}
              onToggleExpandedPage={toggleExpandedPage}
              onSelectAllExport={(selectAll) =>
                bulkSelect(
                  selectAll,
                  pageGroups.flatMap((g) => g.objects.map((o) => o.id)),
                  pageGroups.map((g) => g.sectionType),
                )
              }
              showWeakPasswordWarning={showWeakPasswordWarning}
              onSetShowWeakPasswordWarning={setShowWeakPasswordWarning}
              onSetShowHintWarningAndExport={() => {
                skipHintCheckRef.current = true;
                setShowHintWarning(false);
                handleExport();
              }}
              onSetWeakPasswordExport={() => {
                skipWeakPasswordCheckRef.current = true;
                setShowWeakPasswordWarning(false);
                handleExport();
              }}
            />
          </motion.div>
        ) : tab === 'import' ? (
          <ImportSection
            importPath={importPath}
            importPreview={importPreview}
            importPw={importPw}
            decryptedPreview={decryptedPreview}
            isPreviewing={isPreviewing}
            isDecrypting={isDecrypting}
            isImporting={isImporting}
            importStrategy={importStrategy}
            importSelections={importSelections}
            showStrategySelector={showStrategySelector}
            importSelectedPageIds={importSelectedPageIds}
            importSelectedAttachmentIds={importSelectedAttachmentIds}
            importExpandedPages={importExpandedPages}
            importExpandedObjects={importExpandedObjects}
            importTotalSelected={importTotalSelected}
            onSetImportPath={handleSetImportPath}
            onSetImportPreview={setImportPreview}
            onSetDecryptedPreview={setDecryptedPreview}
            onSetImportPw={setImportPw}
            onSetShowStrategySelector={setShowStrategySelector}
            onPreview={handlePreviewImport}
            onDecrypt={handleDecryptPreview}
            onImport={handleImport}
            onToggleSelection={toggleImportSelection}
            onToggleImportPage={toggleImportPage}
            onToggleImportAttachment={toggleImportAttachment}
            onToggleExpandedImportPage={toggleExpandedImportPage}
            onToggleImportObjectExpanded={toggleImportObjectExpanded}
            onSelectAllImport={handleSelectAllImport}
            onSetStrategy={setImportStrategy}
            objectConflictStrategies={objectConflictStrategies}
            onSetObjectConflictStrategy={handleSetObjectConflictStrategy}
          />
        ) : null}
      </PageContainer>
    </AppShell>
  );
}
