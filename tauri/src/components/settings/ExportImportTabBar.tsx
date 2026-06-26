import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Download, Upload } from 'lucide-react';
import { ICON_SIZE } from '@/lib/iconSizes';


type TabKey = 'export' | 'import';

interface ExportImportTabBarProps {
  tab: TabKey;
  onChange: (tab: TabKey) => void;
}

const PANEL_CONFIG = {
  export: {
    labelKey: 'settings:export',
    Icon: Download,
  },
  import: {
    labelKey: 'settings:import',
    Icon: Upload,
  },
} as const;

export function ExportImportTabBar({ tab, onChange }: ExportImportTabBarProps) {
  const { t } = useTranslation(['settings', 'common']);
  const [hovered, setHovered] = useState<TabKey | null>(null);

  return (
    <div
      style={{
        display: 'flex',
        gap: 2,
        padding: 3,
        borderRadius: 10,
        border: '1px solid var(--border-subtle)',
        background: 'var(--bg-toolbar)',
      }}
    >
      {(['export', 'import'] as const).map((tabKey) => {
        const isActive = tab === tabKey;
        const isHovered = hovered === tabKey;
        const { Icon } = PANEL_CONFIG[tabKey];

        return (
          <button
            key={tabKey}
            onClick={() => onChange(tabKey)}
            onMouseEnter={() => setHovered(tabKey)}
            onMouseLeave={() => setHovered(null)}
            style={{
              flex: 1,
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              gap: 8,
              padding: '10px 12px',
              border: 'none',
              borderRadius: 8,
              cursor: 'pointer',
              fontSize: 'var(--text-body)',
              fontWeight: isActive ? 600 : 500,
              fontFamily: 'inherit',
              background: isActive
                ? 'var(--accent-primary)'
                : isHovered
                  ? 'color-mix(in srgb, var(--accent-primary) 8%, transparent)'
                  : 'transparent',
              color: isActive
                ? 'white'
                : 'var(--text-secondary)',
              transition: 'all var(--duration-fast) var(--ease-smooth)',
            }}
          >
            <Icon
              size={ICON_SIZE.md}
              style={{
                transition: 'transform var(--duration-fast) var(--ease-smooth)',
                transform: isActive ? 'scale(1.05)' : 'scale(1)',
              }}
            />
            {t(PANEL_CONFIG[tabKey].labelKey)}
          </button>
        );
      })}
    </div>
  );
}
