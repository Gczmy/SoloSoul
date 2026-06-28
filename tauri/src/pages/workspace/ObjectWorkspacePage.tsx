import { useState, useEffect, useCallback, useRef, useMemo } from 'react';
import { useNavigate, useSearchParams, useParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { useCancellable } from '@/hooks/useCancellable';
import { AppShell } from '@/components/layout/AppShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { Card } from '@/components/ui/Card';
import { Input } from '@/components/ui/Input';
import { LoadingPlaceholder } from '@/components/ui/LoadingPlaceholder';
import { Button } from '@/components/ui/Button';
import { useAuthStore } from '@/stores/authStore';
import { useObjectStore } from '@/stores/objectStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { useTemplateStore } from '@/stores/templateStore';
import type { TemplateProperty } from '@/types/template';
import type { SensitivityLevel } from '@/components/ui/SensitivityBadge';

// Labels resolved at render time via t() so they support i18n
import { DEBOUNCE_DELAY_MS } from '@/lib/constants';
import { HistoryViewer } from '@/components/object/HistoryViewer';
import { AttachmentViewer } from '@/components/object/AttachmentViewer';
import { Trash, Search } from 'lucide-react';
import { PasswordVerificationDialog } from '@/components/forms/PasswordVerificationDialog';
import { ObjectDetailModal } from '@/components/object/ObjectDetailModal';

import { WorkspaceObjectCard } from './WorkspaceObjectCard';
import { WorkspaceCategoryTabs } from '@/components/workspace/WorkspaceCategoryTabs';
import { ConfirmDeleteDialog } from '@/components/workspace/ConfirmDeleteDialog';
import { useWorkspacePasswordGuard } from '@/hooks/useWorkspacePasswordGuard';
import { ICON_SIZE } from '@/lib/iconSizes';


export function ObjectWorkspacePage() {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const { pageId } = useParams();
  const sectionFilter = searchParams.get('section') || '';
  const detailObjectId = searchParams.get('objectId');
  const [searchQuery, setSearchQuery] = useState('');
  const [debouncedSearchQuery, setDebouncedSearchQuery] = useState('');
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
    loadUserTemplates().catch((err) => console.warn('[Workspace] Load templates failed:', err));
  }, [loadUserTemplates]);

  // Open object detail modal directly when navigated with ?objectId=... (e.g. from search)
  useEffect(() => {
    if (!detailObjectId || !accountId) return;
    invoke('object_get', { objectId: detailObjectId })
      .then((obj) => setDetailObj(obj as (typeof visibleObjects)[number]))
      .catch((err) => console.warn('[Workspace] Fetch object detail failed:', err));
  }, [detailObjectId, accountId]);

  const customPage = pageId ? customPages.find((p) => p.id === pageId) : null;

  const resolveCollectionLabel = useCallback((collectionType: string) => {
    if (['identity', 'travel', 'financial', 'professional'].includes(collectionType)) {
      return t(`navigation:${collectionType}`);
    }
    const cp = customPages.find((p) => p.id === collectionType);
    return cp?.name || collectionType;
  }, [t, customPages]);

  const activeCategoryLabel = sectionFilter
    ? t(`navigation:${sectionFilter}`, sectionFilter)
    : null;

  // Password guard state — shared between detail panel and history viewer.
  const {
    showPwDialog,
    setShowPwDialog,
    pwResolveRef,
    bioAvailable,
    passwordHint,
    passwordVerify,
    verifyVaultPassword,
    handleBiometricUnlock,
  } = useWorkspacePasswordGuard();

  // F011: cache template field metadata so lookups are O(1) instead of O(n²).
  const templateFieldMap = useMemo(() => {
    const map = new Map<string, Map<string, TemplateProperty>>();
    for (const t of userTemplates) {
      map.set(t.id, new Map(t.properties.map((p) => [p.id, p])));
    }
    return map;
  }, [userTemplates]);

  const getFieldProperty = useCallback((
    templateId: string | undefined,
    fieldKey: string,
  ): TemplateProperty | undefined => {
    return templateFieldMap.get(templateId || '')?.get(fieldKey);
  }, [templateFieldMap]);

  const getFieldSensitivity = useCallback((
    templateId: string | undefined,
    fieldKey: string,
    propertyLabels?: Record<string, string>,
  ): SensitivityLevel => {
    // 1. 对象自有 propertyLabels（即使模板被删除也保留敏感度）
    if (propertyLabels?.[fieldKey]) {
      return propertyLabels[fieldKey] as SensitivityLevel;
    }
    // 2. 回退到模板定义
    return (getFieldProperty(templateId, fieldKey)?.sensitivityLevel as SensitivityLevel) || 'public';
  }, [getFieldProperty]);

  const isFieldDeprecated = useCallback((templateId: string | undefined, fieldKey: string): boolean => {
    return !!getFieldProperty(templateId, fieldKey)?.deprecatedAt;
  }, [getFieldProperty]);

  const getFieldName = useCallback((
    templateId: string | undefined,
    fieldKey: string,
    propertyFields?: Record<string, { name: string }>,
  ): string => {
    return getFieldProperty(templateId, fieldKey)?.name
      || propertyFields?.[fieldKey]?.name
      || fieldKey;
  }, [getFieldProperty]);

  useEffect(() => {
    if (accountId) {
      if (pageId) {
        loadObjects(accountId, { parentId: pageId });
      } else {
        loadObjects(accountId, sectionFilter ? { collectionType: sectionFilter } : undefined);
      }
    }
  }, [accountId, sectionFilter, pageId, loadObjects]);

  // Debounce searchQuery to avoid high-frequency IPC calls on every keystroke
  useEffect(() => {
    const timer = setTimeout(() => setDebouncedSearchQuery(searchQuery), DEBOUNCE_DELAY_MS);
    return () => clearTimeout(timer);
  }, [searchQuery]);

  const visibleObjects = useMemo(
    () =>
      objects.filter(
        (obj) =>
          obj.collectionType !== 'page' &&
          obj.collectionType !== 'unknown' &&
          obj.name.toLowerCase().includes(debouncedSearchQuery.toLowerCase()),
      ),
    [objects, debouncedSearchQuery],
  );

  const snapshotReqRef = useRef(0);

  // Load snapshot counts for visible objects
  /* eslint-disable react-hooks/exhaustive-deps */
  useEffect(() => {
    const ids = visibleObjects.map((o) => o.id);
    if (ids.length === 0) return;
    const reqId = ++snapshotReqRef.current;
    invoke<Record<string, number>>('snapshot_count_batch', { objectIds: ids })
      .then((counts) => {
        if (snapshotReqRef.current !== reqId) return; // stale response, discard
        // Ensure every visible object has a snapshot count (default 0)
        const full: Record<string, number> = {};
        for (const id of ids) full[id] = counts[id] ?? 0;
        setSnapshotCounts(full);
      })
      .catch((err) => {
        if (snapshotReqRef.current !== reqId) return; // stale error, discard
        console.warn('[Workspace] Snapshot count batch failed:', err);
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
      .catch((err) => console.warn('[Workspace] Attachment count batch failed:', err));
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
              fontSize: 'var(--text-body-sm)',
              fontWeight: 500,
              cursor: 'pointer',
              transition: 'background 0.2s, border-color 0.2s',
            }}
          >
            + {t('create')}
          </button>
          {pageId && customPage && (
            <Button variant="danger-outline" size="sm" onClick={() => setConfirmPageDelete(true)} title={t('delete')}>
              <Trash size={ICON_SIZE.sm} /> {t('delete')}
            </Button>
          )}
        </div>
      }
    >
      <PageContainer variant="medium" gap="default">
        <div
          style={{ display: 'contents' }}
          onMouseDown={(e) => {
            if (e.detail > 1) e.preventDefault();
          }}
        >
          <WorkspaceCategoryTabs
            sectionFilter={sectionFilter}
            pageId={pageId}
            customPages={customPages}
            activeCustomPages={activeCustomPages}
          />

        <Input
          placeholder={t('search_objects_placeholder')}
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          onClear={() => setSearchQuery('')}
          prefixIcon={<Search size={ICON_SIZE.sm} style={{ color: 'var(--text-tertiary)' }} />}
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
                fontSize: 'var(--text-sm)',
              }}
            >
              {searchQuery ? t('no_matching_objects') : t('no_objects')}
            </p>
          </Card>
        )}
        {!isLoading && visibleObjects.length > 0 && (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--card-gap-sm)' }}>
            {visibleObjects.map((obj) => (
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
                onUploadComplete={refreshAttachmentCounts}
                onAttachments={() => setAttachmentObjId(obj.id)}
                onEdit={() => navigate(`/editor/${obj.id}`)}
                onDelete={() => setConfirmDelete({ id: obj.id, name: obj.name })}
              />
            ))}
          </div>
        )}

        {/* Page delete confirmation dialog */}
        <ConfirmDeleteDialog
          isOpen={confirmPageDelete && !!pageId && !!customPage}
          title={t('object_delete_confirm_title')}
          body={t('object_delete_confirm_body', { name: (customPage?.name || '').length > 28 ? (customPage?.name || '').slice(0, 27) + '…' : (customPage?.name || '') })}
          confirmLabel={t('delete')}
          cancelLabel={t('cancel')}
          onCancel={() => setConfirmPageDelete(false)}
          onConfirm={async () => {
            setConfirmPageDelete(false);
            if (accountId && pageId) {
              await removeCustomPage(accountId, pageId);
              navigate('/');
            }
          }}
        />

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
            onAttachmentsChange={refreshAttachmentCounts}
          />
        )}

        <ConfirmDeleteDialog
          isOpen={!!confirmDelete}
          title={t('object_delete_confirm_title')}
          body={t('object_delete_confirm_body', { name: (confirmDelete?.name || '').length > 28 ? (confirmDelete?.name || '').slice(0, 27) + '…' : (confirmDelete?.name || '') })}
          confirmLabel={t('delete')}
          cancelLabel={t('cancel')}
          onCancel={() => setConfirmDelete(null)}
          onConfirm={() => {
            if (confirmDelete) handleDelete(confirmDelete.id);
          }}
        />
        </div>
      </PageContainer>
      {historyObj && (() => {
        const historyObjData = objects.find((o) => o.id === historyObj.id);
        const historyLabels = historyObjData?.propertyLabels;
        const historyFields = (historyObjData?.properties as Record<string, unknown>)?.__fields as
          | Record<string, { name: string }>
          | undefined;
        return (
          <HistoryViewer
            objectId={historyObj.id}
            objectName={historyObj.name}
            collectionType={historyObj.collectionType}
            onClose={() => setHistoryObj(null)}
            passwordVerify={passwordVerify}
            getFieldSensitivity={(fieldKey) =>
              getFieldSensitivity(historyObj.templateId, fieldKey, historyLabels)
            }
            isFieldDeprecated={(fieldKey) => isFieldDeprecated(historyObj.templateId, fieldKey)}
            getFieldName={(fieldKey) =>
              getFieldName(historyObj.templateId, fieldKey, historyFields)
            }
            fieldOrder={userTemplates
              .find((t) => t.id === historyObj.templateId)
              ?.properties.map((p) => p.id)}
          />
        );
      })()}
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
