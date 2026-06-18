import { useState, useEffect, useCallback, useRef, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { AppShell } from '@/components/layout/AppShell';
import { useToastError } from '@/hooks/useToastError';
import { resolveBackendErrorMessage } from '@/lib/backendError';
import { invoke } from '@tauri-apps/api/core';
import { useAuthStore } from '@/stores/authStore';
import { ExportSection } from '@/components/export/ExportSection';
import { ImportSection } from '@/components/import/ImportSection';
import { useExportEstimate } from '@/hooks/useExportEstimate';
import type {
  PageGroup,
  AttachmentInfo,
  ImportPreview,
  DecryptedImportPreview,
  ImportStrategy,
  PasswordStrength,
} from '@/types/exportImport';
import { assessPasswordStrength } from '@/types/exportImport';

type TabKey = 'export' | 'import';

export function ExportImportPage() {
  const navigate = useNavigate();
  const { onError, onSuccess } = useToastError();
  const { t } = useTranslation(['settings', 'common']);
  const accountId = useAuthStore((s) => s.currentAccount?.id ?? '');

  const [tab, setTab] = useState<TabKey>('export');

  // Export state
  const [pageGroups, setPageGroups] = useState<PageGroup[]>([]);
  const [selectedPageIds, setSelectedPageIds] = useState<Set<string>>(new Set());
  const [selectedObjectIds, setSelectedObjectIds] = useState<Set<string>>(new Set());
  const [expandedPages, setExpandedPages] = useState<Set<string>>(new Set());
  const toggleExpandedPage = (sectionType: string) => {
    setExpandedPages((prev) => {
      const next = new Set(prev);
      if (next.has(sectionType)) next.delete(sectionType);
      else next.add(sectionType);
      return next;
    });
  };
  const [exportPassword, setExportPassword] = useState('');
  const [exportPasswordConfirm, setExportPasswordConfirm] = useState('');
  const [exportHint, setExportHint] = useState('');
  const [savePath, setSavePath] = useState<string | null>(null);
  const [isExporting, setIsExporting] = useState(false);
  const [showWeakWarning, setShowWeakWarning] = useState(false);
  const [showHintWarning, setShowHintWarning] = useState(false);
  const skipHintCheckRef = useRef(false);
  const [selectedTags, setSelectedTags] = useState<Set<string>>(new Set());
  const [includeAttachments, setIncludeAttachments] = useState(false);
  const [selectedAttachmentIds, setSelectedAttachmentIds] = useState<Set<string>>(new Set());
  const [objectAttachments, setObjectAttachments] = useState<Map<string, AttachmentInfo[]>>(
    new Map(),
  );
  const [expandedObjects, setExpandedObjects] = useState<Set<string>>(new Set());
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
  const loadScope = useCallback(() => {
    if (!accountId) return;
    invoke<PageGroup[]>('export_get_scope_tree', { accountId })
      .then(setPageGroups)
      .catch(() => {});
  }, [accountId]);

  useEffect(() => {
    loadScope();
  }, [loadScope]);

  // Page / object / attachment toggles
  const togglePage = (sectionType: string, objectIds: string[]) => {
    setSelectedPageIds((prev) => {
      const next = new Set(prev);
      const isAdding = !next.has(sectionType);
      if (isAdding) next.add(sectionType);
      else next.delete(sectionType);

      setSelectedObjectIds((oPrev) => {
        const oNext = new Set(oPrev);
        for (const id of objectIds) {
          if (isAdding) oNext.add(id);
          else oNext.delete(id);
        }
        return oNext;
      });

      setSelectedAttachmentIds((attPrev) => {
        const attNext = new Set(attPrev);
        for (const id of objectIds) {
          const atts = objectAttachments.get(id) || [];
          for (const att of atts) {
            if (isAdding) attNext.add(att.id);
            else attNext.delete(att.id);
          }
        }
        return attNext;
      });

      if (isAdding && includeAttachments) {
        const unloadedIds = objectIds.filter((id) => !objectAttachments.has(id));
        if (unloadedIds.length > 0) {
          Promise.all(
            unloadedIds.map((id) =>
              invoke<AttachmentInfo[]>('export_get_attachments', { accountId, objectId: id })
                .then((atts) => ({ id, atts }))
                .catch(() => ({ id, atts: [] as AttachmentInfo[] })),
            ),
          ).then((results) => {
            setObjectAttachments((prev) => {
              const n = new Map(prev);
              for (const { id, atts } of results) n.set(id, atts);
              return n;
            });
            setSelectedAttachmentIds((prev) => {
              const n = new Set(prev);
              for (const { atts } of results) for (const att of atts) n.add(att.id);
              return n;
            });
          });
        }
      }
      return next;
    });
  };

  const toggleObject = (id: string, sectionType: string, allIdsInGroup: string[]) => {
    setSelectedObjectIds((prev) => {
      const next = new Set(prev);
      const isAdding = !next.has(id);
      if (isAdding) next.add(id);
      else next.delete(id);

      setSelectedPageIds((pPrev) => {
        const pNext = new Set(pPrev);
        const allSelectedNow = allIdsInGroup.every((oid) => next.has(oid));
        if (allSelectedNow) pNext.add(sectionType);
        else pNext.delete(sectionType);
        return pNext;
      });

      setSelectedAttachmentIds((attPrev) => {
        const attNext = new Set(attPrev);
        const atts = objectAttachments.get(id) || [];
        for (const att of atts) {
          if (isAdding) attNext.add(att.id);
          else attNext.delete(att.id);
        }
        return attNext;
      });

      if (isAdding && includeAttachments && !objectAttachments.has(id)) {
        invoke<AttachmentInfo[]>('export_get_attachments', { accountId, objectId: id })
          .then((atts) => {
            setObjectAttachments((prev) => {
              const n = new Map(prev);
              n.set(id, atts);
              return n;
            });
            setSelectedAttachmentIds((prev) => {
              const n = new Set(prev);
              for (const att of atts) n.add(att.id);
              return n;
            });
          })
          .catch(() => {});
      }
      return next;
    });
  };

  const toggleObjectExpanded = (objectId: string) => {
    setExpandedObjects((prev) => {
      const next = new Set(prev);
      if (next.has(objectId)) {
        next.delete(objectId);
        return next;
      }
      next.add(objectId);
      if (!objectAttachments.has(objectId)) {
        invoke<AttachmentInfo[]>('export_get_attachments', { accountId, objectId })
          .then((atts) => {
            setObjectAttachments((p) => {
              const n = new Map(p);
              n.set(objectId, atts);
              return n;
            });
          })
          .catch(() => {});
      }
      return next;
    });
  };

  const toggleAttachment = (
    attId: string,
    objectId: string,
    sectionType: string,
    allIdsInGroup: string[],
  ) => {
    setSelectedAttachmentIds((prev) => {
      const next = new Set(prev);
      const isAdding = !next.has(attId);
      if (isAdding) next.add(attId);
      else next.delete(attId);
      return next;
    });

    setSelectedObjectIds((prev) => {
      const next = new Set(prev);
      if (!next.has(objectId)) {
        next.add(objectId);
        setSelectedPageIds((pagePrev) => {
          const pageNext = new Set(pagePrev);
          const allSelectedNow = allIdsInGroup.every((oid) => next.has(oid));
          if (allSelectedNow) pageNext.add(sectionType);
          return pageNext;
        });
      }
      return next;
    });
  };

  const totalSelected = selectedObjectIds.size;

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

  // Password strength
  const pwStrength = assessPasswordStrength(exportPassword);
  const pwStrengthLabel: Record<PasswordStrength, string> = {
    none: '',
    weak: t('settings:password_weak'),
    medium: t('settings:password_medium'),
    strong: t('settings:password_strong'),
  };

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

    if (pwStrength === 'weak' && !showWeakWarning) {
      setShowWeakWarning(true);
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
        const selections = Array.from(importSelections.entries()).map(
          ([objectId, selected]) => ({ objectId, selected }),
        );
        const count = await invoke<number>('import_execute_advanced', {
          accountId,
          req: {
            selections,
            strategy: importStrategy,
            sourcePath: importPath,
            password: importPw,
          },
        });
        onSuccess(t('settings:import_success', { count }));
      } else {
        const count = await invoke<number>('import_execute', {
          accountId,
          filePath: importPath,
          password: importPw,
        });
        onSuccess(t('settings:import_success', { count }));
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
      <div
        style={{
          maxWidth: 640,
          margin: '0 auto',
          display: 'flex',
          flexDirection: 'column',
          gap: 16,
        }}
      >
        {/* Tab bar */}
        <div
          style={{
            display: 'flex',
            gap: 0,
            borderRadius: 8,
            overflow: 'hidden',
            border: '1px solid var(--border-subtle)',
          }}
        >
          {(['export', 'import'] as const).map((tabKey) => (
            <button
              key={tabKey}
              onClick={() => setTab(tabKey)}
              style={{
                flex: 1,
                padding: '10px',
                border: 'none',
                cursor: 'pointer',
                background: tab === tabKey ? 'var(--accent-primary)' : 'transparent',
                color: tab === tabKey ? 'white' : 'var(--text-primary)',
                fontSize: 14,
                fontWeight: 500,
              }}
            >
              {tabKey === 'export' ? t('settings:export') : t('settings:import')}
            </button>
          ))}
        </div>

        {tab === 'export' ? (
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
            showWeakWarning={showWeakWarning}
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
            pwStrength={pwStrength}
            pwStrengthLabel={pwStrengthLabel}
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
            onSetShowWeakWarning={setShowWeakWarning}
            onSetShowHintWarning={setShowHintWarning}
            onSetSelectedTags={(updater) => setSelectedTags(updater)}
            onSetIncludeAttachments={setIncludeAttachments}
            onSetIncludePreferences={setIncludePreferences}
            onSetIncludeBehavioral={setIncludeBehavioral}
            onSetShowWeakWarningAndExport={() => {
              setShowWeakWarning(false);
              handleExport();
            }}
            onToggleExpandedPage={toggleExpandedPage}
            onSetShowHintWarningAndExport={() => {
              skipHintCheckRef.current = true;
              setShowHintWarning(false);
              handleExport();
            }}
          />
        ) : (
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
        )}
      </div>
    </AppShell>
  );
}
