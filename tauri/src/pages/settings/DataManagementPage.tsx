import { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { invoke } from '@tauri-apps/api/core';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { HardDrive } from 'lucide-react';

interface VaultStats {
  profile_count: number;
  total_size_bytes: number;
  last_modified?: string;
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

export function DataManagementPage() {
  const navigate = useNavigate();
  const { t } = useTranslation(['settings', 'common']);
  const [stats, setStats] = useState<VaultStats | null>(null);

  useEffect(() => {
    invoke<VaultStats>('get_vault_stats')
      .then(setStats)
      .catch(() => setStats(null));
  }, []);

  return (
    <AppShell title={t('settings:data_management')} onBack={() => navigate('/settings')}>
      <div style={{ maxWidth: 480, margin: '0 auto', display: 'flex', flexDirection: 'column', gap: 12 }}>
        {/* Vault stats card */}
        <Card>
          <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
            <div style={{
              width: 44, height: 44, borderRadius: 10, display: 'flex', alignItems: 'center', justifyContent: 'center',
              background: 'rgba(91,124,153,0.1)',
            }}>
              <HardDrive size={22} style={{ color: 'var(--accent-primary)' }} />
            </div>
            <div>
              <div style={{ fontSize: 13, color: 'var(--text-secondary)' }}>{t('settings:vault_size')}</div>
              <div style={{ fontSize: 20, fontWeight: 600 }}>
                {stats ? formatBytes(stats.total_size_bytes) : '...'}
              </div>
              <div style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>
                {stats ? t('settings:profile_count', { count: stats.profile_count }) : t('common:loading')}
                {stats?.last_modified && ` · ${t('settings:updated')} ${new Date(stats.last_modified).toLocaleDateString()}`}
              </div>
            </div>
          </div>
        </Card>

        {/* Quick actions */}
        <Card>
          <h3 style={{ fontSize: 15, fontWeight: 600, marginBottom: 4 }}>{t('settings:backup')}</h3>
          <p style={{ fontSize: 13, color: 'var(--text-secondary)', marginBottom: 12 }}>
            {t('settings:backup_desc')}
          </p>
          <div style={{ display: 'flex', gap: 8 }}>
            <Button size="sm" variant="primary" onClick={() => navigate('/settings/backup')}>
              {t('settings:create_backup')}
            </Button>
            <Button size="sm" variant="secondary" onClick={() => navigate('/settings/backup')}>
              {t('settings:restore')}
            </Button>
          </div>
        </Card>

        <Card>
          <h3 style={{ fontSize: 15, fontWeight: 600, marginBottom: 4 }}>{t('settings:export_import')}</h3>
          <p style={{ fontSize: 13, color: 'var(--text-secondary)', marginBottom: 12 }}>
            {t('settings:export_import_desc')}
          </p>
          <div style={{ display: 'flex', gap: 8 }}>
            <Button size="sm" variant="primary" onClick={() => navigate('/settings/export-import')}>
              {t('settings:export')}
            </Button>
            <Button size="sm" variant="secondary" onClick={() => navigate('/settings/export-import')}>
              {t('settings:import')}
            </Button>
          </div>
        </Card>

        <Card>
          <h3 style={{ fontSize: 15, fontWeight: 600, marginBottom: 4 }}>{t('settings:trash')}</h3>
          <p style={{ fontSize: 13, color: 'var(--text-secondary)', marginBottom: 12 }}>
            {t('settings:trash_empty')}
          </p>
          <Button size="sm" variant="secondary" onClick={() => navigate('/settings/trash')}>
            {t('settings:trash')}
          </Button>
        </Card>
      </div>
    </AppShell>
  );
}
