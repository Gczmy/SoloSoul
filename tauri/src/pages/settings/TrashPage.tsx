import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';

export function TrashPage() {
  const navigate = useNavigate();
  const { t } = useTranslation(['settings', 'common']);

  return (
    <AppShell title={t('settings:trash')} onBack={() => navigate('/settings')}>
      <div style={{ maxWidth: 480, margin: '0 auto' }}>
        <Card>
          <p
            style={{
              fontSize: 14,
              color: 'var(--text-secondary)',
              textAlign: 'center',
              padding: '24px 0',
            }}
          >
            {t('settings:trash_empty')}
          </p>
        </Card>
      </div>
    </AppShell>
  );
}
