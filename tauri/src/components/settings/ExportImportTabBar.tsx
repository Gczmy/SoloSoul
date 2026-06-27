import { useTranslation } from 'react-i18next';
import { Download, Upload } from 'lucide-react';
import { ICON_SIZE } from '@/lib/iconSizes';
import styles from './ExportImportTabBar.module.css';


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

  return (
    <div className={styles.container}>
      {(['export', 'import'] as const).map((tabKey) => {
        const isActive = tab === tabKey;
        const { Icon } = PANEL_CONFIG[tabKey];

        return (
          <button
            key={tabKey}
            onClick={() => onChange(tabKey)}
            className={`${styles.tab} ${isActive ? styles.tabActive : ''}`}
          >
            <Icon
              size={ICON_SIZE.md}
              className={`${styles.tabIcon} ${isActive ? styles.tabIconActive : ''}`}
            />
            {t(PANEL_CONFIG[tabKey].labelKey)}
          </button>
        );
      })}
    </div>
  );
}
