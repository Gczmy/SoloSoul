import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { Input } from '@/components/ui/Input';
import { Button } from '@/components/ui/Button';
import { useAuthStore } from '@/stores/authStore';
import { useToastError } from '@/hooks/useToastError';
import { Layers, Plus, Trash2, Filter } from 'lucide-react';

interface FilterCondition { field: string; op: string; value: string; }
interface SmartFilter { operator: string; conditions: FilterCondition[]; }
interface CollectionDef {
  id: string; name: string; iconName: string;
  description?: string; filter?: SmartFilter;
  sortOrder: number; createdAt: string;
}

const BUILTIN_COLLECTION_KEYS = ['identity', 'travel', 'financial', 'professional'] as const;

export function CollectionsPage() {
  const navigate = useNavigate();
  const { t } = useTranslation(['settings', 'common', 'navigation']);
  const accountId = useAuthStore((s) => s.currentAccount?.id);
  const { onError, onSuccess } = useToastError();

  const [collections, setCollections] = useState<CollectionDef[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [newName, setNewName] = useState('');
  const [isCreating, setIsCreating] = useState(false);

  const load = async () => {
    if (!accountId) return;
    try {
      const cols = await invoke<CollectionDef[]>('collection_list', { accountId });
      setCollections(cols);
    } catch (e) { onError(e, t('settings:collections_load_failed')); }
    finally { setIsLoading(false); }
  };

  useEffect(() => { load(); }, [accountId]);

  const handleCreate = async () => {
    if (!accountId || !newName.trim()) return;
    setIsCreating(true);
    try {
      await invoke('collection_create', {
        input: { accountId, name: newName.trim(), iconName: 'folder' },
      });
      setNewName('');
      onSuccess(t('settings:collections_created'));
      await load();
    } catch (e) { onError(e, t('settings:collections_create_failed')); }
    finally { setIsCreating(false); }
  };

  const handleDelete = async (id: string) => {
    if (!accountId || !confirm(t('common:confirm'))) return;
    try {
      await invoke('collection_delete', { accountId, collectionId: id });
      onSuccess(t('settings:collections_deleted'));
      await load();
    } catch (e) { onError(e, t('settings:collections_delete_failed')); }
  };

  if (isLoading) return <AppShell title={t('settings:collections_title')} onBack={() => navigate('/settings')}><p>Loading...</p></AppShell>;

  return (
    <AppShell title={t('settings:collections_title')} onBack={() => navigate('/settings')}>
      <div style={{ maxWidth: 520, margin: '0 auto', display: 'flex', flexDirection: 'column', gap: 16 }}>
        {/* Built-in collections (read-only) */}
        <Card>
          <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 12 }}>
            <Layers size={14} style={{ verticalAlign: 'middle', marginRight: 4 }} />
            {t('settings:collections_builtin')}
          </h3>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
            {BUILTIN_COLLECTION_KEYS.map((key) => (
              <div key={key} style={{
                display: 'flex', alignItems: 'center', gap: 8, padding: '8px 10px',
                borderRadius: 8, background: 'var(--bg-toolbar)', fontSize: 13,
              }}>
                <span style={{ fontWeight: 500 }}>{t(`navigation:${key}`, key)}</span>
                <span style={{ marginLeft: 'auto', fontSize: 11, color: 'var(--text-tertiary)' }}>{t('settings:collections_builtin_badge')}</span>
              </div>
            ))}
          </div>
        </Card>

        {/* User collections */}
        <Card>
          <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 12 }}>
            <Layers size={14} style={{ verticalAlign: 'middle', marginRight: 4 }} />
            {t('settings:collections_my')}
          </h3>
          {/* Create form */}
          <div style={{ display: 'flex', gap: 8, marginBottom: 12 }}>
            <Input
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
              placeholder={t('settings:collections_name_placeholder')}
              onKeyDown={(e) => { if (e.key === 'Enter') handleCreate(); }}
              style={{ flex: 1 }}
            />
            <Button onClick={handleCreate} loading={isCreating} disabled={!newName.trim()}>
              <Plus size={14} style={{ marginRight: 4 }} /> {t('settings:collections_add')}
            </Button>
          </div>
          {/* List */}
          {collections.length === 0 ? (
            <p style={{ fontSize: 13, color: 'var(--text-tertiary)', textAlign: 'center', padding: 16 }}>
              {t('settings:collections_empty')}
            </p>
          ) : (
            collections.map((c) => (
              <div key={c.id} style={{
                display: 'flex', alignItems: 'center', gap: 8, padding: '8px 10px',
                borderRadius: 8, background: 'var(--bg-toolbar)', fontSize: 13,
                marginTop: 4,
              }}>
                {c.filter ? <Filter size={12} style={{ color: 'var(--accent-primary)' }} /> : null}
                <span style={{ fontWeight: 500 }}>{c.name}</span>
                {c.filter && <span style={{ fontSize: 10, color: 'var(--accent-primary)', marginLeft: 4 }}>{t('settings:collections_smart_badge')}</span>}
                <span style={{ marginLeft: 'auto' }} />
                <button
                  onClick={() => handleDelete(c.id)}
                  title={t('settings:collections_delete')}
                  style={{ padding: 4, borderRadius: 4, border: 'none', background: 'transparent', cursor: 'pointer', color: 'var(--text-tertiary)' }}
                >
                  <Trash2 size={14} />
                </button>
              </div>
            ))
          )}
        </Card>

        {/* Smart collection info */}
        <Card>
          <div style={{ fontSize: 12, color: 'var(--text-tertiary)', lineHeight: 1.6 }}>
            {t('settings:collections_smart_desc')}
          </div>
        </Card>
      </div>
    </AppShell>
  );
}
