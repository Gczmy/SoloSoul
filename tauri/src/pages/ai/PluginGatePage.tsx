import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { isDevOrDebug } from '@/lib/env';
import { PluginDashboardPage } from './PluginDashboardPage';

export function PluginGatePage() {
  const navigate = useNavigate();
  const { t } = useTranslation(['plugin', 'settings']);

  if (isDevOrDebug()) {
    return <PluginDashboardPage />;
  }

  return (
    <AppShell
      title={t('settings:items.plugins', { defaultValue: '插件' })}
      onBack={() => navigate('/settings')}
    >
      <div
        style={{
          maxWidth: 600,
          margin: '0 auto',
          paddingTop: 24,
        }}
      >
        <Card
          style={{
            textAlign: 'center',
            padding: '40px 24px',
            color: 'var(--text-secondary)',
          }}
        >
          <div
            style={{
              fontSize: 18,
              fontWeight: 600,
              color: 'var(--text-primary)',
              marginBottom: 8,
            }}
          >
            {t('plugin:coming_soon_title', { defaultValue: '插件系统开发中' })}
          </div>
          <div style={{ fontSize: 13, lineHeight: 1.6 }}>
            {t('plugin:coming_soon_desc', {
              defaultValue: '插件系统正在全力开发中，敬请期待后续版本。',
            })}
          </div>
        </Card>
      </div>
    </AppShell>
  );
}
