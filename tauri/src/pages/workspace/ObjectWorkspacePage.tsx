import { useState, useEffect, useCallback, useRef } from 'react';
import { useNavigate, useSearchParams, useParams } from 'react-router-dom';
import React from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { Input } from '@/components/ui/Input';
import { Button } from '@/components/ui/Button';
import { useAuthStore } from '@/stores/authStore';
import { useObjectStore } from '@/stores/objectStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { useTemplateStore } from '@/stores/templateStore';
import { SensitivityBadge, getSensitivityStyle, type SensitivityLevel } from '@/components/ui/SensitivityBadge';
import { DeprecatedBadge } from '@/components/ui/DeprecatedBadge';
import { HistoryViewer } from '@/components/object/HistoryViewer';
import { AttachmentViewer } from '@/components/object/AttachmentViewer';
import { Pencil, Trash2, Trash, Clock, Paperclip, X, FileText } from 'lucide-react';
import { PasswordVerificationDialog } from '@/components/forms/PasswordVerificationDialog';
import { ObjectDetailModal } from '@/components/object/ObjectDetailModal';
import { PAGE_ICON_MAP } from '@/lib/pageIcons';

// Labels resolved at render time via t() so they support i18n
const CATEGORY_TYPES = ['identity', 'travel', 'financial', 'professional'] as const;
const CATEGORY_ICONS: Record<string, typeof PAGE_ICON_MAP.profile> = {
  identity: PAGE_ICON_MAP.profile,
  travel: PAGE_ICON_MAP.travel,
  financial: PAGE_ICON_MAP.financial,
  professional: PAGE_ICON_MAP.professional,
};

/** Extract displayable key-value pairs from object properties (filters internal __ fields).
 *  When `fieldOrder` is provided, fields are sorted to match the template definition order.
 */
function flattenProperties(
  props: Record<string, unknown> | undefined,
  fieldOrder?: string[]
): { key: string; value: string }[] {
  if (!props) return [];
  const entries: { key: string; value: string }[] = [];
  for (const [k, v] of Object.entries(props)) {
    if (k.startsWith('__')) continue; // skip internal fields like __attachments
    if (v === null || v === undefined || v === '') continue;
    if (typeof v === 'string') {
      entries.push({ key: k, value: v });
    } else if (typeof v === 'number' || typeof v === 'boolean') {
      entries.push({ key: k, value: String(v) });
    } else if (Array.isArray(v) && v.length > 0) {
      entries.push({ key: k, value: v.join(', ') });
    }
  }
  if (fieldOrder && fieldOrder.length > 0) {
    const orderMap = new Map(fieldOrder.map((id, i) => [id, i]));
    entries.sort((a, b) => {
      const ia = orderMap.get(a.key);
      const ib = orderMap.get(b.key);
      if (ia !== undefined && ib !== undefined) return ia - ib;
      if (ia !== undefined) return -1;
      if (ib !== undefined) return 1;
      return a.key.localeCompare(b.key);
    });
  }
  return entries;
}

