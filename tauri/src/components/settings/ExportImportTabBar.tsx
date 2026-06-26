import { useTranslation } from 'react-i18next';

type TabKey = 'export' | 'import';

interface ExportImportTabBarProps {
  tab: TabKey;
  onChange: (tab: TabKey) => void;
}

export function ExportImportTabBar({ tab, onChange }: ExportImportTabBarProps) {
  const { t } = useTranslation(['settings', 'common']);

  return (
    <div
      style={{
        display: 'flex',
        gap: 0,
        borderRadius: 8,
        overflow: 'hidden',
        border: '1px solid var(--border-subtle)',
        background: 'var(--bg-toolbar)',
      }}
    >
      {(['export', 'import'] as const).map((tabKey) => {
        const isActive = tab === tabKey;
        return (
          <button
            key={tabKey}
            onClick={() => onChange(tabKey)}
            className={isActive ? 'selected-accent' : 'interactive-accent-light'}
            style={{
              flex: 1,
              padding: '10px',
              border: 'none',
              cursor: 'pointer',
              fontSize: 14,
              fontWeight: isActive ? 600 : 500,
              fontFamily: 'inherit',
              borderRadius: 6,
              margin: 3,
            }}
          >
            {tabKey === 'export' ? t('settings:export') : t('settings:import')}
          </button>
        );
      })}
    </div>
  );
}
