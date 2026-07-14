import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { AppShell } from '@/components/layout/AppShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { Card } from '@/components/ui/Card';
import { CardGrid } from '@/components/ui/CardGrid';
import { PAGE_ICON_MAP, resolveCustomIcon } from '@/lib/pageIcons';
import { useActiveCustomPages } from '@/components/layout/useNavigationItems';
import { useIsMobile } from '@/hooks/useIsMobile';
import type { ProfileSection } from '@/types';
import type { LucideIcon } from 'lucide-react';
import styles from './HomePage.module.css';

// Icons sourced from PAGE_ICON_MAP — §7.4 Single Source of Truth
const sections: {
  type: ProfileSection;
  labelKey: string;
  icon: LucideIcon;
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

type QuickCard = {
  path: string;
  labelKey: string;
  icon: LucideIcon;
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
  {
    path: '/help',
    labelKey: 'help',
    icon: PAGE_ICON_MAP.help,
    descKey: 'help_desc',
  },
];

function HomeCard({
  icon: Icon,
  title,
  desc,
  onClick,
}: {
  icon: LucideIcon;
  title: string;
  desc?: string;
  onClick: () => void;
}) {
  const isMobile = useIsMobile();

  return (
    <Card interactive onClick={onClick}>
      {isMobile ? (
        <>
          <div className={styles.cardHeader}>
            <Icon size={24} className={styles.cardIcon} />
            <h3 className={styles.cardTitle}>{title}</h3>
          </div>
          {desc && <p className={styles.cardDesc}>{desc}</p>}
        </>
      ) : (
        <>
          <div style={{ marginBottom: 8 }}>
            <Icon size={28} />
          </div>
          <h3
            style={{
              fontSize: 'var(--text-card-title)',
              fontWeight: 600,
              marginBottom: 4,
            }}
          >
            {title}
          </h3>
          {desc && (
            <p style={{ fontSize: 'var(--text-body-sm)', color: 'var(--text-secondary)' }}>
              {desc}
            </p>
          )}
        </>
      )}
    </Card>
  );
}

export function HomePage() {
  const navigate = useNavigate();
  const { t } = useTranslation(['common', 'navigation']);
  const activeCustomPages = useActiveCustomPages();

  return (
    <AppShell title={t('common:home')}>
      <PageContainer variant="wide" gap="section">
        <Card>
          <h2 style={{ fontSize: 'var(--text-page-title)', fontWeight: 600, marginBottom: 4 }}>
            {t('common:welcome_back')}
          </h2>
          <p style={{ fontSize: 'var(--text-body)', color: 'var(--text-secondary)' }}>
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

        {/* Profile Sections + Custom Pages */}
        <CardGrid>
          {sections.map((s) => (
            <HomeCard
              key={s.type}
              icon={s.icon}
              title={t(`navigation:${s.labelKey}`)}
              desc={t(`common:${s.descKey}`)}
              onClick={() => navigate(`/workspace?section=${s.type}`)}
            />
          ))}
          {activeCustomPages.map((page) => (
            <HomeCard
              key={page.id}
              icon={resolveCustomIcon(page.iconId)}
              title={page.name}
              desc={page.description}
              onClick={() => navigate(`/workspace/custom/${page.id}`)}
            />
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
            <HomeCard
              key={q.path}
              icon={q.icon}
              title={t(`navigation:${q.labelKey}`)}
              desc={t(`common:${q.descKey}`)}
              onClick={() => navigate(q.path, { state: { fromHome: true } })}
            />
          ))}
        </CardGrid>
      </PageContainer>
    </AppShell>
  );
}
