/**
 * P007：云同步设置页——同步状态卡（上次同步时间 / 连接器 / 自动同步 / 间隔）。
 */
import { Clock } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Card } from '@/components/ui/Card';
import { CONNECTOR_OPTIONS, type SavedCloudSyncConfig } from './cloudSyncShared';
import styles from '../CloudSyncPage.module.css';

interface CloudSyncStatusCardProps {
  savedConfig: SavedCloudSyncConfig;
}

export function CloudSyncStatusCard({ savedConfig }: CloudSyncStatusCardProps) {
  const { t } = useTranslation(['settings', 'common']);
  return (
    <Card className={styles.statusCard}>
      <h2 className={styles.sectionTitle}>
        <Clock size={20} style={{ marginRight: 8 }} />
        {t('settings:cloud_sync_status')}
      </h2>
      <div className={styles.statusGrid}>
        <div className={styles.statusItem}>
          <span className={styles.statusLabel}>{t('settings:cloud_sync_last_sync')}</span>
          <span className={styles.statusValue}>
            {savedConfig.lastSyncAt
              ? new Date(savedConfig.lastSyncAt).toLocaleString()
              : t('settings:cloud_sync_never')}
          </span>
        </div>
        <div className={styles.statusItem}>
          <span className={styles.statusLabel}>{t('settings:cloud_sync_connector')}</span>
          <span className={styles.statusValue}>
            {CONNECTOR_OPTIONS.find((o) => o.value === savedConfig.connectorType)?.label ||
              savedConfig.connectorType}
          </span>
        </div>
        <div className={styles.statusItem}>
          <span className={styles.statusLabel}>{t('settings:cloud_sync_auto_sync')}</span>
          <span className={styles.statusValue}>
            {savedConfig.enabled ? t('common:enabled') : t('common:disabled')}
          </span>
        </div>
        <div className={styles.statusItem}>
          <span className={styles.statusLabel}>{t('settings:cloud_sync_interval')}</span>
          <span className={styles.statusValue}>
            {savedConfig.intervalSecs
              ? `${Math.round(savedConfig.intervalSecs / 60)} 分钟`
              : '—'}
          </span>
        </div>
      </div>
    </Card>
  );
}
