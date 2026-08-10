import { useTranslation } from 'react-i18next';
import { Download, FileText, Upload } from 'lucide-react';
import { ICON_SIZE } from '@/lib/constants';
import styles from './ExportImportTabBar.module.css';

export type ExportImportTabKey = 'export' | 'import' | 'document';

interface ExportImportTabBarProps {
  tab: ExportImportTabKey;
  onChange: (tab: ExportImportTabKey) => void;
}

const PANEL_CONFIG = {
  export: {
    labelKey: 'settings:export',
    Icon: Download,
  },
  document: {
    labelKey: 'settings:export_as_document',
    Icon: FileText,
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
      {(['export', 'document', 'import'] as const).map((tabKey) => {
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
