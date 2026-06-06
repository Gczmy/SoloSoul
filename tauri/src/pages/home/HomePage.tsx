import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { IdCard, Plane, Banknote, Briefcase } from 'lucide-react';
import type { ProfileSection } from '@/types';

const sections: { type: ProfileSection; labelKey: string; icon: typeof IdCard; descKey: string }[] = [
  { type: 'identity', labelKey: 'profile', icon: IdCard, descKey: 'identity_desc' },
  { type: 'travel', labelKey: 'travel', icon: Plane, descKey: 'travel_desc' },
  { type: 'financial', labelKey: 'financial', icon: Banknote, descKey: 'financial_desc' },
  { type: 'professional', labelKey: 'professional', icon: Briefcase, descKey: 'professional_desc' },
];

export function HomePage() {
  const navigate = useNavigate();
  const { t } = useTranslation(['common', 'navigation']);

  return (
    <AppShell title={t('common:home')}>
      <div
        style={{
          display: 'flex',
          flexDirection: 'column',
          gap: 20,
          maxWidth: 720,
          margin: '0 auto',
        }}
      >
        <Card>
          <h2 style={{ fontSize: 20, fontWeight: 600, marginBottom: 4 }}>{t('common:welcome_back')}</h2>
          <p style={{ fontSize: 14, color: 'var(--text-secondary)' }}>
            {t('common:vault_description')}
          </p>
        </Card>

        <div
          style={{
            display: 'grid',
            gridTemplateColumns: 'repeat(auto-fill, minmax(240px, 1fr))',
            gap: 12,
          }}
        >
          {sections.map((s) => (
            <Card key={s.type} interactive onClick={() => navigate(`/workspace?section=${s.type}`)}>
              <div style={{ marginBottom: 8 }}><s.icon size={28} /></div>
              <h3 style={{ fontSize: 15, fontWeight: 600, marginBottom: 4 }}>
                {t(`navigation:${s.labelKey}`)}
              </h3>
              <p style={{ fontSize: 13, color: 'var(--text-secondary)' }}>{t(`common:${s.descKey}`)}</p>
            </Card>
          ))}
        </div>
      </div>
    </AppShell>
  );
}
