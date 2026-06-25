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
            style={{
              flex: 1,
              padding: '10px',
              border: 'none',
              cursor: 'pointer',
              background: isActive
                ? 'color-mix(in srgb, var(--accent-primary) 10%, transparent)'
                : 'transparent',
              color: isActive ? 'var(--accent-primary)' : 'var(--text-tertiary)',
              fontSize: 14,
              fontWeight: isActive ? 600 : 500,
              fontFamily: 'inherit',
              borderRadius: 6,
              margin: 3,
              transition: 'all 0.15s ease',
            }}
            onMouseEnter={(e) => {
              if (!isActive) {
                e.currentTarget.style.background =
                  'color-mix(in srgb, var(--accent-primary) 6%, transparent)';
                e.currentTarget.style.color = 'var(--text-primary)';
              }
            }}
            onMouseLeave={(e) => {
              if (!isActive) {
                e.currentTarget.style.background = 'transparent';
                e.currentTarget.style.color = 'var(--text-tertiary)';
              }
            }}
          >
            {tabKey === 'export' ? t('settings:export') : t('settings:import')}
          </button>
        );
      })}
    </div>
  );
}
