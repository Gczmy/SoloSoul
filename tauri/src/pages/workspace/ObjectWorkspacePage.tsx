import { useState, useEffect } from 'react';
import { useNavigate, useSearchParams, useParams } from 'react-router-dom';
import React from 'react';
import { useTranslation } from 'react-i18next';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { Input } from '@/components/ui/Input';
import { Button } from '@/components/ui/Button';
import { useAuthStore } from '@/stores/authStore';
import { useObjectStore } from '@/stores/objectStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { Pencil, Trash2 } from 'lucide-react';
import { PAGE_ICON_MAP, resolveCustomIcon } from '@/lib/pageIcons';

// Labels resolved at render time via t() so they support i18n
const CATEGORY_TYPES = ['identity', 'travel', 'financial', 'professional'] as const;
const CATEGORY_ICONS: Record<string, typeof PAGE_ICON_MAP.profile> = {
  identity: PAGE_ICON_MAP.profile,
  travel: PAGE_ICON_MAP.travel,
  financial: PAGE_ICON_MAP.financial,
  professional: PAGE_ICON_MAP.professional,
};

/** Extract displayable key-value pairs from object properties. */
function flattenProperties(
  props: Record<string, unknown> | undefined
): { key: string; value: string }[] {
  if (!props) return [];
  const result: { key: string; value: string }[] = [];
  for (const [k, v] of Object.entries(props)) {
    if (v === null || v === undefined || v === '') continue;
    if (typeof v === 'string') {
      result.push({ key: k, value: v });
    } else if (typeof v === 'number' || typeof v === 'boolean') {
      result.push({ key: k, value: String(v) });
    }
  }
  return result;
}

export function ObjectWorkspacePage() {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const { pageId } = useParams();
  const sectionFilter = searchParams.get('section') || '';
  const [searchQuery, setSearchQuery] = useState('');
  const [deletingId, setDeletingId] = useState<string | null>(null);
  const [confirmDelete, setConfirmDelete] = useState<{ id: string; name: string } | null>(null);

  const accountId = useAuthStore((s) => s.currentAccount?.id);
  const { t } = useTranslation(['common', 'navigation', 'editor']);
  const { objects, loadObjects, deleteObject, isLoading, error } = useObjectStore();
  const customPages = useSettingsStore((s) => s.settings.customPages);
  const customPage = pageId ? customPages.find((p) => p.id === pageId) : null;

  const activeCategoryLabel = sectionFilter ? t(`navigation:${sectionFilter}`, sectionFilter) : null;

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
      actions={
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
      }
    >
      <div style={{ maxWidth: 640, margin: '0 auto', display: 'flex', flexDirection: 'column', gap: 16 }}>
        {!pageId && (
          <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap' }}>
            {CATEGORY_TYPES.map((catType) => (
              <button
                key={catType}
                onClick={() => navigate(`/workspace?section=${catType}`)}
                style={{
                  padding: '6px 14px', borderRadius: 8, border: '1px solid var(--border-subtle)',
                  background: sectionFilter === catType ? 'var(--accent-primary)' : 'transparent',
                  color: sectionFilter === catType ? 'white' : 'var(--text-primary)',
                  fontSize: 13, cursor: 'pointer', display: 'flex', alignItems: 'center', gap: 4,
                }}
              >
                <span>{React.createElement(CATEGORY_ICONS[catType], { size: 16 })}</span>
                {t(`navigation:${catType}`, catType)}
              </button>
            ))}
            {sectionFilter && (
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
        )}

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
            const fields = flattenProperties(obj.properties as Record<string, unknown> | undefined);
            return (
              <Card key={obj.id} interactive onClick={() => navigate(`/editor/${obj.id}`)}>
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
                        {t(`navigation:${obj.collectionType}`, obj.collectionType)}
                      </span>
                    </div>
                  </div>
                  {/* Edit + Delete actions */}
                  <div style={{ display: 'flex', gap: 2 }} onClick={(e) => e.stopPropagation()}>
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
                {/* Property chips */}
                {fields.length > 0 && (
                  <div style={{ display: 'flex', gap: 6, flexWrap: 'wrap' }}>
                    {fields.map((f) => (
                      <span
                        key={f.key}
                        style={{
                          padding: '3px 8px', borderRadius: 6, fontSize: 11,
                          background: 'var(--bg-toolbar)', color: 'var(--text-secondary)',
                          border: '1px solid var(--border-subtle)',
                          maxWidth: 180, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap',
                        }}
                        title={`${t(`editor:fields.${f.key}`, f.key)}: ${f.value}`}
                      >
                        <span style={{ fontWeight: 500, color: 'var(--text-tertiary)', marginRight: 4 }}>
                          {t(`editor:fields.${f.key}`, f.key)}:
                        </span>
                        {f.value}
                      </span>
                    ))}
                  </div>
                )}
              </Card>
            );
          })}

        {/* Delete confirmation dialog */}
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
    </AppShell>
  );
}
