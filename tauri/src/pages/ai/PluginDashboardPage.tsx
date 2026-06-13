import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { Puzzle } from 'lucide-react';

/** P3 — Plugin dashboard, wireframe for future plugin management */
export function PluginDashboardPage() {
  const navigate = useNavigate();
  const { t } = useTranslation(['settings', 'common']);
  return (
    <AppShell
      title={t('settings:items.plugins', { defaultValue: 'Plugins' })}
      onBack={() => navigate('/home')}
    >
      <div
        style={{
          maxWidth: 600,
          margin: '0 auto',
          display: 'flex',
          flexDirection: 'column',
          gap: 16,
        }}
      >
        <Card>
          <div style={{ textAlign: 'center', padding: '48px 24px' }}>
            <Puzzle
              size={48}
              style={{ marginBottom: 16, opacity: 0.3, color: 'var(--text-tertiary)' }}
            />
            <h2 style={{ fontSize: 18, fontWeight: 600, margin: '0 0 8px' }}>
              {t('settings:plugins_title', { defaultValue: 'Plugin System' })}
            </h2>
            <p style={{ fontSize: 14, color: 'var(--text-secondary)', margin: 0 }}>
              {t('settings:plugins_description', {
                defaultValue: 'Plugin system is under development. Check back later.',
              })}
            </p>
          </div>
        </Card>
      </div>
    </AppShell>
  );
}
