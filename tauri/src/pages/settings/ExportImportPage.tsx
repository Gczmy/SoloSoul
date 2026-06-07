import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { SecurePasswordInput } from '@/components/forms/PasswordInput';
import { useToastError } from '@/hooks/useToastError';
import { invoke } from '@tauri-apps/api/core';
import { save, open } from '@tauri-apps/plugin-dialog';
import { useAuthStore } from '@/stores/authStore';

// ── Types matching Rust backend ─────────────────────────────

interface PageGroup {
  sectionType: string;
  pageName: string;
  objectCount: number;
  objects: ObjectSummary[];
}

interface ObjectSummary {
  id: string;
  name: string;
  collectionType: string;
  sectionType: string;
  sensitivityLevel: string;
  createdAt: string;
  updatedAt: string;
  tags?: string[];
}

interface ExportScope {
  selectedPageIds: string[];
  selectedObjectIds: string[];
  selectedTags: string[];
  includeAttachments: boolean;
  includePreferences: boolean;
}

interface ExportEstimate {
  objectCount: number;
  attachmentCount: number;
  estimatedBytes: number;
}

interface ImportPreviewResponse {
  filePath: string;
  version: string;
  objectCount: number;
  hasAttachments: boolean;
  extraFiles: string[];
  exportTime: string | null;
  passwordHint: string | null;
}

interface DecryptedImportPreview {
  objects: ObjectSummary[];
  conflicts: ConflictInfo[];
  hasPreferences: boolean;
  hasAuditLog: boolean;
}

interface ConflictInfo {
  objectId: string;
  name: string;
}

type ImportStrategy = 'SkipExisting' | 'Overwrite' | 'Merge';

interface ImportSelection {
  objectId: string;
  selected: boolean;
}

// ── Password strength ───────────────────────────────────────

type PasswordStrength = 'none' | 'weak' | 'medium' | 'strong';

function assessPasswordStrength(pw: string): PasswordStrength {
  if (!pw) return 'none';
  let score = 0;
  if (pw.length >= 8) score++;
  if (pw.length >= 12) score++;
  if (/[a-z]/.test(pw) && /[A-Z]/.test(pw)) score++;
  if (/\d/.test(pw)) score++;
  if (/[^a-zA-Z0-9]/.test(pw)) score++;
  if (score <= 1) return 'weak';
  if (score <= 3) return 'medium';
  return 'strong';
}

// ── Component ───────────────────────────────────────────────

