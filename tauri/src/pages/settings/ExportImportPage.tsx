import { useState, useEffect, useCallback, useRef, useMemo } from 'react';
import { createPortal } from 'react-dom';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { SensitivityBadge } from '@/components/ui/SensitivityBadge';
import { SecurePasswordInput } from '@/components/forms/PasswordInput';
import { useToastError } from '@/hooks/useToastError';
import { resolveBackendErrorMessage } from '@/lib/backendError';
import { invoke } from '@tauri-apps/api/core';
import { save, open } from '@tauri-apps/plugin-dialog';
import { useAuthStore } from '@/stores/authStore';
import type { SensitivityLevel } from '@/components/ui/SensitivityBadge';
import { Paperclip, Info } from 'lucide-react';
import { formatBytes } from '@/lib/format';

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
  tags: string[];
}

interface AttachmentInfo {
  id: string;
  fileName: string;
  sizeBytes: number;
}

interface ExportEstimate {
  objectCount: number;
  attachmentCount: number;
  attachmentSelectedCount: number;
  estimatedBytes: number;
}

interface ImportPreview {
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

type ImportStrategy = 'skipExisting' | 'overwrite' | 'merge';

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

/** Hover info card explaining attachment export limits. */
function AttachmentLimitsInfo() {
  const { t } = useTranslation('settings');
  const [show, setShow] = useState(false);
  const btnRef = useRef<HTMLButtonElement>(null);
  const [pos, setPos] = useState<{ top: number; left: number } | null>(null);

  useEffect(() => {
    if (show && btnRef.current) {
      const rect = btnRef.current.getBoundingClientRect();
      setPos({ top: rect.bottom + 6, left: rect.left });
    }
  }, [show]);

  return (
    <div style={{ display: 'inline-flex', alignItems: 'center' }}>
      <button
        ref={btnRef}
        type="button"
        onMouseEnter={() => setShow(true)}
        onMouseLeave={() => setShow(false)}
        aria-label={t('attachment_limits_title')}
        style={{
          background: 'none',
          border: 'none',
          padding: 2,
          display: 'flex',
          alignItems: 'center',
          color: 'var(--text-tertiary)',
          cursor: 'pointer',
        }}
      >
        <Info size={14} />
      </button>
      {show &&
        pos &&
        createPortal(
          <div
            style={{
              position: 'fixed',
              top: pos.top,
              left: pos.left,
              zIndex: 5000,
              background: 'var(--bg-elevated)',
              border: '1px solid var(--border-subtle)',
              borderRadius: 8,
              padding: 12,
              boxShadow: 'var(--shadow-md)',
              fontSize: 12,
              color: 'var(--text-secondary)',
              maxWidth: 520,
              lineHeight: 1.5,
            }}
          >
            <div
              style={{
                fontWeight: 600,
                marginBottom: 8,
                color: 'var(--text-primary)',
              }}
            >
              {t('attachment_limits_title')}
            </div>
            <table style={{ borderCollapse: 'collapse', width: '100%' }}>
              <thead>
                <tr style={{ borderBottom: '1px solid var(--border-subtle)' }}>
                  <th style={{ textAlign: 'left', padding: '4px 8px', fontWeight: 600 }}>
                    {t('attachment_limits_type')}
                  </th>
                  <th style={{ textAlign: 'left', padding: '4px 8px', fontWeight: 600 }}>
                    {t('attachment_limits_threshold')}
                  </th>
                  <th style={{ textAlign: 'left', padding: '4px 8px', fontWeight: 600 }}>
                    {t('attachment_limits_behavior')}
                  </th>
                </tr>
              </thead>
              <tbody>
                <tr style={{ borderBottom: '1px solid var(--border-subtle)' }}>
                  <td style={{ padding: '4px 8px' }}>{t('attachment_limit_single_size')}</td>
                  <td style={{ padding: '4px 8px' }}>100 MB</td>
                  <td style={{ padding: '4px 8px' }}>
                    {t('attachment_limit_single_size_behavior')}
                  </td>
                </tr>
                <tr style={{ borderBottom: '1px solid var(--border-subtle)' }}>
                  <td style={{ padding: '4px 8px' }}>{t('attachment_limit_single_count')}</td>
                  <td style={{ padding: '4px 8px' }}>50</td>
                  <td style={{ padding: '4px 8px' }}>
                    {t('attachment_limit_single_count_behavior')}
                  </td>
                </tr>
                <tr>
                  <td style={{ padding: '4px 8px' }}>{t('attachment_limit_total_size')}</td>
                  <td style={{ padding: '4px 8px' }}>1 GB</td>
                  <td style={{ padding: '4px 8px' }}>
                    {t('attachment_limit_total_size_behavior')}
                  </td>
                </tr>
              </tbody>
            </table>
          </div>,
          document.body,
        )}
    </div>
  );
}

/** Cancel button styled for the hard-coded warning panel background (#fff3e0). */
function WarningCancelButton({ onClick, children }: { onClick: () => void; children: string }) {
  const [hovered, setHovered] = useState(false);
  return (
    <button
      type="button"
      onClick={onClick}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      style={{
        padding: '6px 12px',
        fontSize: 13,
        borderRadius: 6,
        border: '1px solid #ffcc80',
        background: hovered ? '#ffffff' : 'rgba(255, 255, 255, 0.85)',
        color: '#663c00',
        cursor: 'pointer',
        fontWeight: 500,
        transition: 'background 0.15s',
        fontFamily: 'inherit',
      }}
    >
      {children}
    </button>
  );
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
  const [showHintWarning, setShowHintWarning] = useState(false);
  const skipHintCheckRef = useRef(false);

  // ── Export estimate ──────────────────────────────────────────
  const [exportEstimate, setExportEstimate] = useState<ExportEstimate | null>(null);
  const [estimating, setEstimating] = useState(false);

  // ── P1: Tag filter ──────────────────────────────────────────
  const [selectedTags, setSelectedTags] = useState<Set<string>>(new Set());

  // ── P1/P2: Export extras ─────────────────────────────────────
  const [includeAttachments, setIncludeAttachments] = useState(false);
  const [selectedAttachmentIds, setSelectedAttachmentIds] = useState<Set<string>>(new Set());
  const [objectAttachments, setObjectAttachments] = useState<Map<string, AttachmentInfo[]>>(
    new Map(),
  );
  const [expandedObjects, setExpandedObjects] = useState<Set<string>>(new Set());
  const [includePreferences, setIncludePreferences] = useState(false);
  const [includeBehavioral, setIncludeBehavioral] = useState(false);

  // ── Import state ────────────────────────────────────────────
  const [importPath, setImportPath] = useState('');
  const [importPreview, setImportPreview] = useState<ImportPreview | null>(null);
  const [importPw, setImportPw] = useState('');
  const [decryptedPreview, setDecryptedPreview] = useState<DecryptedImportPreview | null>(null);
  const [isPreviewing, setIsPreviewing] = useState(false);
  const [isDecrypting, setIsDecrypting] = useState(false);
  const [isImporting, setIsImporting] = useState(false);

  // ── P2: Import strategy ─────────────────────────────────────
  const [importStrategy, setImportStrategy] = useState<ImportStrategy>('skipExisting');
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
      const isAdding = !next.has(sectionType);
      if (isAdding) {
        next.add(sectionType);
      } else {
        next.delete(sectionType);
      }

      setSelectedObjectIds((oPrev) => {
        const oNext = new Set(oPrev);
        for (const id of objectIds) {
          if (isAdding) oNext.add(id);
          else oNext.delete(id);
        }
        return oNext;
      });

      // Cascade: add/remove attachments for already-loaded objects
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

      // Async load attachments for unloaded objects when adding
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
              for (const { id, atts } of results) {
                n.set(id, atts);
              }
              return n;
            });
            setSelectedAttachmentIds((prev) => {
              const n = new Set(prev);
              for (const { atts } of results) {
                for (const att of atts) n.add(att.id);
              }
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
      if (isAdding) {
        next.add(id);
      } else {
        next.delete(id);
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

      // Cascade attachments for already-loaded object
      setSelectedAttachmentIds((attPrev) => {
        const attNext = new Set(attPrev);
        const atts = objectAttachments.get(id) || [];
        for (const att of atts) {
          if (isAdding) attNext.add(att.id);
          else attNext.delete(att.id);
        }
        return attNext;
      });

      // Async load attachments if adding and not yet loaded
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

  const totalSelected = selectedObjectIds.size;

  // ── Attachment helpers ──────────────────────────────────────
  const toggleObjectExpanded = async (objectId: string) => {
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

    // Cascade: if adding and parent object not selected, select it (and page if all objects selected)
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
          selectedTags: Array.from(selectedTags),
          includeAttachments,
          selectedAttachmentIds: Array.from(selectedAttachmentIds),
          includePreferences,
          includeBehavioral,
        },
      })
        .then(setExportEstimate)
        .catch(() => setExportEstimate(null))
        .finally(() => setEstimating(false));
    }, 300);
    return () => clearTimeout(debounce);
  }, [
    totalSelected,
    selectedPageIds,
    selectedObjectIds,
    selectedTags,
    includeAttachments,
    selectedAttachmentIds,
    includePreferences,
    includeBehavioral,
    accountId,
  ]);

  // ── Password strength ───────────────────────────────────────
  const pwStrength = assessPasswordStrength(exportPassword);
  const pwStrengthLabel: Record<PasswordStrength, string> = {
    none: '',
    weak: t('settings:password_weak'),
    medium: t('settings:password_medium'),
    strong: t('settings:password_strong'),
  };

  // ── Compute whether selected objects contain sensitive data ─
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

  // ── Collect all unique tags from selected objects ──────────
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

  // ── Export handler ──────────────────────────────────────────
  const handleExport = async () => {
    if (totalSelected === 0 || !exportPassword || !savePath) return;

    if (exportPassword !== exportPasswordConfirm) {
      onError(new Error(t('settings:password_mismatch')), '');
      return;
    }

    // Warn if the password hint contains parts of the password
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

  // ── Import handlers ─────────────────────────────────────────
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
      // Initialize all selections to true
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

  // ── Helpers ─────────────────────────────────────────────────
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
                <p style={{ fontSize: 13, color: 'var(--text-tertiary)' }}>{t('common:no_data')}</p>
              ) : (
                <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
                  {pageGroups.map((group) => {
                    const allIds = group.objects.map((o) => o.id);
                    const pageChecked = selectedPageIds.has(group.sectionType);
                    const someChecked =
                      !pageChecked && allIds.some((id) => selectedObjectIds.has(id));
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
                            <span
                              style={{
                                transform: expanded ? 'rotate(90deg)' : 'none',
                                transition: 'transform 0.15s',
                                fontSize: 10,
                              }}
                            >
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
                            <div key={obj.id}>
                              <label
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
                                <SensitivityBadge
                                  level={obj.sensitivityLevel as SensitivityLevel}
                                />
                                {includeAttachments && (
                                  <button
                                    type="button"
                                    onClick={(e) => {
                                      e.preventDefault();
                                      e.stopPropagation();
                                      toggleObjectExpanded(obj.id);
                                    }}
                                    style={{
                                      fontSize: 10,
                                      background: 'none',
                                      border: 'none',
                                      cursor: 'pointer',
                                      padding: '0 4px',
                                      transform: expandedObjects.has(obj.id)
                                        ? 'rotate(90deg)'
                                        : 'none',
                                      transition: 'transform 0.15s',
                                      color: 'var(--text-tertiary)',
                                    }}
                                  >
                                    ▶
                                  </button>
                                )}
                              </label>
                              {includeAttachments && expandedObjects.has(obj.id) && (
                                <div style={{ paddingLeft: 52, paddingBottom: 4 }}>
                                  {(objectAttachments.get(obj.id) || []).length === 0 ? (
                                    <span style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>
                                      {t('settings:no_attachments', 'No attachments')}
                                    </span>
                                  ) : (
                                    <>
                                      {/* Attachment list header */}
                                      <div
                                        style={{
                                          display: 'flex',
                                          alignItems: 'center',
                                          gap: 4,
                                          padding: '2px 0',
                                          fontSize: 11,
                                          color: 'var(--text-tertiary)',
                                          borderBottom: '1px solid var(--border-subtle)',
                                          marginBottom: 2,
                                        }}
                                      >
                                        <Paperclip size={10} />
                                        <span>
                                          {t('settings:attachments_label', 'Attachments')} (
                                          {(objectAttachments.get(obj.id) || []).length})
                                        </span>
                                      </div>
                                      {(objectAttachments.get(obj.id) || []).map((att) => (
                                        <label
                                          key={att.id}
                                          style={{
                                            display: 'flex',
                                            alignItems: 'center',
                                            gap: 6,
                                            padding: '2px 0 2px 16px',
                                            cursor: 'pointer',
                                          }}
                                        >
                                          <input
                                            type="checkbox"
                                            checked={selectedAttachmentIds.has(att.id)}
                                            onChange={() =>
                                              toggleAttachment(
                                                att.id,
                                                obj.id,
                                                group.sectionType,
                                                allIds,
                                              )
                                            }
                                            style={{ accentColor: 'var(--accent-primary)' }}
                                          />
                                          <Paperclip
                                            size={10}
                                            style={{ color: 'var(--text-tertiary)', flexShrink: 0 }}
                                          />
                                          <span style={{ fontSize: 12, flex: 1 }}>
                                            {att.fileName}
                                          </span>
                                          <span
                                            style={{ fontSize: 11, color: 'var(--text-tertiary)' }}
                                          >
                                            {formatBytes(att.sizeBytes)}
                                          </span>
                                        </label>
                                      ))}
                                    </>
                                  )}
                                </div>
                              )}
                            </div>
                          ))}
                      </div>
                    );
                  })}
                </div>
              )}
            </Card>

            {/* ── P1: Tag filter ───────────────────────────────── */}
            {allTags.length > 0 && (
              <Card>
                <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 8 }}>
                  {t('settings:filter_by_tags')}
                </h3>
                <div style={{ display: 'flex', flexWrap: 'wrap', gap: 6 }}>
                  {allTags.map((tag) => {
                    const isSelected = selectedTags.has(tag);
                    return (
                      <button
                        key={tag}
                        onClick={() => {
                          setSelectedTags((prev) => {
                            const next = new Set(prev);
                            if (next.has(tag)) next.delete(tag);
                            else next.add(tag);
                            return next;
                          });
                        }}
                        style={{
                          fontSize: 12,
                          padding: '4px 10px',
                          borderRadius: 12,
                          border: '1px solid var(--border-subtle)',
                          background: isSelected ? 'var(--accent-primary)' : 'var(--bg-elevated)',
                          color: isSelected ? 'white' : 'var(--text-primary)',
                          cursor: 'pointer',
                        }}
                      >
                        {tag}
                      </button>
                    );
                  })}
                </div>
              </Card>
            )}

            {/* ── Export size estimate ── */}
            {totalSelected > 0 && (
              <Card>
                <div
                  style={{
                    display: 'flex',
                    justifyContent: 'space-between',
                    alignItems: 'center',
                    fontSize: 13,
                    padding: '4px 0',
                  }}
                >
                  <span style={{ fontWeight: 600, color: 'var(--text-primary)' }}>
                    {t('settings:export_estimate_label', 'Export file estimated size')}
                  </span>
                  <span style={{ color: 'var(--text-secondary)' }}>
                    {estimating
                      ? t('settings:estimating')
                      : exportEstimate
                        ? `${t('settings:objects_count', { n: exportEstimate.objectCount })}` +
                          (exportEstimate.attachmentSelectedCount > 0
                            ? ` + ${t('settings:attachments_count', { n: exportEstimate.attachmentSelectedCount })}`
                            : '') +
                          ` · ${formatBytes(exportEstimate.estimatedBytes)}`
                        : ''}
                  </span>
                </div>
              </Card>
            )}
            <Card>
              <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 8 }}>
                {t('settings:export_options')}
              </h3>
              <div style={{ padding: '4px 0' }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                  <label
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      gap: 8,
                      cursor: 'pointer',
                      fontSize: 13,
                    }}
                  >
                    <input
                      type="checkbox"
                      checked={includeAttachments}
                      onChange={() => setIncludeAttachments(!includeAttachments)}
                      style={{ accentColor: 'var(--accent-primary)' }}
                    />
                    {t('settings:include_attachments')}
                  </label>
                  <AttachmentLimitsInfo />
                </div>
                <div
                  style={{
                    paddingLeft: 24,
                    fontSize: 11,
                    color: 'var(--text-tertiary)',
                    marginTop: 2,
                  }}
                >
                  {t('settings:include_attachments_desc')}
                </div>
              </div>
              <div style={{ padding: '4px 0' }}>
                <label
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: 8,
                    cursor: 'pointer',
                    fontSize: 13,
                  }}
                >
                  <input
                    type="checkbox"
                    checked={includePreferences}
                    onChange={() => setIncludePreferences(!includePreferences)}
                    style={{ accentColor: 'var(--accent-primary)' }}
                  />
                  {t('settings:include_preferences')}
                </label>
                <div
                  style={{
                    paddingLeft: 24,
                    fontSize: 11,
                    color: 'var(--text-tertiary)',
                    marginTop: 2,
                  }}
                >
                  {t('settings:include_preferences_desc')}
                </div>
              </div>
              <div style={{ padding: '4px 0' }}>
                <label
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: 8,
                    cursor: 'pointer',
                    fontSize: 13,
                  }}
                >
                  <input
                    type="checkbox"
                    checked={includeBehavioral}
                    onChange={() => setIncludeBehavioral(!includeBehavioral)}
                    style={{ accentColor: 'var(--accent-primary)' }}
                  />
                  {t('settings:include_behavioral')}
                </label>
                <div
                  style={{
                    paddingLeft: 24,
                    fontSize: 11,
                    color: 'var(--text-tertiary)',
                    marginTop: 2,
                  }}
                >
                  {t('settings:include_behavioral_desc')}
                </div>
              </div>
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
                showHintButton={false}
              />
              <div style={{ marginTop: 8 }}>
                <SecurePasswordInput
                  value={exportPasswordConfirm}
                  onChange={(v) => setExportPasswordConfirm(v)}
                  placeholder={t('settings:confirm_password')}
                  showHintButton={false}
                />
              </div>
              {exportPassword &&
                exportPasswordConfirm &&
                exportPassword !== exportPasswordConfirm && (
                  <div style={{ marginTop: 4, fontSize: 12, color: '#d32f2f' }}>
                    {t('settings:password_mismatch')}
                  </div>
                )}
              {exportPassword && (
                <div style={{ marginTop: 6, fontSize: 12, color: 'var(--text-secondary)' }}>
                  {t('settings:password_strength')}:{' '}
                  <span
                    style={{
                      color:
                        pwStrength === 'weak'
                          ? '#d32f2f'
                          : pwStrength === 'medium'
                            ? '#e68a00'
                            : '#2e7d32',
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

            {/* Password hint risk confirmation dialog */}
            {showHintWarning && (
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
                <p style={{ marginBottom: 8, fontWeight: 600 }}>
                  {t('settings:hint_contains_password_title')}
                </p>
                <p style={{ marginBottom: 10 }}>{t('settings:hint_contains_password_confirm')}</p>
                <div style={{ display: 'flex', gap: 8 }}>
                  <WarningCancelButton onClick={() => setShowHintWarning(false)}>
                    {t('common:cancel')}
                  </WarningCancelButton>
                  <Button
                    size="sm"
                    onClick={async () => {
                      skipHintCheckRef.current = true;
                      setShowHintWarning(false);
                      await handleExport();
                    }}
                  >
                    {t('settings:export_anyway')}
                  </Button>
                </div>
              </div>
            )}

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
                <p style={{ marginBottom: 8, fontWeight: 600 }}>
                  {t('settings:weak_password_title')}
                </p>
                <p style={{ marginBottom: 10 }}>{t('settings:weak_password_confirm')}</p>
                <div style={{ display: 'flex', gap: 8 }}>
                  <WarningCancelButton onClick={() => setShowWeakWarning(false)}>
                    {t('common:cancel')}
                  </WarningCancelButton>
                  <Button
                    size="sm"
                    onClick={async () => {
                      setShowWeakWarning(false);
                      await handleExport();
                    }}
                  >
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
                  <p>
                    {t('settings:version')}: {importPreview.version}
                  </p>
                  <p>
                    {t('settings:export_time')}: {importPreview.exportTime || t('settings:unknown')}
                  </p>
                  <p>{t('settings:objects_count', { n: importPreview.objectCount })}</p>
                  {importPreview.hasAttachments && (
                    <p style={{ color: 'var(--accent-primary)' }}>
                      {t('settings:includes_attachments')}
                    </p>
                  )}
                  {importPreview.extraFiles.length > 0 &&
                    importPreview.extraFiles.includes('preferences.enc') && (
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
                    showHintButton={false}
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
                              <SensitivityBadge level={obj.sensitivityLevel as SensitivityLevel} />
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
                          {t('settings:conflict_warning', {
                            count: decryptedPreview.conflicts.length,
                          })}
                        </div>
                      )}
                    </div>

                    {/* ── P2: Strategy selector + import button ── */}
                    {!showStrategySelector ? (
                      <div style={{ marginTop: 8 }}>
                        <Button
                          onClick={() => setShowStrategySelector(true)}
                          size="sm"
                          variant="secondary"
                          style={{ marginRight: 8 }}
                        >
                          {t('settings:advanced_import')}
                        </Button>
                        <Button
                          onClick={handleImport}
                          loading={isImporting}
                          disabled={!importPw || isImporting}
                        >
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
                        {(['skipExisting', 'overwrite', 'merge'] as ImportStrategy[]).map((s) => (
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
                          <Button
                            size="sm"
                            variant="secondary"
                            onClick={() => setShowStrategySelector(false)}
                          >
                            {t('common:cancel')}
                          </Button>
                          <Button
                            onClick={handleImport}
                            loading={isImporting}
                            disabled={!importPw || isImporting}
                          >
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
