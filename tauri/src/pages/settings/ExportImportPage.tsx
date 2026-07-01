import { motion } from 'framer-motion';
import { useState, useEffect, useCallback, useRef, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { AppShell } from '@/components/layout/AppShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { useToastError } from '@/hooks/useToastError';
import { resolveBackendErrorMessage } from '@/lib/backendError';
import { invoke } from '@tauri-apps/api/core';
import { useAuthStore } from '@/stores/authStore';
import { ExportSection } from '@/components/export/ExportSection';
import { ImportSection } from '@/components/import/ImportSection';
import { ExportImportTabBar } from '@/components/settings/ExportImportTabBar';
import { useExportEstimate } from '@/hooks/useExportEstimate';
import { useExportScope } from '@/hooks/useExportScope';
import type {
  PageGroup,
  ImportPreview,
  DecryptedImportPreview,
  ImportStrategy,
  ImportResult,
} from '@/types/exportImport';

type TabKey = 'export' | 'import';

export function ExportImportPage() {
  const navigate = useNavigate();
  const { onError, onSuccess } = useToastError();
  const { t } = useTranslation(['settings', 'common']);
  const accountId = useAuthStore((s) => s.currentAccount?.id ?? '');

  const [tab, setTab] = useState<TabKey>('export');

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

  // Import state
  const [importPath, setImportPath] = useState('');
  const [importPreview, setImportPreview] = useState<ImportPreview | null>(null);
  const [importPw, setImportPw] = useState('');
  const [decryptedPreview, setDecryptedPreview] = useState<DecryptedImportPreview | null>(null);
  const [isPreviewing, setIsPreviewing] = useState(false);
  const [isDecrypting, setIsDecrypting] = useState(false);
  const [isImporting, setIsImporting] = useState(false);
  const [importStrategy, setImportStrategy] = useState<ImportStrategy>('skipExisting');
  const [importSelections, setImportSelections] = useState<Map<string, boolean>>(new Map());
  const [showStrategySelector, setShowStrategySelector] = useState(false);

  // Load scope tree
  const [scopeLoaded, setScopeLoaded] = useState(false);
  const loadScope = useCallback(() => {
    if (!accountId) return;
    setScopeLoaded(false);
    invoke<PageGroup[]>('export_get_scope_tree', { accountId })
      .then((groups) => {
        setPageGroups(groups);
        setScopeLoaded(true);
      })
      .catch(() => {
        setScopeLoaded(true);
      });
  }, [accountId]);

  useEffect(() => {
    loadScope();
  }, [loadScope]);

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
    skipHintCheckRef.current = false;

    // 检查 2: 密码安全性低（不足 8 位）→ 软警告
    if (!skipWeakPasswordCheckRef.current && exportPassword.length < 8) {
      setShowWeakPasswordWarning(true);
      return;
    }
    skipWeakPasswordCheckRef.current = false;

    setIsExporting(true);
    try {
      await invoke<string>('export_execute', {
        accountId,
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
          savePath,
        },
      });
      onSuccess(t('settings:export_success'));
    } catch (e) {
      onError(new Error(resolveBackendErrorMessage(e)), t('common:export_failed'));
    } finally {
      setIsExporting(false);
    }
  };

  // Import handlers
  const handlePreviewImport = async () => {
    if (!importPath || isPreviewing) return;
    setIsPreviewing(true);
    try {
      const preview = await invoke<ImportPreview>('import_parse_package', {
        filePath: importPath,
      });
      setImportPreview(preview);
      setDecryptedPreview(null);
    } catch (e) {
      onError(new Error(resolveBackendErrorMessage(e)), t('common:preview_failed'));
    } finally {
      setIsPreviewing(false);
    }
  };

  const handleDecryptPreview = async () => {
    if (!importPath || !importPw || isDecrypting) return;
    setIsDecrypting(true);
    try {
      const preview = await invoke<DecryptedImportPreview>('import_decrypt_preview', {
        filePath: importPath,
        password: importPw,
      });
      setDecryptedPreview(preview);
      const selMap = new Map<string, boolean>();
      for (const obj of preview.objects) {
        selMap.set(obj.id, true);
      }
      setImportSelections(selMap);
    } catch (e) {
      onError(new Error(resolveBackendErrorMessage(e)), t('common:decrypt_failed'));
    } finally {
      setIsDecrypting(false);
    }
  };

  const handleImport = async () => {
    if (!importPath || !importPw) return;
    setIsImporting(true);
    try {
      if (showStrategySelector && decryptedPreview) {
        const selections = Array.from(importSelections.entries()).map(([objectId, selected]) => ({
          objectId,
          selected,
        }));
        const result = await invoke<ImportResult>('import_execute_advanced', {
          accountId,
          req: {
            selections,
            strategy: importStrategy,
            sourcePath: importPath,
            password: importPw,
          },
        });
        onSuccess(
          t('settings:import_success_with_attachments', {
            count: result.objectCount,
            attachments: result.attachmentCount,
          }),
        );
      } else {
        const result = await invoke<ImportResult>('import_execute', {
          accountId,
          filePath: importPath,
          password: importPw,
        });
        onSuccess(
          t('settings:import_success_with_attachments', {
            count: result.objectCount,
            attachments: result.attachmentCount,
          }),
        );
      }
      setImportPreview(null);
      setDecryptedPreview(null);
      setImportPath('');
      setImportPw('');
      setShowStrategySelector(false);
      loadScope();
    } catch (e) {
      onError(new Error(resolveBackendErrorMessage(e)), t('common:import_failed'));
    } finally {
      setIsImporting(false);
    }
  };

  const toggleImportSelection = (id: string) => {
    setImportSelections((prev) => {
      const next = new Map(prev);
      next.set(id, !next.get(id));
      return next;
    });
  };

  return (
    <AppShell title={t('settings:export_import')} onBack={() => navigate('/settings')}>
      <PageContainer variant="medium" gap="default">
        <ExportImportTabBar tab={tab} onChange={setTab} />

        {tab === 'export' && scopeLoaded ? (
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
            onSetImportPath={setImportPath}
            onSetImportPreview={setImportPreview}
            onSetDecryptedPreview={setDecryptedPreview}
            onSetImportPw={setImportPw}
            onSetShowStrategySelector={setShowStrategySelector}
            onPreview={handlePreviewImport}
            onDecrypt={handleDecryptPreview}
            onImport={handleImport}
            onToggleSelection={toggleImportSelection}
            onSetStrategy={setImportStrategy}
          />
        ) : null}
      </PageContainer>
    </AppShell>
  );
}
