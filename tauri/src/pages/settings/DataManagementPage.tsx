import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';

export function DataManagementPage() {
  const navigate = useNavigate();
  const { t } = useTranslation(['settings', 'common']);

  return (
    <AppShell title={t('settings:data_management')} onBack={() => navigate('/settings')}>
      <div
        style={{
          maxWidth: 480,
          margin: '0 auto',
          display: 'flex',
          flexDirection: 'column',
          gap: 12,
        }}
      >
        <Card>
          <h3 style={{ fontSize: 15, fontWeight: 600, marginBottom: 4 }}>{t('settings:backup')}</h3>
          <p style={{ fontSize: 13, color: 'var(--text-secondary)', marginBottom: 12 }}>
            {t('settings:backup_desc')}
          </p>
          <div style={{ display: 'flex', gap: 8 }}>
            <Button size="sm" variant="primary">
              {t('settings:create_backup')}
            </Button>
            <Button size="sm" variant="secondary">
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
            <Button size="sm" variant="primary">
              {t('settings:export')}
            </Button>
            <Button size="sm" variant="secondary">
              {t('settings:import')}
            </Button>
          </div>
        </Card>
      </div>
    </AppShell>
  );
}
