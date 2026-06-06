import { useTranslation } from 'react-i18next';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { Smartphone } from 'lucide-react';

/** P3 — Sync page, wireframe for future device sync */
export function SyncPage() {
  const { t } = useTranslation(['settings', 'common']);
  return (
    <AppShell title={t('settings:items.sync', { defaultValue: 'Device Sync' })}>
      <div style={{ maxWidth: 600, margin: '0 auto', display: 'flex', flexDirection: 'column', gap: 16 }}>
        <Card>
          <div style={{ textAlign: 'center', padding: '48px 24px' }}>
            <Smartphone size={48} style={{ marginBottom: 16, opacity: 0.3, color: 'var(--text-tertiary)' }} />
            <h2 style={{ fontSize: 18, fontWeight: 600, margin: '0 0 8px' }}>
              {t('settings:sync_title', { defaultValue: 'Device Sync' })}
            </h2>
            <p style={{ fontSize: 14, color: 'var(--text-secondary)', margin: 0 }}>
              {t('settings:sync_description', { defaultValue: 'Device sync is under development.' })}
            </p>
          </div>
        </Card>
      </div>
    </AppShell>
  );
}