export function ExportImportPage() {
  const navigate = useNavigate();
  const { onError, onSuccess } = useToastError();
  const { t } = useTranslation(['settings', 'common']);
  const accountId = useAuthStore((s) => s.currentAccount?.id ?? '');

  const [tab, setTab] = useState<'export' | 'import'>('export');

  // ── Export state ────────────────────────────────────────────
  const [pageGroups, setPageGroups] = useState<PageGroup[]>([]);
  const [selectedPageIds, setSelectedPageIds] = useState<Set<string>>(new Set());
  const [selectedObjectIds, setSelectedObjectIds] = useState<Set<string>>(new Set());
  const [expandedPages, setExpandedPages] = useState<Set<string>>(new Set());
  const [exportPassword, setExportPassword] = useState('');
  const [exportPasswordConfirm, setExportPasswordConfirm] = useState('');
  const [exportHint, setExportHint] = useState('');
  const [savePath, setSavePath] = useState<string | null>(null);
  const [isExporting, setIsExporting] = useState(false);
  const [showWeakWarning, setShowWeakWarning] = useState(false);

  // ── Export estimate ──────────────────────────────────────────
  const [exportEstimate, setExportEstimate] = useState<ExportEstimate | null>(null);
  const [estimating, setEstimating] = useState(false);

  // ── P2: Export extras ────────────────────────────────────────
  const [includeAttachments, setIncludeAttachments] = useState(false);
  const [includePreferences, setIncludePreferences] = useState(false);

  // ── Import state ────────────────────────────────────────────
  const [importPath, setImportPath] = useState('');
  const [importPreview, setImportPreview] = useState<ImportPreviewResponse | null>(null);
  const [importPw, setImportPw] = useState('');
  const [decryptedPreview, setDecryptedPreview] = useState<DecryptedImportPreview | null>(null);
  const [isPreviewing, setIsPreviewing] = useState(false);
  const [isDecrypting, setIsDecrypting] = useState(false);
  const [isImporting, setIsImporting] = useState(false);

  // ── P2: Import strategy ─────────────────────────────────────
  const [importStrategy, setImportStrategy] = useState<ImportStrategy>('SkipExisting');
  const [importSelections, setImportSelections] = useState<Map<string, boolean>>(new Map());
  const [showStrategySelector, setShowStrategySelector] = useState(false);

  // ── Load scope tree ─────────────────────────────────────────
  const loadScope = useCallback(() => {
    if (!accountId) return;
    invoke<PageGroup[]>('export_get_scope_tree', { accountId })
      .then(setPageGroups)
      .catch(() => {});
  }, [accountId]);

  useEffect(() => {
    loadScope();
  }, [loadScope]);

  // ── Page checkbox ───────────────────────────────────────────
  const togglePage = (sectionType: string, objectIds: string[]) => {
    setSelectedPageIds((prev) => {
      const next = new Set(prev);
      if (next.has(sectionType)) {
        next.delete(sectionType);
        setSelectedObjectIds((oPrev) => {
          const oNext = new Set(oPrev);
          for (const id of objectIds) oNext.delete(id);
          return oNext;
        });
      } else {
        next.add(sectionType);
        setSelectedObjectIds((oPrev) => {
          const oNext = new Set(oPrev);
          for (const id of objectIds) oNext.add(id);
          return oNext;
        });
      }
      return next;
    });
  };

  const toggleObject = (id: string, sectionType: string, allIdsInGroup: string[]) => {
    setSelectedObjectIds((prev) => {
      const next = new Set(prev);
      const wasSelected = next.has(id);
      if (wasSelected) {
        next.delete(id);
      } else {
        next.add(id);
      }
      setSelectedPageIds((pPrev) => {
        const pNext = new Set(pPrev);
        const allSelectedNow = allIdsInGroup.every((oid) => next.has(oid));
        if (allSelectedNow) {
          pNext.add(sectionType);
        } else {
          pNext.delete(sectionType);
        }
        return pNext;
      });
      return next;
    });
  };

  const totalSelected = selectedObjectIds.size;

  // ── Estimate export size when selection changes ──────────────
  useEffect(() => {
    if (totalSelected === 0) {
      setExportEstimate(null);
      return;
    }
    const debounce = setTimeout(() => {
      setEstimating(true);
      invoke<ExportEstimate>('export_estimate_size', {
        accountId,
        scope: {
          selectedPageIds: Array.from(selectedPageIds),
          selectedObjectIds: Array.from(selectedObjectIds),
          selectedTags: [],
          includeAttachments,
          includePreferences,
        },
      })
        .then(setExportEstimate)
        .catch(() => setExportEstimate(null))
        .finally(() => setEstimating(false));
    }, 300);
    return () => clearTimeout(debounce);
  }, [totalSelected, selectedPageIds, selectedObjectIds, includeAttachments, includePreferences, accountId]);

  // ── Password strength ───────────────────────────────────────
  const pwStrength = assessPasswordStrength(exportPassword);
  const pwStrengthLabel: Record<PasswordStrength, string> = {
    none: '',
    weak: t('settings:password_weak'),
    medium: t('settings:password_medium'),
    strong: t('settings:password_strong'),
  };

  // ── Compute whether selected objects contain sensitive data ─
  const hasSensitiveData = (() => {
    for (const group of pageGroups) {
      for (const obj of group.objects) {
        if (selectedObjectIds.has(obj.id) && (obj.sensitivityLevel === 'sensitive' || obj.sensitivityLevel === 'critical')) {
          return true;
        }
      }
    }
    return false;
  })();

  // ── Collect all unique tags from selected objects ──────────
  const allTags = (() => {
    const tagSet = new Set<string>();
    for (const group of pageGroups) {
      for (const obj of group.objects) {
        if (selectedObjectIds.has(obj.id)) {
          // tags are on the ObjectSummary type but we don't have them in frontend type yet
        }
      }
    }
    return Array.from(tagSet);
  })();

  // ── Export handler ──────────────────────────────────────────
  const handleExport = async () => {
    if (totalSelected === 0 || !exportPassword || !savePath) return;

    if (exportPassword !== exportPasswordConfirm) {
      onError(new Error(t('settings:password_mismatch')), '');
      return;
    }

    if (pwStrength === 'weak' && !showWeakWarning) {
      setShowWeakWarning(true);
      return;
    }

    setIsExporting(true);
    try {
      const path = await invoke<string>('export_execute', {
        accountId,
        req: {
          scope: {
            selectedPageIds: Array.from(selectedPageIds),
            selectedObjectIds: Array.from(selectedObjectIds),
            selectedTags: [],
            includeAttachments,
            includePreferences,
          },
          password: exportPassword,
          passwordHint: exportHint || null,
          savePath,
        },
      });
      onSuccess(t('settings:export_success'));
    } catch (e) {
      onError(e, t('common:export_failed'));
    } finally {
      setIsExporting(false);
    }
  };

  // ── Import handlers ─────────────────────────────────────────
  const handlePreviewImport = async () => {
    if (!importPath || isPreviewing) return;
    setIsPreviewing(true);
    try {
      const preview = await invoke<ImportPreviewResponse>('import_parse_package', {
        filePath: importPath,
      });
      setImportPreview(preview);
      setDecryptedPreview(null);
    } catch (e) {
      onError(e, t('common:preview_failed'));
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
      // Initialize all selections to true
      const selMap = new Map<string, boolean>();
      for (const obj of preview.objects) {
        selMap.set(obj.id, true);
      }
      setImportSelections(selMap);
    } catch (e) {
      onError(e, t('common:decrypt_failed'));
    } finally {
      setIsDecrypting(false);
    }
  };

  const handleImport = async () => {
    if (!importPath || !importPw) return;
    setIsImporting(true);
    try {
      // If strategy selector is open, use advanced import
      if (showStrategySelector && decryptedPreview) {
        const selections: ImportSelection[] = Array.from(importSelections.entries()).map(
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
      onError(e, t('common:import_failed'));
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

  // ── Helpers ─────────────────────────────────────────────────
  const sensitivityBadge = (level: string) => {
    const colors: Record<string, string> = {
      public: 'var(--text-tertiary)',
      internal: 'var(--accent-primary)',
      sensitive: '#e68a00',
      critical: '#d32f2f',
    };
    return (
      <span
        style={{
          fontSize: 10,
          color: colors[level] || 'var(--text-tertiary)',
          border: '1px solid currentColor',
          borderRadius: 3,
          padding: '0 4px',
          lineHeight: '16px',
          textTransform: 'uppercase',
        }}
      >
        {level}
      </span>
    );
  };

  const formatBytes = (bytes: number) => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1048576) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / 1048576).toFixed(1)} MB`;
  };

  // ── Render ─────────────────────────────────────────────────
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
        {/* ── Tab bar ────────────────────────────────────────── */}
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

        {/* ═══════════════════════════════════════════════════════
            EXPORT TAB
           ═══════════════════════════════════════════════════════ */}
        {tab === 'export' && (
          <>
            <p style={{ fontSize: 13, color: 'var(--text-secondary)' }}>
              {t('settings:export_desc')}
            </p>

            {/* ── Page & Object tree ──────────────────────────── */}
            <Card>
              <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 8 }}>
                {t('settings:select_objects')}
              </h3>
              {pageGroups.length === 0 ? (
                <p style={{ fontSize: 13, color: 'var(--text-tertiary)' }}>
                  {t('common:no_data')}
                </p>
              ) : (
                <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
                  {pageGroups.map((group) => {
                    const allIds = group.objects.map((o) => o.id);
                    const pageChecked = selectedPageIds.has(group.sectionType);
                    const someChecked = !pageChecked && allIds.some((id) => selectedObjectIds.has(id));
                    const expanded = expandedPages.has(group.sectionType);
                    return (
                      <div key={group.sectionType}>
                        {/* Page row */}
                        <div
                          style={{
                            display: 'flex',
                            alignItems: 'center',
                            gap: 8,
                            padding: '8px 0',
                            cursor: 'pointer',
                            userSelect: 'none',
                          }}
                        >
                          <input
                            type="checkbox"
                            checked={pageChecked}
                            ref={(el) => {
                              if (el) el.indeterminate = someChecked && !pageChecked;
                            }}
                            onChange={() => togglePage(group.sectionType, allIds)}
                            style={{ accentColor: 'var(--accent-primary)' }}
                          />
                          <span
                            onClick={() => {
                              setExpandedPages((prev) => {
                                const next = new Set(prev);
                                if (next.has(group.sectionType)) next.delete(group.sectionType);
                                else next.add(group.sectionType);
                                return next;
                              });
                            }}
                            style={{
                              fontSize: 14,
                              fontWeight: 600,
                              flex: 1,
                              display: 'flex',
                              alignItems: 'center',
                              gap: 4,
                            }}
                          >
                            <span style={{ transform: expanded ? 'rotate(90deg)' : 'none', transition: 'transform 0.15s', fontSize: 10 }}>
                              ▶
                            </span>
                            {t(`navigation:${group.sectionType}`, group.pageName)}
                          </span>
                          <span style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>
                            {t('common:object_count', { n: group.objectCount })}
                          </span>
                        </div>

                        {/* Object rows (collapsible) */}
                        {expanded &&
                          group.objects.map((obj) => (
                            <label
                              key={obj.id}
                              style={{
                                display: 'flex',
                                alignItems: 'center',
                                gap: 8,
                                padding: '4px 0 4px 28px',
                                cursor: 'pointer',
                              }}
                            >
                              <input
                                type="checkbox"
                                checked={selectedObjectIds.has(obj.id)}
                                onChange={() => toggleObject(obj.id, group.sectionType, allIds)}
                                style={{ accentColor: 'var(--accent-primary)' }}
                              />
                              <span style={{ fontSize: 13, flex: 1 }}>{obj.name}</span>
                              {sensitivityBadge(obj.sensitivityLevel)}
                            </label>
                          ))}
                      </div>
                    );
                  })}
                </div>
              )}
            </Card>

            {/* ── P2: Export extras (attachments + preferences) ── */}
            {totalSelected > 0 && (
              <Card>
                <div style={{ fontSize: 13, color: 'var(--text-secondary)', padding: '4px 0' }}>
                  {estimating
                    ? t('settings:estimating')
                    : exportEstimate
                      ? `${exportEstimate.objectCount} ${t('settings:objects_count')}` +
                        (exportEstimate.attachmentCount > 0
                          ? ` + ${exportEstimate.attachmentCount} ${t('settings:attachments_count')}`
                          : '') +
                        ` · ${formatBytes(exportEstimate.estimatedBytes)}`
                      : ''}
                </div>
              </Card>
            )}
            <Card>
              <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 8 }}>
                {t('settings:export_options')}
              </h3>
              <label style={{ display: 'flex', alignItems: 'center', gap: 8, padding: '4px 0', cursor: 'pointer', fontSize: 13 }}>
                <input
                  type="checkbox"
                  checked={includeAttachments}
                  onChange={() => setIncludeAttachments(!includeAttachments)}
                  style={{ accentColor: 'var(--accent-primary)' }}
                />
                {t('settings:include_attachments')}
              </label>
              <label style={{ display: 'flex', alignItems: 'center', gap: 8, padding: '4px 0', cursor: 'pointer', fontSize: 13 }}>
                <input
                  type="checkbox"
                  checked={includePreferences}
                  onChange={() => setIncludePreferences(!includePreferences)}
                  style={{ accentColor: 'var(--accent-primary)' }}
                />
                {t('settings:include_preferences')}
              </label>
            </Card>

            {/* ── Save path ─────────────────────────────────── */}
            <Card>
              <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 8 }}>
                {t('common:export_path')}
              </h3>
              <div style={{ fontSize: 13, color: 'var(--text-secondary)', marginBottom: 8 }}>
                {savePath || t('settings:no_file_selected')}
              </div>
              <Button
                variant="secondary"
                size="sm"
                onClick={async () => {
                  const fp = await save({
                    filters: [{ name: 'SoloSoul Export', extensions: ['solosoul'] }],
                    defaultPath: `solosoul_export_${Date.now()}.solosoul`,
                  });
                  if (fp) setSavePath(fp);
                }}
              >
                {t('common:browse')}
              </Button>
            </Card>

            {/* ── Encryption ─────────────────────────────────── */}
            <Card>
              <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 8 }}>
                {t('settings:encryption')}
              </h3>

              {/* Sensitive data warning (P1) */}
              {hasSensitiveData && (
                <div
                  style={{
                    marginBottom: 10,
                    padding: '8px 12px',
                    background: '#fff3e0',
                    borderRadius: 6,
                    fontSize: 12,
                    color: '#663c00',
                    border: '1px solid #ffcc80',
                  }}
                >
                  {t('settings:sensitive_export_warning')}
                </div>
              )}

              <SecurePasswordInput
                value={exportPassword}
                onChange={(v) => {
                  setExportPassword(v);
                  setShowWeakWarning(false);
                }}
                placeholder={t('common:password_placeholder')}
              />
              <div style={{ marginTop: 8 }}>
                <SecurePasswordInput
                  value={exportPasswordConfirm}
                  onChange={(v) => setExportPasswordConfirm(v)}
                  placeholder={t('settings:confirm_password')}
                />
              </div>
              {exportPassword && exportPasswordConfirm && exportPassword !== exportPasswordConfirm && (
                <div style={{ marginTop: 4, fontSize: 12, color: '#d32f2f' }}>
                  {t('settings:password_mismatch')}
                </div>
              )}
              {exportPassword && (
                <div style={{ marginTop: 6, fontSize: 12, color: 'var(--text-secondary)' }}>
                  {t('settings:password_strength')}:{' '}
                  <span
                    style={{
                      color: pwStrength === 'weak' ? '#d32f2f' : pwStrength === 'medium' ? '#e68a00' : '#2e7d32',
                    }}
                  >
                    {pwStrengthLabel[pwStrength]}
                  </span>
                  {pwStrength === 'weak' && (
                    <span style={{ marginLeft: 8, color: '#d32f2f', fontSize: 11 }}>
                      {t('settings:password_weak_warning')}
                    </span>
                  )}
                </div>
              )}
              <div style={{ marginTop: 8 }}>
                <input
                  type="text"
                  value={exportHint}
                  onChange={(e) => setExportHint(e.target.value)}
                  placeholder={t('common:password_hint')}
                  maxLength={200}
                  style={{
                    width: '100%',
                    padding: '10px 14px',
                    fontSize: 14,
                    border: '1px solid var(--border-subtle)',
                    borderRadius: 8,
                    background: 'var(--bg-elevated)',
                    color: 'var(--text-primary)',
                    fontFamily: 'inherit',
                    outline: 'none',
                  }}
                />
              </div>
            </Card>

            {/* Weak password confirmation dialog (P1) */}
            {showWeakWarning && (
              <div
                style={{
                  padding: '12px 16px',
                  borderRadius: 8,
                  background: '#fff3e0',
                  border: '1px solid #ffcc80',
                  fontSize: 13,
                  color: '#663c00',
                }}
              >
                <p style={{ marginBottom: 8, fontWeight: 600 }}>{t('settings:weak_password_title')}</p>
                <p style={{ marginBottom: 10 }}>{t('settings:weak_password_confirm')}</p>
                <div style={{ display: 'flex', gap: 8 }}>
                  <Button size="sm" variant="secondary" onClick={() => setShowWeakWarning(false)}>
                    {t('common:cancel')}
                  </Button>
                  <Button size="sm" onClick={async () => {
                    setShowWeakWarning(false);
                    await handleExport();
                  }}>
                    {t('settings:export_anyway')}
                  </Button>
                </div>
              </div>
            )}

            <Button
              onClick={handleExport}
              loading={isExporting}
              disabled={totalSelected === 0 || !exportPassword || !savePath}
            >
              {t('settings:export_selected')} ({totalSelected})
            </Button>
          </>
        )}

        {/* ═══════════════════════════════════════════════════════
            IMPORT TAB
           ═══════════════════════════════════════════════════════ */}
        {tab === 'import' && (
          <>
            <p style={{ fontSize: 13, color: 'var(--text-secondary)' }}>
              {t('settings:import_desc')}
            </p>

            {/* ── File selector ──────────────────────────────── */}
            <Card>
              <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 8 }}>
                {t('settings:select_file')}
              </h3>
              <div style={{ fontSize: 13, color: 'var(--text-secondary)', marginBottom: 8 }}>
                {importPath || t('settings:no_file_selected')}
              </div>
              <Button
                size="sm"
                onClick={async () => {
                  const selected = await open({
                    filters: [{ name: 'SoloSoul Export', extensions: ['solosoul'] }],
                    multiple: false,
                  });
                  if (selected) {
                    setImportPath(selected as string);
                    setImportPreview(null);
                    setDecryptedPreview(null);
                    setImportPw('');
                    setShowStrategySelector(false);
                  }
                }}
              >
                {t('settings:select_file')}
              </Button>
              {importPath && !importPreview && (
                <div style={{ marginTop: 8 }}>
                  <Button
                    size="sm"
                    onClick={handlePreviewImport}
                    loading={isPreviewing}
                    disabled={isPreviewing}
                  >
                    {t('settings:preview')}
                  </Button>
                </div>
              )}
            </Card>

            {/* ── Parsed manifest preview ────────────────────── */}
            {importPreview && (
              <Card>
                <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 8 }}>
                  {t('settings:import_preview')}
                </h3>
                <div style={{ fontSize: 13, display: 'flex', flexDirection: 'column', gap: 6 }}>
                  <p>{t('settings:version')}: {importPreview.version}</p>
                  <p>{t('settings:export_time')}: {importPreview.exportTime || t('settings:unknown')}</p>
                  <p>{t('settings:objects_count')}: {importPreview.objectCount}</p>
                  {importPreview.hasAttachments && (
                    <p style={{ color: 'var(--accent-primary)' }}>
                      {t('settings:includes_attachments')}
                    </p>
                  )}
                  {importPreview.extraFiles.length > 0 && importPreview.extraFiles.includes('preferences.enc') && (
                    <p style={{ color: 'var(--accent-primary)' }}>
                      {t('settings:includes_preferences')}
                    </p>
                  )}
                </div>

                {/* ── Password hint (P1) ─────────────────────── */}
                {importPreview.passwordHint && (
                  <div
                    style={{
                      marginTop: 8,
                      padding: '8px 12px',
                      background: 'var(--bg-elevated-hover)',
                      borderRadius: 6,
                      fontSize: 13,
                      color: 'var(--text-secondary)',
                    }}
                  >
                    {t('settings:password_hint_label')}: {importPreview.passwordHint}
                  </div>
                )}

                {/* ── Password + decrypt ─────────────────────── */}
                <div style={{ marginTop: 12 }}>
                  <SecurePasswordInput
                    value={importPw}
                    onChange={(v) => setImportPw(v)}
                    placeholder={t('common:password_placeholder')}
                  />
                </div>
                {!decryptedPreview && (
                  <div style={{ marginTop: 8 }}>
                    <Button
                      onClick={handleDecryptPreview}
                      loading={isDecrypting}
                      disabled={!importPw || isDecrypting}
                    >
                      {t('settings:decrypt_and_preview')}
                    </Button>
                  </div>
                )}

                {/* ── Decrypted preview with conflicts ──────── */}
                {decryptedPreview && (
                  <>
                    <div
                      style={{
                        marginTop: 12,
                        borderTop: '1px solid var(--border-subtle)',
                        paddingTop: 12,
                      }}
                    >
                      <h4 style={{ fontSize: 13, fontWeight: 600, marginBottom: 6 }}>
                        {t('settings:objects_in_package')} ({decryptedPreview.objects.length})
                      </h4>

                      {/* Object selection list (P2) */}
                      <div style={{ maxHeight: 240, overflowY: 'auto', fontSize: 13 }}>
                        {decryptedPreview.objects.map((obj) => {
                          const isConflict = decryptedPreview.conflicts.some(
                            (c) => c.objectId === obj.id,
                          );
                          const isSelected = importSelections.get(obj.id) ?? true;
                          return (
                            <div
                              key={obj.id}
                              style={{
                                display: 'flex',
                                alignItems: 'center',
                                gap: 6,
                                padding: '3px 0',
                              }}
                            >
                              <input
                                type="checkbox"
                                checked={isSelected}
                                onChange={() => toggleImportSelection(obj.id)}
                                style={{ accentColor: 'var(--accent-primary)' }}
                              />
                              <span style={{ flex: 1 }}>{obj.name}</span>
                              {sensitivityBadge(obj.sensitivityLevel)}
                              {isConflict && (
                                <span
                                  style={{
                                    fontSize: 11,
                                    color: '#e68a00',
                                    border: '1px solid #e68a00',
                                    borderRadius: 3,
                                    padding: '0 4px',
                                  }}
                                >
                                  {t('settings:conflict')}
                                </span>
                              )}
                            </div>
                          );
                        })}
                      </div>

                      {/* Conflict summary */}
                      {decryptedPreview.conflicts.length > 0 && (
                        <div
                          style={{
                            marginTop: 8,
                            padding: '8px 12px',
                            background: '#fff3e0',
                            borderRadius: 6,
                            fontSize: 12,
                            color: '#663c00',
                          }}
                        >
                          {t('settings:conflict_warning', { count: decryptedPreview.conflicts.length })}
                        </div>
                      )}
                    </div>

                    {/* ── P2: Strategy selector + import button ── */}
                    {!showStrategySelector ? (
                      <div style={{ marginTop: 8 }}>
                        <Button onClick={() => setShowStrategySelector(true)} size="sm" variant="secondary" style={{ marginRight: 8 }}>
                          {t('settings:advanced_import')}
                        </Button>
                        <Button onClick={handleImport} loading={isImporting} disabled={!importPw || isImporting}>
                          {t('settings:quick_import')}
                        </Button>
                      </div>
                    ) : (
                      <div
                        style={{
                          marginTop: 12,
                          padding: 12,
                          border: '1px solid var(--border-subtle)',
                          borderRadius: 8,
                        }}
                      >
                        <h4 style={{ fontSize: 13, fontWeight: 600, marginBottom: 8 }}>
                          {t('settings:import_strategy_title')}
                        </h4>
                        {(['SkipExisting', 'Overwrite', 'Merge'] as ImportStrategy[]).map((s) => (
                          <label
                            key={s}
                            style={{
                              display: 'flex',
                              alignItems: 'center',
                              gap: 8,
                              padding: '6px 0',
                              cursor: 'pointer',
                              fontSize: 13,
                            }}
                          >
                            <input
                              type="radio"
                              checked={importStrategy === s}
                              onChange={() => setImportStrategy(s)}
                              style={{ accentColor: 'var(--accent-primary)' }}
                            />
                            <div>
                              <strong>{t(`settings:strategy_${s}`)}</strong>
                              <p style={{ fontSize: 11, color: 'var(--text-tertiary)', margin: 0 }}>
                                {t(`settings:strategy_${s}_desc`)}
                              </p>
                            </div>
                          </label>
                        ))}
                        <div style={{ marginTop: 8, display: 'flex', gap: 8 }}>
                          <Button size="sm" variant="secondary" onClick={() => setShowStrategySelector(false)}>
                            {t('common:cancel')}
                          </Button>
                          <Button onClick={handleImport} loading={isImporting} disabled={!importPw || isImporting}>
                            {t('settings:import_action')} ({importSelections.size})
                          </Button>
                        </div>
                      </div>
                    )}
                  </>
                )}
              </Card>
            )}
            {importPreview && !decryptedPreview && (
              <p style={{ fontSize: 12, color: 'var(--text-tertiary)', textAlign: 'center' }}>
                {t('settings:password_required_for_decrypt')}
              </p>
            )}
          </>
        )}
      </div>
    </AppShell>
  );
}
