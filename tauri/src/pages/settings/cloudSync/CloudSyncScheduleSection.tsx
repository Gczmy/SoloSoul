/**
 * P007：云同步设置页——同步计划 section（开关 / 间隔 / Wi-Fi only / 自动导入）。
 */
import { HardDrive } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Card } from '@/components/ui/Card';
import styles from '../CloudSyncPage.module.css';

interface CloudSyncScheduleSectionProps {
  enabled: boolean;
  onEnabledChange: (v: boolean) => void;
  intervalSecs: number;
  onIntervalSecs: (v: number) => void;
  wifiOnly: boolean;
  onWifiOnlyChange: (v: boolean) => void;
  autoImport: boolean;
  onAutoImportChange: (v: boolean) => void;
}

export function CloudSyncScheduleSection({
  enabled,
  onEnabledChange,
  intervalSecs,
  onIntervalSecs,
  wifiOnly,
  onWifiOnlyChange,
  autoImport,
  onAutoImportChange,
}: CloudSyncScheduleSectionProps) {
  const { t } = useTranslation(['settings']);
  return (
    <Card>
      <h2 className={styles.sectionTitle}>
        <HardDrive size={20} style={{ marginRight: 8 }} />
        {t('settings:cloud_sync_schedule')}
      </h2>

      <div className={styles.fieldGroup}>
        <label className={styles.label}>
          <input
            type="checkbox"
            checked={enabled}
            onChange={(e) => onEnabledChange(e.target.checked)}
            className={styles.checkbox}
          />
          {t('settings:cloud_sync_auto_sync')}
        </label>
      </div>

      {enabled && (
        <>
          <div className={styles.fieldGroup}>
            <label className={styles.label}>
              {t('settings:cloud_sync_interval')}
              <input
                type="number"
                min={60}
                max={86400}
                value={intervalSecs}
                onChange={(e) =>
                  onIntervalSecs(Math.max(60, parseInt(e.target.value) || 60))
                }
                className={styles.input}
                style={{ width: 100 }}
              />
              {' '}{t('settings:cloud_sync_interval_hint')}
            </label>
          </div>

          <div className={styles.fieldGroup}>
            <label className={styles.label}>
              <input
                type="checkbox"
                checked={wifiOnly}
                onChange={(e) => onWifiOnlyChange(e.target.checked)}
                className={styles.checkbox}
              />
              {t('settings:cloud_sync_wifi_only')}
            </label>
          </div>

          <div className={styles.fieldGroup}>
            <label className={styles.label}>
              <input
                type="checkbox"
                checked={autoImport}
                onChange={(e) => onAutoImportChange(e.target.checked)}
                className={styles.checkbox}
              />
              {t('settings:cloud_sync_auto_import')}
            </label>
            <p className={styles.hint}>{t('settings:cloud_sync_auto_import_hint')}</p>
          </div>
        </>
      )}
    </Card>
  );
}
