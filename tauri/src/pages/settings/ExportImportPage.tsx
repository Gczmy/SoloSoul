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
  const [importSelectedPageIds, setImportSelectedPageIds] = useState<Set<string>>(new Set());
  const [importSelectedAttachmentIds, setImportSelectedAttachmentIds] = useState<Set<string>>(
    new Set(),
  );
  const [importExpandedPages, setImportExpandedPages] = useState<Set<string>>(new Set());
  const [importExpandedObjects, setImportExpandedObjects] = useState<Set<string>>(new Set());

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

    // 检查 2: 密码安全性低（不足 8 位）→ 软警告
    if (!skipWeakPasswordCheckRef.current && exportPassword.length < 8) {
      setShowWeakPasswordWarning(true);
      return;
    }

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
      // 导出成功后重置 skip ref，下次导出重新检查
      skipHintCheckRef.current = false;
      skipWeakPasswordCheckRef.current = false;
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

      // 全选所有对象
      const selMap = new Map<string, boolean>();
      for (const obj of preview.objects) {
        selMap.set(obj.id, true);
      }
      setImportSelections(selMap);

      // 全选所有附件
      const attIds = new Set(preview.attachments.map((a) => a.id));
      setImportSelectedAttachmentIds(attIds);

      // 按 section_type 构建页面全选集合
      const pageIds = new Set<string>();
      for (const obj of preview.objects) {
        const st = obj.sectionType || 'uncategorized';
        pageIds.add(st);
      }
      setImportSelectedPageIds(pageIds);
    } catch (e) {
      onError(new Error(resolveBackendErrorMessage(e)), t('common:decrypt_failed'));
    } finally {
      setIsDecrypting(false);
    }
  };

  const handleImport = async () => {
    if (!importPath || !importPw || importTotalSelected === 0) return;
    setIsImporting(true);
    try {
      const selections = Array.from(importSelections.entries()).map(([objectId, selected]) => ({
        objectId,
        selected,
      }));
      const selAttIds =
        importSelectedAttachmentIds.size > 0
          ? Array.from(importSelectedAttachmentIds)
          : [];

      if (showStrategySelector && decryptedPreview) {
        const result = await invoke<ImportResult>('import_execute_advanced', {
          accountId,
          req: {
            selections,
            strategy: importStrategy,
            sourcePath: importPath,
            password: importPw,
            selectedAttachmentIds: selAttIds.length > 0 ? selAttIds : null,
          },
        });
        onSuccess(
          t('settings:import_success_with_attachments', {
            count: result.objectCount,
            attachments: result.attachmentCount,
          }),
        );
      } else {
        // Quick Import: use SkipExisting strategy, respect selections
        const result = await invoke<ImportResult>('import_execute_advanced', {
          accountId,
          req: {
            selections,
            strategy: 'skipExisting',
            sourcePath: importPath,
            password: importPw,
            selectedAttachmentIds: selAttIds.length > 0 ? selAttIds : null,
          },
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

  // ── 导入树选择处理 ──

  const toggleImportSelection = (id: string) => {
    setImportSelections((prev) => {
      const next = new Map(prev);
      const newVal = !next.get(id);
      next.set(id, newVal);
      return next;
    });
  };

  const toggleImportPage = (sectionType: string, objectIds: string[]) => {
    setImportSelectedPageIds((prev) => {
      const next = new Set(prev);
      const currentlyChecked = next.has(sectionType);
      if (currentlyChecked) {
        next.delete(sectionType);
      } else {
        next.add(sectionType);
      }
      return next;
    });
    // 同步切换该页面下所有对象的选择状态
    setImportSelections((prev) => {
      const next = new Map(prev);
      const currentlyChecked = importSelectedPageIds.has(sectionType);
      for (const id of objectIds) {
        next.set(id, !currentlyChecked);
      }
      return next;
    });
    // 同步切换该页面下所有附件
    if (decryptedPreview) {
      const pageAttIds = decryptedPreview.attachments
        .filter((a) => objectIds.includes(a.objectId))
        .map((a) => a.id);
      setImportSelectedAttachmentIds((prev) => {
        const next = new Set(prev);
        const currentlyChecked = importSelectedPageIds.has(sectionType);
        for (const attId of pageAttIds) {
          if (currentlyChecked) {
            next.delete(attId);
          } else {
            next.add(attId);
          }
        }
        return next;
      });
    }
  };

  const toggleImportAttachment = (attId: string) => {
    setImportSelectedAttachmentIds((prev) => {
      const next = new Set(prev);
      if (next.has(attId)) {
        next.delete(attId);
      } else {
        next.add(attId);
      }
      return next;
    });
  };

  const toggleExpandedImportPage = (sectionType: string) => {
    setImportExpandedPages((prev) => {
      const next = new Set(prev);
      if (next.has(sectionType)) {
        next.delete(sectionType);
      } else {
        next.add(sectionType);
      }
      return next;
    });
  };

  const toggleImportObjectExpanded = (objectId: string) => {
    setImportExpandedObjects((prev) => {
      const next = new Set(prev);
      if (next.has(objectId)) {
        next.delete(objectId);
      } else {
        next.add(objectId);
      }
      return next;
    });
  };

  // 全选/取消全选
  const handleSelectAllImport = useCallback(
    (selectAll: boolean) => {
      if (!decryptedPreview) return;
      const selMap = new Map<string, boolean>();
      for (const obj of decryptedPreview.objects) {
        selMap.set(obj.id, selectAll);
      }
      setImportSelections(selMap);

      if (selectAll) {
        const attIds = new Set(decryptedPreview.attachments.map((a) => a.id));
        setImportSelectedAttachmentIds(attIds);
        const pageIds = new Set<string>();
        for (const obj of decryptedPreview.objects) {
          pageIds.add(obj.sectionType || 'uncategorized');
        }
        setImportSelectedPageIds(pageIds);
      } else {
        setImportSelectedAttachmentIds(new Set());
        setImportSelectedPageIds(new Set());
      }
    },
    [decryptedPreview],
  );

  // 导入总选择数
  const importTotalSelected = useMemo(() => {
    let count = 0;
    for (const v of importSelections.values()) {
      if (v) count++;
    }
    return count;
  }, [importSelections]);

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
            importSelectedPageIds={importSelectedPageIds}
            importSelectedAttachmentIds={importSelectedAttachmentIds}
            importExpandedPages={importExpandedPages}
            importExpandedObjects={importExpandedObjects}
            importTotalSelected={importTotalSelected}
            onSetImportPath={setImportPath}
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
          />
        ) : null}
      </PageContainer>
    </AppShell>
  );
}
