import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { AppShell } from '@/components/layout/AppShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { Card } from '@/components/ui/Card';
import { CardGrid } from '@/components/ui/CardGrid';
import { PAGE_ICON_MAP } from '@/lib/pageIcons';
import type { ProfileSection } from '@/types';

// Icons sourced from PAGE_ICON_MAP — §7.4 Single Source of Truth
const sections: {
  type: ProfileSection;
  labelKey: string;
  icon: typeof PAGE_ICON_MAP.profile;
  descKey: string;
}[] = [
  { type: 'identity', labelKey: 'identity', icon: PAGE_ICON_MAP.profile, descKey: 'identity_desc' },
  { type: 'travel', labelKey: 'travel', icon: PAGE_ICON_MAP.travel, descKey: 'travel_desc' },
  {
    type: 'financial',
    labelKey: 'financial',
    icon: PAGE_ICON_MAP.financial,
    descKey: 'financial_desc',
  },
  {
    type: 'professional',
    labelKey: 'professional',
    icon: PAGE_ICON_MAP.professional,
    descKey: 'professional_desc',
  },
];

const helpCard: QuickCard = {
  path: '/help',
  labelKey: 'help',
  icon: PAGE_ICON_MAP.help,
  descKey: 'help_desc',
};

type QuickCard = {
  path: string;
  labelKey: string;
  icon: typeof PAGE_ICON_MAP.profile;
  descKey: string;
};

const quickCards: QuickCard[] = [
  {
    path: '/settings',
    labelKey: 'settings',
    icon: PAGE_ICON_MAP.settings,
    descKey: 'settings_desc',
  },
  {
    path: '/settings/trash',
    labelKey: 'trash',
    icon: PAGE_ICON_MAP.trash,
    descKey: 'trash_desc',
  },
  {
    path: '/plugins',
    labelKey: 'plugins',
    icon: PAGE_ICON_MAP.plugins,
    descKey: 'plugins_desc',
  },
  {
    path: '/ocr',
    labelKey: 'ocr',
    icon: PAGE_ICON_MAP.ocr,
    descKey: 'ocr_desc',
  },
  {
    path: '/search',
    labelKey: 'search',
    icon: PAGE_ICON_MAP.search,
    descKey: 'search_desc',
  },
  {
    path: '/llm-chat',
    labelKey: 'ai_chat',
    icon: PAGE_ICON_MAP.ai_chat,
    descKey: 'ai_chat_desc',
  },
  helpCard,
];

export function HomePage() {
  const navigate = useNavigate();
  const { t } = useTranslation(['common', 'navigation']);

  return (
    <AppShell title={t('common:home')}>
      <PageContainer variant="wide" gap="section">
        <Card>
          <h2 style={{ fontSize: 'var(--text-page-title)', fontWeight: 600, marginBottom: 4 }}>
            {t('common:welcome_back')}
          </h2>
          <p style={{ fontSize: 'var(--text-sm)', color: 'var(--text-secondary)' }}>
            {t('common:vault_description')}
          </p>
        </Card>

        <h2
          style={{
            fontSize: 'var(--text-section-title)',
            fontWeight: 600,
            marginBottom: -8,
            color: 'var(--text-primary)',
          }}
        >
          {t('common:data_sections')}
        </h2>

        {/* Profile Sections */}
        <CardGrid>
          {sections.map((s) => (
            <Card key={s.type} interactive onClick={() => navigate(`/workspace?section=${s.type}`)}>
              <div style={{ marginBottom: 8 }}>
                <s.icon size={28} />
              </div>
              <h3 style={{ fontSize: 'var(--text-card-title)', fontWeight: 600, marginBottom: 4 }}>
                {t(`navigation:${s.labelKey}`)}
              </h3>
              <p style={{ fontSize: 'var(--text-body-sm)', color: 'var(--text-secondary)' }}>
                {t(`common:${s.descKey}`)}
              </p>
            </Card>
          ))}
        </CardGrid>

        <h2
          style={{
            fontSize: 'var(--text-section-title)',
            fontWeight: 600,
            marginBottom: -8,
            color: 'var(--text-primary)',
          }}
        >
          {t('common:quick_access')}
        </h2>

        {/* Quick Access Cards */}
        <CardGrid>
          {quickCards.map((q) => (
            <Card
              key={q.path}
              interactive
              onClick={() => navigate(q.path, { state: { fromHome: true } })}
            >
              <div style={{ marginBottom: 8 }}>
                <q.icon size={28} />
              </div>
              <h3 style={{ fontSize: 'var(--text-card-title)', fontWeight: 600, marginBottom: 4 }}>
                {t(`navigation:${q.labelKey}`)}
              </h3>
              <p style={{ fontSize: 'var(--text-body-sm)', color: 'var(--text-secondary)' }}>
                {t(`common:${q.descKey}`)}
              </p>
            </Card>
          ))}
        </CardGrid>
      </PageContainer>
    </AppShell>
  );
}
