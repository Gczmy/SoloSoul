import { useState, useEffect, useCallback, useRef, useMemo } from 'react';
import { useNavigate, useSearchParams, useParams } from 'react-router-dom';
import React from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { useCancellable } from '@/hooks/useCancellable';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { Input } from '@/components/ui/Input';
import { Button } from '@/components/ui/Button';
import { LoadingPlaceholder } from '@/components/ui/LoadingPlaceholder';
import { useAuthStore } from '@/stores/authStore';
import { useObjectStore } from '@/stores/objectStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { useTemplateStore } from '@/stores/templateStore';
import { type TemplateProperty } from '@/types/template';
import { type SensitivityLevel } from '@/components/ui/SensitivityBadge';
import { HistoryViewer } from '@/components/object/HistoryViewer';
import { AttachmentViewer } from '@/components/object/AttachmentViewer';
import { Trash } from 'lucide-react';
import { PasswordVerificationDialog } from '@/components/forms/PasswordVerificationDialog';
import { ObjectDetailModal } from '@/components/object/ObjectDetailModal';
import { PAGE_ICON_MAP, resolveCustomIcon } from '@/lib/pageIcons';
import { WorkspaceObjectCard } from './WorkspaceObjectCard';

// Labels resolved at render time via t() so they support i18n
const CATEGORY_TYPES = ['identity', 'travel', 'financial', 'professional'] as const;
const CATEGORY_ICONS: Record<string, typeof PAGE_ICON_MAP.profile> = {
  identity: PAGE_ICON_MAP.profile,
  travel: PAGE_ICON_MAP.travel,
  financial: PAGE_ICON_MAP.financial,
  professional: PAGE_ICON_MAP.professional,
};

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
  const [historyObj, setHistoryObj] = useState<{
    id: string;
    name: string;
    collectionType: string;
    templateId?: string;
  } | null>(null);
  const [snapshotCounts, setSnapshotCounts] = useState<Record<string, number>>({});
  const [attachmentObjId, setAttachmentObjId] = useState<string | null>(null);
  const [attachmentCounts, setAttachmentCounts] = useState<Record<string, number>>({});
  const [detailObj, setDetailObj] = useState<(typeof visibleObjects)[number] | null>(null);

  const accountId = useAuthStore((s) => s.currentAccount?.id);
  const { t } = useTranslation(['common', 'navigation', 'editor']);
  const { objects, loadObjects, deleteObject, isLoading, error } = useObjectStore();
  const customPages = useSettingsStore((s) => s.settings.customPages);
  const activeCustomPages = customPages.filter((p) => !p.deletedAt);
  const removeCustomPage = useSettingsStore((s) => s.removeCustomPage);
  const { templates: userTemplates, loadTemplates: loadUserTemplates } = useTemplateStore();
  const makeCancellable = useCancellable();

  useEffect(() => {
    loadUserTemplates().catch(() => {});
  }, [loadUserTemplates]);

  // Open object detail modal directly when navigated with ?objectId=... (e.g. from search)
  useEffect(() => {
    if (!detailObjectId || !accountId) return;
    invoke('object_get', { objectId: detailObjectId })
      .then((obj) => setDetailObj(obj as (typeof visibleObjects)[number]))
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

  const activeCategoryLabel = sectionFilter
    ? t(`navigation:${sectionFilter}`, sectionFilter)
    : null;

  /** Password dialog state — shared between detail panel and history viewer. */
  const [showPwDialog, setShowPwDialog] = useState(false);
  const pwResolveRef = useRef<
    ((result: { ok: boolean; method: 'password' | 'touchId' | 'faceId' }) => void) | null
  >(null);
  const [bioAvailable, setBioAvailable] = useState<{ available: boolean; biometryType?: string }>({
    available: false,
  });
  const [passwordHint, setPasswordHint] = useState<string | null>(null);

  // Check biometric availability on mount + load password hint
  useEffect(() => {
    invoke<{ available: boolean; configured: boolean; biometryType?: string }>(
      'biometric_check_availability',
      { accountId: accountId || '' },
    )
      .then((r) =>
        setBioAvailable({ available: r.available && r.configured, biometryType: r.biometryType }),
      )
      .catch(() => {});
    if (accountId) {
      invoke<Array<{ id: string; passwordHint?: string }>>('vault_list_accounts')
        .then((accounts) => {
          const acc = accounts.find((a) => a.id === accountId);
          setPasswordHint(acc?.passwordHint || null);
        })
        .catch(() => {
          /* ignore */
        });
    }
  }, [accountId]);

  const passwordVerify = useCallback(async (): Promise<{
    ok: boolean;
    method: 'password' | 'touchId' | 'faceId';
  }> => {
    return new Promise((resolve) => {
      pwResolveRef.current = resolve;
      setShowPwDialog(true);
    });
  }, []);

  /** Unlock the current account with the master password — used by PasswordVerificationDialog
   *  before revealing critical fields. This ensures the vault is open so the subsequent
   *  critical-field audit log can be written. */
  // Hover handlers for workspace tab buttons
  const onTabEnter = useCallback((e: React.MouseEvent<HTMLButtonElement>) => {
    if (e.currentTarget.dataset.active === 'true') return;
    e.currentTarget.style.background = 'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
    e.currentTarget.style.borderColor = 'var(--accent-primary)';
  }, []);
  const onTabLeave = useCallback((e: React.MouseEvent<HTMLButtonElement>) => {
    if (e.currentTarget.dataset.active === 'true') return;
    e.currentTarget.style.background = 'var(--bg-toolbar)';
    e.currentTarget.style.borderColor = 'var(--border-subtle)';
  }, []);
  const onClearEnter = useCallback((e: React.MouseEvent<HTMLButtonElement>) => {
    e.currentTarget.style.borderColor = 'var(--accent-primary)';
    e.currentTarget.style.color = 'var(--text-primary)';
    e.currentTarget.style.boxShadow = '0 0 0 2px color-mix(in srgb, var(--accent-primary) 10%, transparent)';
  }, []);
  const onClearLeave = useCallback((e: React.MouseEvent<HTMLButtonElement>) => {
    e.currentTarget.style.borderColor = 'var(--border-subtle)';
    e.currentTarget.style.color = 'var(--text-tertiary)';
    e.currentTarget.style.boxShadow = 'none';
  }, []);

  const verifyVaultPassword = useCallback(
    async (password: string): Promise<boolean> => {
      if (!accountId) return false;
      try {
        await invoke('unlock_with_password', { accountId, password });
        return true;
      } catch {
        return false;
      }
    },
    [accountId],
  );

  /** Biometric unlock handler — used by PasswordVerificationDialog. */
  const handleBiometricUnlock = useCallback(async (): Promise<boolean> => {
    if (!accountId) return false;
    try {
      await invoke('biometric_unlock', {
        accountId,
        location: 'critical_data_access',
        action: 'unlock',
        biometryType: bioAvailable.biometryType,
      });
      const method = (bioAvailable.biometryType as 'touchId' | 'faceId') || 'touchId';
      pwResolveRef.current?.({ ok: true, method });
      return true;
    } catch {
      return false;
    }
  }, [accountId, bioAvailable.biometryType]);

  // F011: cache template field metadata so lookups are O(1) instead of O(n²).
  const templateFieldMap = useMemo(() => {
    const map = new Map<string, Map<string, TemplateProperty>>();
    for (const t of userTemplates) {
      map.set(t.id, new Map(t.properties.map((p) => [p.id, p])));
    }
    return map;
  }, [userTemplates]);

  const getFieldProperty = (
    templateId: string | undefined,
    fieldKey: string,
  ): TemplateProperty | undefined => {
    return templateFieldMap.get(templateId || '')?.get(fieldKey);
  };

  const getFieldSensitivity = (
    templateId: string | undefined,
    fieldKey: string,
  ): SensitivityLevel => {
    return (getFieldProperty(templateId, fieldKey)?.sensitivityLevel as SensitivityLevel) || 'public';
  };

  const isFieldDeprecated = (templateId: string | undefined, fieldKey: string): boolean => {
    return !!getFieldProperty(templateId, fieldKey)?.deprecatedAt;
  };

  const getFieldName = (templateId: string | undefined, fieldKey: string): string => {
    return getFieldProperty(templateId, fieldKey)?.name || fieldKey;
  };

  useEffect(() => {
    if (accountId) {
      if (pageId) {
        loadObjects(accountId, { parentId: pageId });
      } else {
        loadObjects(accountId, sectionFilter ? { collectionType: sectionFilter } : undefined);
      }
    }
  }, [accountId, sectionFilter, pageId, loadObjects]);

  const visibleObjects = useMemo(
    () =>
      objects.filter(
        (obj) =>
          obj.collectionType !== 'page' &&
          obj.collectionType !== 'unknown' &&
          obj.name.toLowerCase().includes(searchQuery.toLowerCase()),
      ),
    [objects, searchQuery],
  );

  const snapshotReqRef = useRef(0);

  // Load snapshot counts for visible objects
  /* eslint-disable react-hooks/exhaustive-deps */
  useEffect(() => {
    const ids = visibleObjects.map((o) => o.id);
    if (ids.length === 0) return;
    const reqId = ++snapshotReqRef.current;
    // Initialize with 0 immediately so badges render without waiting for the invoke
    const initial: Record<string, number> = {};
    for (const id of ids) initial[id] = 0;
    setSnapshotCounts(initial);

    invoke<Record<string, number>>('snapshot_count_batch', { objectIds: ids })
      .then((counts) => {
        if (snapshotReqRef.current !== reqId) return; // stale response, discard
        // Ensure every visible object has a snapshot count (default 0)
        const full: Record<string, number> = {};
        for (const id of ids) full[id] = counts[id] ?? 0;
        setSnapshotCounts(full);
      })
      .catch((e) => {
        if (snapshotReqRef.current !== reqId) return; // stale error, discard
        // eslint-disable-next-line no-console
        console.error('snapshot_count_batch failed:', e);
      });
    // Increment ref on cleanup so in-flight responses become stale (handles Strict Mode + unmount)
    return () => { snapshotReqRef.current++; };
  /* eslint-enable react-hooks/exhaustive-deps */
  }, [visibleObjects]);

  // Load attachment counts for visible objects
  const refreshAttachmentCounts = useCallback(() => {
    const { isCancelled, cancel } = makeCancellable();
    const ids = visibleObjects.map((o) => o.id);
    if (ids.length === 0) {
      cancel();
      return cancel;
    }
    invoke<Record<string, number>>('attachment_count_batch', { objectIds: ids })
      .then((counts) => {
        if (!isCancelled()) setAttachmentCounts(counts);
      })
      .catch(() => {});
    return cancel;
  }, [visibleObjects, makeCancellable]);

  useEffect(() => {
    return refreshAttachmentCounts();
  }, [refreshAttachmentCounts]);

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
            onMouseEnter={(e) => {
              e.currentTarget.style.background = 'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
              e.currentTarget.style.borderColor = 'var(--accent-primary)';
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.background = 'var(--bg-toolbar)';
              e.currentTarget.style.borderColor = 'var(--border-subtle)';
            }}
            style={{
              padding: '8px 16px',
              borderRadius: 8,
              border: '1px solid var(--border-subtle)',
              background: 'var(--bg-toolbar)',
              color: 'var(--text-primary)',
              fontSize: 13,
              fontWeight: 500,
              cursor: 'pointer',
              transition: 'background 0.2s, border-color 0.2s',
            }}
          >
            + {t('create')}
          </button>
          {pageId && customPage && (
            <button
              onClick={() => setConfirmPageDelete(true)}
              title={t('delete')}
              onMouseEnter={(e) => {
                e.currentTarget.style.borderColor = '#e74c3c';
                e.currentTarget.style.background = 'color-mix(in srgb, #e74c3c 10%, transparent)';
                e.currentTarget.style.boxShadow = '0 0 0 2px color-mix(in srgb, #e74c3c 15%, transparent)';
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.borderColor = 'var(--border-subtle)';
                e.currentTarget.style.background = 'transparent';
                e.currentTarget.style.boxShadow = 'none';
              }}
              style={{
                padding: '8px 12px',
                borderRadius: 8,
                border: '1px solid var(--border-subtle)',
                background: 'transparent',
                color: '#e74c3c',
                cursor: 'pointer',
                fontSize: 13,
                display: 'flex',
                alignItems: 'center',
                gap: 4,
                transition: 'border-color 0.2s, box-shadow 0.2s, background 0.2s',
              }}
            >
              <Trash size={14} /> {t('delete')}
            </button>
          )}
        </div>
      }
    >
      <div
        style={{
          maxWidth: 640,
          margin: '0 auto',
          display: 'flex',
          flexDirection: 'column',
          gap: 16,
        }}
        onMouseDown={(e) => {
          if (e.detail > 1) e.preventDefault();
        }}
      >
        <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap' }}>
          {CATEGORY_TYPES.map((catType) => {
            const isActive = !pageId && sectionFilter === catType;
            return (
              <button
                key={catType}
                data-active={isActive ? 'true' : 'false'}
                onClick={() => navigate(`/workspace?section=${catType}`)}
                onMouseEnter={onTabEnter}
                onMouseLeave={onTabLeave}                  style={{
                      padding: '6px 14px',
                      borderRadius: 8,
                      border: isActive ? '1px solid var(--accent-primary)' : '1px solid var(--border-subtle)',
                      background: isActive ? 'color-mix(in srgb, var(--accent-primary) 10%, transparent)' : 'var(--bg-toolbar)',
                      color: isActive ? 'var(--accent-primary)' : 'var(--text-primary)',
                      boxShadow: isActive ? '0 0 0 1px var(--accent-primary)' : 'none',
                      fontSize: 13,
                      cursor: 'pointer',
                      display: 'flex',
                      alignItems: 'center',
                      gap: 4,
                      transition: 'background 0.2s, border-color 0.2s, color 0.2s, box-shadow 0.2s',
                    }}
                  >
                    {React.createElement(CATEGORY_ICONS[catType], { size: 16 })}
                    {t(`navigation:${catType}`, catType)}
                  </button>
                );
              })}
              {activeCustomPages.map((page) => {
            const isActive = pageId === page.id;
            return (
              <button
                key={page.id}
                data-active={isActive ? 'true' : 'false'}
                onClick={() => navigate(`/workspace/custom/${page.id}`)}
                onMouseEnter={onTabEnter}
                onMouseLeave={onTabLeave}
                style={{
                  padding: '6px 14px',
                  borderRadius: 8,
                  border: isActive ? '1px solid var(--accent-primary)' : '1px solid var(--border-subtle)',
                  background: isActive ? 'color-mix(in srgb, var(--accent-primary) 10%, transparent)' : 'var(--bg-toolbar)',
                  color: isActive ? 'var(--accent-primary)' : 'var(--text-primary)',
                  boxShadow: isActive ? '0 0 0 1px var(--accent-primary)' : 'none',
                  fontSize: 13,
                  cursor: 'pointer',
                  display: 'flex',
                  alignItems: 'center',
                  gap: 4,
                  transition: 'background 0.2s, border-color 0.2s, color 0.2s, box-shadow 0.2s',
                }}
              >
                {React.createElement(resolveCustomIcon(page.iconId), { size: 16 })}
                {page.name}
              </button>
            );
          })}
          {(sectionFilter || pageId) && (
            <button
              onClick={() => navigate('/workspace')}
              onMouseEnter={onClearEnter}
              onMouseLeave={onClearLeave}
              style={{
                padding: '6px 14px',
                borderRadius: 8,
                border: '1px solid var(--border-subtle)',
                background: 'transparent',
                color: 'var(--text-tertiary)',
                fontSize: 13,
                cursor: 'pointer',
                transition: 'border-color 0.2s, box-shadow 0.2s, color 0.2s',
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
          onClear={() => setSearchQuery('')}
        />

        {isLoading && (
          <Card>
            <LoadingPlaceholder variant="elevated" minHeight={80} />
          </Card>
        )}
        {!isLoading && error && (
          <Card>
            <p style={{ textAlign: 'center', color: '#e74c3c', padding: '24px 0' }}>{error}</p>
          </Card>
        )}
        {!isLoading && !error && visibleObjects.length === 0 && (
          <Card>
            <p
              style={{
                textAlign: 'center',
                color: 'var(--text-secondary)',
                padding: '24px 0',
                fontSize: 14,
              }}
            >
              {searchQuery ? t('no_matching_objects') : t('no_objects')}
            </p>
          </Card>
        )}
        {!isLoading &&
          visibleObjects.map((obj) => (
            <WorkspaceObjectCard
              key={obj.id}
              obj={obj}
              collectionLabel={resolveCollectionLabel(obj.collectionType)}
              userTemplates={userTemplates}
              snapshotCount={snapshotCounts[obj.id]}
              attachmentCount={attachmentCounts[obj.id]}
              onClick={() => setDetailObj(obj)}
              onHistory={() =>
                setHistoryObj({
                  id: obj.id,
                  name: obj.name,
                  collectionType: obj.collectionType,
                  templateId: obj.templateId || undefined,
                })
              }
              onAttachments={() => setAttachmentObjId(obj.id)}
              onEdit={() => navigate(`/editor/${obj.id}`)}
              onDelete={() => setConfirmDelete({ id: obj.id, name: obj.name })}
            />
          ))}

        {/* Page delete confirmation dialog */}
        {confirmPageDelete && pageId && customPage && (
          <div
            style={{
              position: 'fixed',
              inset: 0,
              zIndex: 1000,
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              background: 'rgba(0,0,0,0.4)',
              backdropFilter: 'blur(4px)',
            }}
            onClick={() => setConfirmPageDelete(false)}
          >
            <div
              style={{
                background: 'var(--bg-elevated)',
                borderRadius: 12,
                padding: '24px 28px',
                maxWidth: 360,
                width: '90%',
                boxShadow: 'var(--shadow-lg)',
                border: '1px solid var(--border-subtle)',
              }}
              onClick={(e) => e.stopPropagation()}
            >
              <h3 style={{ margin: '0 0 8px', fontSize: 16, fontWeight: 600 }}>
                {t('object_delete_confirm_title')}
              </h3>
              <p
                style={{
                  margin: '0 0 20px',
                  fontSize: 14,
                  color: 'var(--text-secondary)',
                  lineHeight: 1.5,
                }}
              >
                {t('object_delete_confirm_body', { name: customPage.name.length > 28 ? customPage.name.slice(0, 27) + '…' : customPage.name })}
              </p>
              <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end' }}>
                <Button variant="secondary" onClick={() => setConfirmPageDelete(false)}>
                  {t('cancel')}
                </Button>
                <button
                  onClick={async () => {
                    setConfirmPageDelete(false);
                    if (accountId) {
                      await removeCustomPage(accountId, pageId);
                      navigate('/');
                    }
                  }}
                  style={{
                    padding: '8px 16px',
                    borderRadius: 8,
                    border: 'none',
                    background: '#e74c3c',
                    color: 'white',
                    fontSize: 13,
                    fontWeight: 500,
                    cursor: 'pointer',
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
              position: 'fixed',
              inset: 0,
              zIndex: 1000,
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              background: 'rgba(0,0,0,0.4)',
              backdropFilter: 'blur(4px)',
            }}
            onClick={() => setConfirmDelete(null)}
          >
            <div
              style={{
                background: 'var(--bg-elevated)',
                borderRadius: 12,
                padding: '24px 28px',
                maxWidth: 360,
                width: '90%',
                boxShadow: 'var(--shadow-lg)',
                border: '1px solid var(--border-subtle)',
              }}
              onClick={(e) => e.stopPropagation()}
            >
              <h3 style={{ margin: '0 0 8px', fontSize: 16, fontWeight: 600 }}>
                {t('object_delete_confirm_title')}
              </h3>
              <p
                style={{
                  margin: '0 0 20px',
                  fontSize: 14,
                  color: 'var(--text-secondary)',
                  lineHeight: 1.5,
                }}
              >
                {t('object_delete_confirm_body', { name: confirmDelete.name.length > 28 ? confirmDelete.name.slice(0, 27) + '…' : confirmDelete.name })}
              </p>
              <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end' }}>
                <Button variant="secondary" onClick={() => setConfirmDelete(null)}>
                  {t('cancel')}
                </Button>
                <button
                  onClick={() => handleDelete(confirmDelete.id)}
                  style={{
                    padding: '8px 16px',
                    borderRadius: 8,
                    border: 'none',
                    background: '#e74c3c',
                    color: 'white',
                    fontSize: 13,
                    fontWeight: 500,
                    cursor: 'pointer',
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
          objectName={historyObj.name}
          collectionType={historyObj.collectionType}
          onClose={() => setHistoryObj(null)}
          passwordVerify={passwordVerify}
          getFieldSensitivity={(fieldKey) => getFieldSensitivity(historyObj.templateId, fieldKey)}
          isFieldDeprecated={(fieldKey) => isFieldDeprecated(historyObj.templateId, fieldKey)}
          getFieldName={(fieldKey) => getFieldName(historyObj.templateId, fieldKey)}
          fieldOrder={userTemplates
            .find((t) => t.id === historyObj.templateId)
            ?.properties.map((p) => p.id)}
        />
      )}
      {attachmentObjId && (
        <AttachmentViewer
          objectId={attachmentObjId}
          onClose={() => setAttachmentObjId(null)}
          onCountChange={refreshAttachmentCounts}
        />
      )}

      {/* Unified password verification dialog (detail panel + history cards) */}
      <PasswordVerificationDialog
        open={showPwDialog}
        onClose={() => {
          setShowPwDialog(false);
          pwResolveRef.current?.({ ok: false, method: 'password' });
        }}
        onVerify={async (password) => {
          const ok = await verifyVaultPassword(password);
          if (ok) pwResolveRef.current?.({ ok: true, method: 'password' });
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
