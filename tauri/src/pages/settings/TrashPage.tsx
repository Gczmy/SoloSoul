import { useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { Input } from '@/components/ui/Input';
import { Button } from '@/components/ui/Button';
import { useAuthStore } from '@/stores/authStore';
import { useTrashStore, TrashTimeFilter, TrashTypeFilter } from '@/stores/trashStore';
import { Trash2, RotateCcw, FileText } from 'lucide-react';

const TIME_OPTIONS: { value: TrashTimeFilter; labelKey: string }[] = [
  { value: 'all', labelKey: 'all' },
  { value: '1d', labelKey: '1d' },
  { value: '3d', labelKey: '3d' },
  { value: '7d', labelKey: '7d' },
  { value: '30d', labelKey: '30d' },
  { value: 'half_year', labelKey: 'half_year' },
];

const TYPE_OPTIONS: { value: TrashTypeFilter; i18nKey: string }[] = [
  { value: 'all', i18nKey: 'all' },
  { value: 'page', i18nKey: 'page' },
  { value: 'object', i18nKey: 'object' },
];

function timeAgo(ms: number, t: (k: string) => string): string {
  const diff = Date.now() - ms;
  const mins = Math.floor(diff / 60000);
  if (mins < 60) return t('time_minutes_ago').replace('{n}', String(mins));
  const hours = Math.floor(mins / 60);
  if (hours < 24) return t('time_hours_ago').replace('{n}', String(hours));
  const days = Math.floor(hours / 24);
  if (days < 30) return t('time_days_ago').replace('{n}', String(days));
  return t('time_months_ago').replace('{n}', String(Math.floor(days / 30)));
}

export function TrashPage() {
  const navigate = useNavigate();
  const { t } = useTranslation(['settings', 'common']);
  const accountId = useAuthStore((s) => s.currentAccount?.id);
  const {
    items, timeFilter, typeFilter, searchQuery,
    loadItems, setTimeFilter, setTypeFilter, setSearchQuery,
    restoreItem, permanentDelete, isLoading,
  } = useTrashStore();

  useEffect(() => {
    if (accountId) loadItems(accountId);
  }, [accountId, timeFilter]);

  const filtered = items
    .filter((i) => typeFilter === 'all' || i.itemType === typeFilter)
    .filter((i) => !searchQuery || i.name.toLowerCase().includes(searchQuery.toLowerCase()));

  return (
    <AppShell title={t('settings:trash')} onBack={() => navigate('/settings')}>
      <div style={{ maxWidth: 600, margin: '0 auto', display: 'flex', flexDirection: 'column', gap: 12 }}>
        {/* Filters */}
        <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap' }}>
          {TIME_OPTIONS.map((opt) => (
            <button
              key={opt.value}
              onClick={() => setTimeFilter(opt.value)}
              style={{
                padding: '5px 12px', borderRadius: 6, border: '1px solid var(--border-subtle)',
                background: timeFilter === opt.value ? 'var(--accent-primary)' : 'transparent',
                color: timeFilter === opt.value ? 'white' : 'var(--text-secondary)',
                fontSize: 12, cursor: 'pointer',
              }}
            >
              {t(`settings:${opt.labelKey}`, opt.labelKey)}
            </button>
          ))}
          <span style={{ width: 1, background: 'var(--border-subtle)', margin: '2px 4px' }} />
          {TYPE_OPTIONS.map((opt) => (
            <button
              key={opt.value}
              onClick={() => setTypeFilter(opt.value)}
              style={{
                padding: '5px 12px', borderRadius: 6, border: '1px solid var(--border-subtle)',
                background: typeFilter === opt.value ? 'var(--accent-primary)' : 'transparent',
                color: typeFilter === opt.value ? 'white' : 'var(--text-secondary)',
                fontSize: 12, cursor: 'pointer',
              }}
            >
              {t(`settings:trash_type_${opt.value}`)}
            </button>
          ))}
        </div>

        <Input
          placeholder={t('settings:search_logs')}
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
        />

        {/* List */}
        {isLoading ? (
          <Card><p style={{ textAlign: 'center', color: 'var(--text-tertiary)', padding: 24 }}>{t('common:loading')}</p></Card>
        ) : filtered.length === 0 ? (
          <Card>
            <div style={{ textAlign: 'center', padding: '48px 24px' }}>
              <Trash2 size={48} style={{ marginBottom: 12, opacity: 0.25, color: 'var(--text-tertiary)' }} />
              <p style={{ fontSize: 14, color: 'var(--text-secondary)' }}>{t('settings:trash_empty')}</p>
            </div>
          </Card>
        ) : (
          filtered.map((item) => (
            <Card key={item.id}>
              <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
                  <FileText size={18} style={{ color: 'var(--text-tertiary)' }} />
                  <div>
                    <div style={{ fontSize: 13, fontWeight: 500 }}>{item.name}</div>
                    <div style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>
                      {t(`settings:trash_type_${item.itemType}`)} · {timeAgo(item.deletedAt, t)}
                      {item.expiresAt && ` · ${t('settings:trash_expires_in', { days: Math.max(0, Math.floor((item.expiresAt - Date.now()) / 86400000)) })}`}
                    </div>
                  </div>
                </div>
                <div style={{ display: 'flex', gap: 6 }}>
                  <Button size="sm" onClick={() => restoreItem(item.id)}>
                    <RotateCcw size={13} style={{ marginRight: 3 }} /> {t('common:restore')}
                  </Button>
                  <Button size="sm" variant="secondary" onClick={() => permanentDelete([item.id])}>
                    {t('common:delete_permanently')}
                  </Button>
                </div>
              </div>
            </Card>
          ))
        )}
      </div>
    </AppShell>
  );
}