export function ObjectWorkspacePage() {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const { pageId } = useParams();
  const sectionFilter = searchParams.get('section') || '';
  const detailObjectId = searchParams.get('objectId');
  const [searchQuery, setSearchQuery] = useState('');
  const [, setDeletingId] = useState<string | null>(null);
  const [confirmDelete, setConfirmDelete] = useState<{ id: string; name: string } | null>(null);
  const [confirmPageDelete, setConfirmPageDelete] = useState(false);
  const [historyObj, setHistoryObj] = useState<{ id: string; collectionType: string; templateId?: string } | null>(null);
  const [snapshotCounts, setSnapshotCounts] = useState<Record<string, number>>({});
  const [attachmentObjId, setAttachmentObjId] = useState<string | null>(null);
  const [attachmentCounts, setAttachmentCounts] = useState<Record<string, number>>({});
  const [detailObj, setDetailObj] = useState<typeof visibleObjects[number] | null>(null);

  const accountId = useAuthStore((s) => s.currentAccount?.id);
  const { t } = useTranslation(['common', 'navigation', 'editor']);
  const { objects, loadObjects, deleteObject, isLoading, error } = useObjectStore();
  const customPages = useSettingsStore((s) => s.settings.customPages);
  const activeCustomPages = customPages.filter((p) => !p.deletedAt);
  const removeCustomPage = useSettingsStore((s) => s.removeCustomPage);
  const { templates: userTemplates, loadTemplates: loadUserTemplates } = useTemplateStore();

  useEffect(() => {
    loadUserTemplates().catch(() => {});
  }, [loadUserTemplates]);

  // Open object detail modal directly when navigated with ?objectId=... (e.g. from search)
  useEffect(() => {
    if (!detailObjectId || !accountId) return;
    invoke('object_get', { objectId: detailObjectId })
      .then((obj) => setDetailObj(obj as typeof visibleObjects[number]))
      .catch(() => {});
  }, [detailObjectId, accountId]);

  const customPage = pageId ? customPages.find((p) => p.id === pageId) : null;

  const resolveCollectionLabel = (collectionType: string) => {
    if (['identity', 'travel', 'financial', 'professional'].includes(collectionType)) {
      return t(`navigation:${collectionType}`);
    }
    const cp = customPages.find((p) => p.id === collectionType);
    return cp?.name || collectionType;
  };

  const activeCategoryLabel = sectionFilter ? t(`navigation:${sectionFilter}`, sectionFilter) : null;

  /** Password dialog state — shared between detail panel and history viewer. */
  const [showPwDialog, setShowPwDialog] = useState(false);
  const pwResolveRef = useRef<((ok: boolean) => void) | null>(null);
  const [bioAvailable, setBioAvailable] = useState<{ available: boolean; biometryType?: string }>({ available: false });
  const [passwordHint, setPasswordHint] = useState<string | null>(null);

  // Check biometric availability on mount + load password hint
  useEffect(() => {
    invoke<{ available: boolean; configured: boolean; biometryType?: string }>('biometric_check_availability', { accountId: accountId || '' })
      .then((r) => setBioAvailable({ available: r.available && r.configured, biometryType: r.biometryType }))
      .catch(() => {});
    if (accountId) {
      invoke<Array<{ id: string; passwordHint?: string }>>('list_accounts').then((accounts) => {
        const acc = accounts.find((a) => a.id === accountId);
        setPasswordHint(acc?.passwordHint || null);
      }).catch(() => { /* ignore */ });
    }
  }, [accountId]);

  const passwordVerify = useCallback(async (): Promise<boolean> => {
    return new Promise((resolve) => {
      pwResolveRef.current = resolve;
      setShowPwDialog(true);
    });
  }, []);

  /** Verify password against vault — used by PasswordVerificationDialog. */
  const verifyVaultPassword = useCallback(async (password: string): Promise<boolean> => {
    if (accountId) {
      try {
        const ok = await invoke<boolean>('verify_password', { accountId, password });
        if (ok) return true;
      } catch { /* ignore */ }
    }
    const accounts = await invoke<Array<{ id: string; passwordHint?: string }>>('list_accounts').catch(() => []);
    for (const acc of accounts) {
      try {
        const ok = await invoke<boolean>('verify_password', { accountId: acc.id, password });
        if (ok) return true;
      } catch { /* ignore */ }
    }
    return false;
  }, [accountId]);

  /** Biometric unlock handler — used by PasswordVerificationDialog. */
  const handleBiometricUnlock = useCallback(async (): Promise<boolean> => {
    if (!accountId) return false;
    try {
      await invoke('biometric_unlock', { accountId, location: 'critical_data_access', action: 'unlock' });
      pwResolveRef.current?.(true);
      return true;
    } catch {
      return false;
    }
  }, [accountId]);

  /** Resolve sensitivity level for a property key via its template definition. */
  const getFieldSensitivity = (templateId: string | undefined, fieldKey: string): SensitivityLevel => {
    const prop = userTemplates.find((t) => t.id === templateId)?.properties.find((p) => p.id === fieldKey);
    return (prop?.sensitivityLevel as SensitivityLevel) || 'public';
  };

  const isFieldDeprecated = (templateId: string | undefined, fieldKey: string): boolean => {
    const prop = userTemplates.find((t) => t.id === templateId)?.properties.find((p) => p.id === fieldKey);
    return !!prop?.deprecatedAt;
  };

  const getFieldName = (templateId: string | undefined, fieldKey: string): string => {
    const prop = userTemplates.find((t) => t.id === templateId)?.properties.find((p) => p.id === fieldKey);
    return prop?.name || fieldKey;
  };

  useEffect(() => {
    if (accountId) {
      if (pageId) {
        loadObjects(accountId, { parentId: pageId });
      } else {
        loadObjects(accountId, sectionFilter ? { collectionType: sectionFilter } : undefined);
      }
    }
  }, [accountId, sectionFilter, pageId]);

  const visibleObjects = objects.filter(
    (obj) =>
      obj.collectionType !== 'page' &&
      obj.collectionType !== 'unknown' &&
      obj.name.toLowerCase().includes(searchQuery.toLowerCase())
  );

  // Load snapshot counts for visible objects
  useEffect(() => {
    const ids = visibleObjects.map(o => o.id);
    if (ids.length === 0) return;
    invoke<Record<string, number>>('snapshot_count_batch', { objectIds: ids })
      .then(setSnapshotCounts)
      .catch(() => {});
  }, [visibleObjects.length]);

  // Load attachment counts for visible objects
  const refreshAttachmentCounts = useCallback(() => {
    const ids = visibleObjects.map(o => o.id);
    if (ids.length === 0) return;
    invoke<Record<string, number>>('attachment_count_batch', { objectIds: ids })
      .then(setAttachmentCounts)
      .catch(() => {});
  }, [visibleObjects.length]);

  useEffect(() => { refreshAttachmentCounts(); }, [refreshAttachmentCounts]);


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

  return (
    <AppShell
      title={customPage?.name || activeCategoryLabel || t('objects')}
      onBack={() => navigate('/home')}
      actions={
        <div style={{ display: 'flex', gap: 8 }}>
          <button
            onClick={() => navigate(newObjectUrl)}
            style={{
              padding: '8px 16px', borderRadius: 8, border: 'none',
              background: 'var(--accent-primary)', color: 'white',
              fontSize: 13, fontWeight: 500, cursor: 'pointer',
            }}
          >
            + {t('create')}
          </button>
          {pageId && customPage && (
            <button
              onClick={() => setConfirmPageDelete(true)}
              title={t('delete')}
              style={{
                padding: '8px 12px', borderRadius: 8, border: '1px solid var(--border-subtle)',
                background: 'transparent', color: '#e74c3c', cursor: 'pointer',
                fontSize: 13, display: 'flex', alignItems: 'center', gap: 4,
              }}
            >
              <Trash size={14} /> {t('delete')} {customPage?.name || t('objects')}
            </button>
          )}
        </div>
      }
    >
      <div style={{ maxWidth: 640, margin: '0 auto', display: 'flex', flexDirection: 'column', gap: 16 }} onMouseDown={(e) => { if (e.detail > 1) e.preventDefault(); }}>
        <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap' }}>
          {CATEGORY_TYPES.map((catType) => (
            <button
              key={catType}
              onClick={() => navigate(`/workspace?section=${catType}`)}
              style={{
                padding: '6px 14px', borderRadius: 8, border: '1px solid var(--border-subtle)',
                background: !pageId && sectionFilter === catType ? 'var(--accent-primary)' : 'transparent',
                color: !pageId && sectionFilter === catType ? 'white' : 'var(--text-primary)',
                fontSize: 13, cursor: 'pointer', display: 'flex', alignItems: 'center', gap: 4,
              }}
            >
              {React.createElement(CATEGORY_ICONS[catType], { size: 16 })}
              {t(`navigation:${catType}`, catType)}
            </button>
          ))}
          {activeCustomPages.map((page) => (
            <button
              key={page.id}
              onClick={() => navigate(`/workspace/custom/${page.id}`)}
              style={{
                padding: '6px 14px', borderRadius: 8, border: '1px solid var(--border-subtle)',
                background: pageId === page.id ? 'var(--accent-primary)' : 'transparent',
                color: pageId === page.id ? 'white' : 'var(--text-primary)',
                fontSize: 13, cursor: 'pointer', display: 'flex', alignItems: 'center', gap: 4,
              }}
            >
              <FileText size={16} />
              {page.name}
            </button>
          ))}
          {(sectionFilter || pageId) && (
            <button
              onClick={() => navigate('/workspace')}
              style={{
                padding: '6px 14px', borderRadius: 8, border: '1px solid var(--border-subtle)',
                background: 'transparent', color: 'var(--text-tertiary)',
                fontSize: 13, cursor: 'pointer',
              }}
            >
              {t('clear')}
            </button>
          )}
        </div>

        <Input
          placeholder={t('search_objects_placeholder')}
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
        />

        {isLoading && (
          <Card>
            <p style={{ textAlign: 'center', color: 'var(--text-tertiary)', padding: '24px 0' }}>
              {t('loading')}
            </p>
          </Card>
        )}
        {!isLoading && error && (
          <Card>
            <p style={{ textAlign: 'center', color: '#e74c3c', padding: '24px 0' }}>{error}</p>
          </Card>
        )}
        {!isLoading && !error && visibleObjects.length === 0 && (
          <Card>
            <p style={{ textAlign: 'center', color: 'var(--text-secondary)', padding: '24px 0', fontSize: 14 }}>
              {searchQuery ? t('no_matching_objects') : t('no_objects')}
            </p>
          </Card>
        )}
        {!isLoading &&
          visibleObjects.map((obj) => {
            const tpl = userTemplates.find((t) => t.id === obj.templateId);
            const fieldOrder = tpl?.properties.map((p) => p.id);
            const fields = flattenProperties(obj.properties as Record<string, unknown> | undefined, fieldOrder);
            return (
              <Card key={obj.id} interactive onClick={() => setDetailObj(obj)}>
                {/* Header row */}
                <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: fields.length > 0 ? 8 : 0 }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
                    <PAGE_ICON_MAP.custom size={22} />
                    <div>
                      <span style={{ fontSize: 14, fontWeight: 600 }}>{obj.name}</span>
                      <span style={{
                        fontSize: 10, color: 'var(--text-tertiary)', marginLeft: 8,
                        padding: '1px 5px', borderRadius: 4, background: 'var(--bg-elevated)',
                      }}>
                        {resolveCollectionLabel(obj.collectionType)}
                      </span>
                    </div>
                  </div>
                  {/* Edit + Delete + History actions */}
                  <div style={{ display: 'flex', gap: 2 }} onClick={(e) => e.stopPropagation()}>
                    <div style={{ position: 'relative' }}>
                      <button
                        onClick={() => setHistoryObj({ id: obj.id, collectionType: obj.collectionType, templateId: obj.templateId || undefined })}
                        title="History"
                        style={{
                          width: 32, height: 32, display: 'flex', alignItems: 'center', justifyContent: 'center',
                          border: 'none', borderRadius: 8, background: 'transparent', cursor: 'pointer',
                          color: 'var(--text-tertiary)', transition: 'all 0.15s ease',
                        }}
                        onMouseEnter={(e) => { e.currentTarget.style.background = 'rgba(128,128,128,0.08)'; e.currentTarget.style.color = 'var(--text-primary)'; }}
                        onMouseLeave={(e) => { e.currentTarget.style.background = 'transparent'; e.currentTarget.style.color = 'var(--text-tertiary)'; }}
                      >
                        <Clock size={14} />
                      </button>
                      {/* Badge count */}
                      {snapshotCounts[obj.id] !== undefined && snapshotCounts[obj.id] > 0 && (
                        <span style={{
                          position: 'absolute', top: -2, right: -2, minWidth: 14, height: 14,
                          display: 'flex', alignItems: 'center', justifyContent: 'center',
                          background: 'var(--accent-primary)', color: 'white', fontSize: 9, fontWeight: 700,
                          borderRadius: 7, padding: '0 3px', lineHeight: 1,
                        }}>
                          {snapshotCounts[obj.id]}
                        </span>
                      )}
                    </div>
                    {/* Attachment button */}
                    <div style={{ position: 'relative' }}>
                      <button
                        onClick={() => setAttachmentObjId(obj.id)}
                        title="Attachments"
                        style={{
                          width: 32, height: 32, display: 'flex', alignItems: 'center', justifyContent: 'center',
                          border: 'none', borderRadius: 8, background: 'transparent', cursor: 'pointer',
                          color: 'var(--text-tertiary)', transition: 'all 0.15s ease',
                        }}
                        onMouseEnter={(e) => { e.currentTarget.style.background = 'rgba(128,128,128,0.08)'; e.currentTarget.style.color = 'var(--text-primary)'; }}
                        onMouseLeave={(e) => { e.currentTarget.style.background = 'transparent'; e.currentTarget.style.color = 'var(--text-tertiary)'; }}
                      >
                        <Paperclip size={14} />
                      </button>
                      {attachmentCounts[obj.id] !== undefined && attachmentCounts[obj.id] > 0 && (
                        <span style={{
                          position: 'absolute', top: -2, right: -2, minWidth: 14, height: 14,
                          display: 'flex', alignItems: 'center', justifyContent: 'center',
                          background: 'var(--accent-primary)', color: 'white', fontSize: 9, fontWeight: 700,
                          borderRadius: 7, padding: '0 3px', lineHeight: 1,
                        }}>
                          {attachmentCounts[obj.id]}
                        </span>
                      )}
                    </div>
                    <button
                      onClick={() => navigate(`/editor/${obj.id}`)}
                      title="Edit"
                      style={{
                        width: 32, height: 32, display: 'flex', alignItems: 'center', justifyContent: 'center',
                        border: 'none', borderRadius: 8, background: 'transparent', cursor: 'pointer',
                        color: 'var(--text-tertiary)', transition: 'all 0.15s ease',
                      }}
                      onMouseEnter={(e) => {
                        e.currentTarget.style.background = 'rgba(128,128,128,0.08)';
                        e.currentTarget.style.color = 'var(--text-primary)';
                      }}
                      onMouseLeave={(e) => {
                        e.currentTarget.style.background = 'transparent';
                        e.currentTarget.style.color = 'var(--text-tertiary)';
                      }}
                    >
                      <Pencil size={14} />
                    </button>
                    <button
                      onClick={() => setConfirmDelete({ id: obj.id, name: obj.name })}
                      title="Move to trash"
                      style={{
                        width: 32, height: 32, display: 'flex', alignItems: 'center', justifyContent: 'center',
                        border: 'none', borderRadius: 8, background: 'transparent', cursor: 'pointer',
                        color: 'var(--text-tertiary)', transition: 'all 0.15s ease',
                      }}
                      onMouseEnter={(e) => {
                        e.currentTarget.style.background = 'rgba(231,76,60,0.1)';
                        e.currentTarget.style.color = '#e74c3c';
                      }}
                      onMouseLeave={(e) => {
                        e.currentTarget.style.background = 'transparent';
                        e.currentTarget.style.color = 'var(--text-tertiary)';
                      }}
                    >
                      <Trash2 size={14} />
                    </button>
                  </div>
                </div>
                {/* Property chips — label always visible, value blurred when masked */}
                {fields.length > 0 && (
                  <div style={{ display: 'flex', gap: 6, flexWrap: 'wrap' }}>
                    {fields.map((f) => {
                      const sens = getFieldSensitivity(obj.templateId, f.key);
                      const deprecated = isFieldDeprecated(obj.templateId, f.key);
                      const isMasked = sens !== 'public';
                      const fieldLabel = getFieldName(obj.templateId, f.key);
                      const s = getSensitivityStyle(sens);
                      return (
                      <span
                        key={f.key}
                        style={{
                          padding: '3px 8px', borderRadius: 6, fontSize: 11,
                          background: 'var(--bg-toolbar)', color: 'var(--text-secondary)',
                          border: `1px solid ${isMasked ? s.fg : s.fg}`,
                          maxWidth: 220, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap',
                          opacity: deprecated ? 0.6 : 1,
                          ...(isMasked ? {
                            boxShadow: `0 0 3px ${s.fg}44`,
                          } : {
                            boxShadow: `0 0 2px ${s.fg}33`,
                          }),
                        }}
                      >
                        <span style={{ fontWeight: 600, textDecoration: deprecated ? 'line-through' : 'none' }}>{fieldLabel}</span>
                        <span style={{ margin: '0 3px' }}>:</span>
                        <span style={{
                          ...(isMasked ? {
                            filter: 'blur(5px)',
                            cursor: 'default',
                            userSelect: 'none',
                            background: 'var(--bg-subtle, rgba(128,128,128,0.12))',
                            borderRadius: 2,
                            padding: '0 2px',
                          } : { color: 'var(--text-primary)' }),
                        }}>
                          {isMasked ? '••••' : f.value}
                        </span>
                      </span>
                      );
                    })}
                  </div>
                )}
                {/* Tag pills */}
                {obj.tags && obj.tags.length > 0 && (
                  <div style={{ display: 'flex', gap: 4, flexWrap: 'wrap', marginTop: 6 }}>
                    {obj.tags.map((tag) => (
                      <span key={tag} style={{
                        padding: '1px 7px', borderRadius: 10, fontSize: 10,
                        background: 'rgba(91,124,153,0.08)', color: 'var(--accent-primary)',
                        fontWeight: 500,
                      }}>
                        {tag}
                      </span>
                    ))}
                  </div>
                )}
              </Card>
            );
          })}

        {/* Page delete confirmation dialog */}
        {confirmPageDelete && pageId && customPage && (
          <div
            style={{
              position: 'fixed', inset: 0, zIndex: 1000,
              display: 'flex', alignItems: 'center', justifyContent: 'center',
              background: 'rgba(0,0,0,0.4)', backdropFilter: 'blur(4px)',
            }}
            onClick={() => setConfirmPageDelete(false)}
          >
            <div
              style={{
                background: 'var(--bg-elevated)', borderRadius: 12, padding: '24px 28px',
                maxWidth: 360, width: '90%', boxShadow: 'var(--shadow-lg)',
                border: '1px solid var(--border-subtle)',
              }}
              onClick={(e) => e.stopPropagation()}
            >
              <h3 style={{ margin: '0 0 8px', fontSize: 16, fontWeight: 600 }}>{t('object_delete_confirm_title')}</h3>
              <p style={{ margin: '0 0 20px', fontSize: 14, color: 'var(--text-secondary)', lineHeight: 1.5 }}>
                {t('object_delete_confirm_body', { name: customPage.name })}
              </p>
              <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end' }}>
                <Button variant="secondary" onClick={() => setConfirmPageDelete(false)}>{t('cancel')}</Button>
                <button
                  onClick={async () => {
                    setConfirmPageDelete(false);
                    if (accountId) {
                      await removeCustomPage(accountId, pageId);
                      navigate('/');
                    }
                  }}
                  style={{
                    padding: '8px 16px', borderRadius: 8, border: 'none',
                    background: '#e74c3c', color: 'white',
                    fontSize: 13, fontWeight: 500, cursor: 'pointer',
                  }}
                >
                  {t('delete')}
                </button>
              </div>
            </div>
          </div>
        )}

        {/* Delete confirmation dialog */}
        {/* Object detail modal */}
        {detailObj && (
          <ObjectDetailModal
            object={detailObj}
            onClose={() => setDetailObj(null)}
            onHistory={() => {
              setHistoryObj({ id: detailObj.id, collectionType: detailObj.collectionType, templateId: detailObj.templateId || undefined });
              setDetailObj(null);
            }}
            onAttachments={() => {
              setAttachmentObjId(detailObj.id);
              setDetailObj(null);
            }}
            onEdit={() => {
              navigate(`/editor/${detailObj.id}`);
              setDetailObj(null);
            }}
            onDelete={() => {
              setConfirmDelete({ id: detailObj.id, name: detailObj.name });
              setDetailObj(null);
            }}
          />
        )}

        {confirmDelete && (
          <div
            style={{
              position: 'fixed', inset: 0, zIndex: 1000,
              display: 'flex', alignItems: 'center', justifyContent: 'center',
              background: 'rgba(0,0,0,0.4)', backdropFilter: 'blur(4px)',
            }}
            onClick={() => setConfirmDelete(null)}
          >
            <div
              style={{
                background: 'var(--bg-elevated)', borderRadius: 12, padding: '24px 28px',
                maxWidth: 360, width: '90%', boxShadow: 'var(--shadow-lg)',
                border: '1px solid var(--border-subtle)',
              }}
              onClick={(e) => e.stopPropagation()}
            >
              <h3 style={{ margin: '0 0 8px', fontSize: 16, fontWeight: 600 }}>{t('object_delete_confirm_title')}</h3>
              <p style={{ margin: '0 0 20px', fontSize: 14, color: 'var(--text-secondary)', lineHeight: 1.5 }}>
                {t('object_delete_confirm_body', { name: confirmDelete.name })}
              </p>
              <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end' }}>
                <Button variant="secondary" onClick={() => setConfirmDelete(null)}>{t('cancel')}</Button>
                <button
                  onClick={() => handleDelete(confirmDelete.id)}
                  style={{
                    padding: '8px 16px', borderRadius: 8, border: 'none',
                    background: '#e74c3c', color: 'white',
                    fontSize: 13, fontWeight: 500, cursor: 'pointer',
                  }}
                >
                  {t('delete')}
                </button>
              </div>
            </div>
          </div>
        )}
      </div>
      {historyObj && (
        <HistoryViewer
          objectId={historyObj.id}
          onClose={() => setHistoryObj(null)}
          passwordVerify={passwordVerify}
          getFieldSensitivity={(fieldKey) => getFieldSensitivity(historyObj.templateId, fieldKey)}
          isFieldDeprecated={(fieldKey) => isFieldDeprecated(historyObj.templateId, fieldKey)}
          getFieldName={(fieldKey) => getFieldName(historyObj.templateId, fieldKey)}
          fieldOrder={userTemplates.find((t) => t.id === historyObj.templateId)?.properties.map((p) => p.id)}
        />
      )}
      {attachmentObjId && <AttachmentViewer objectId={attachmentObjId} onClose={() => setAttachmentObjId(null)} onCountChange={refreshAttachmentCounts} />}

      {/* Unified password verification dialog (detail panel + history cards) */}
      <PasswordVerificationDialog
        open={showPwDialog}
        onClose={() => { setShowPwDialog(false); pwResolveRef.current?.(false); }}
        onVerify={async (password) => {
          const ok = await verifyVaultPassword(password);
          if (ok) pwResolveRef.current?.(true);
          return ok;
        }}
        title={t('common:critical_access_title')}
        description={t('common:critical_access_desc')}
        confirmLabel={t('common:unlock')}
        hint={passwordHint}
        biometricType={bioAvailable.available ? bioAvailable.biometryType : undefined}
        onBiometric={bioAvailable.available ? handleBiometricUnlock : undefined}
      />
    </AppShell>
  );
}
