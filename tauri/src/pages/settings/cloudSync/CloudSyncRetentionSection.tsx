/**
 * P007：云同步设置页——保留策略 section（recentFull 数量 + daily/weekly/monthly 开关）。
 */
import { HardDrive } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Card } from '@/components/ui/Card';
import type { RetentionPolicy } from './cloudSyncShared';
import styles from '../CloudSyncPage.module.css';

interface CloudSyncRetentionSectionProps {
  retention: RetentionPolicy;
  onRetentionChange: (v: RetentionPolicy) => void;
}

export function CloudSyncRetentionSection({
  retention,
  onRetentionChange,
}: CloudSyncRetentionSectionProps) {
  const { t } = useTranslation(['settings']);
  return (
    <Card>
      <h2 className={styles.sectionTitle}>
        <HardDrive size={20} style={{ marginRight: 8 }} />
        {t('settings:cloud_sync_retention')}
      </h2>

      <div className={styles.retentionGrid}>
        <label className={styles.retentionItem}>
          <input
            type="number"
            min={1}
            max={100}
            value={retention.recentFull}
            onChange={(e) =>
              onRetentionChange({
                ...retention,
                recentFull: Math.max(1, parseInt(e.target.value) || 1),
              })
            }
            className={styles.input}
            style={{ width: 80 }}
          />
          <span>{t('settings:cloud_sync_recent_full')}</span>
        </label>

        <label className={styles.retentionItem}>
          <input
            type="checkbox"
            checked={retention.daily}
            onChange={(e) => onRetentionChange({ ...retention, daily: e.target.checked })}
            className={styles.checkbox}
          />
          {t('settings:cloud_sync_daily')}
        </label>

        <label className={styles.retentionItem}>
          <input
            type="checkbox"
            checked={retention.weekly}
            onChange={(e) => onRetentionChange({ ...retention, weekly: e.target.checked })}
            className={styles.checkbox}
          />
          {t('settings:cloud_sync_weekly')}
        </label>

        <label className={styles.retentionItem}>
          <input
            type="checkbox"
            checked={retention.monthly}
            onChange={(e) => onRetentionChange({ ...retention, monthly: e.target.checked })}
            className={styles.checkbox}
          />
          {t('settings:cloud_sync_monthly')}
        </label>
      </div>
    </Card>
  );
}
