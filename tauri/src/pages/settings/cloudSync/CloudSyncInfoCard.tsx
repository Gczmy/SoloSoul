/**
 * P007：云同步设置页——说明信息卡（纯静态内容）。
 */
import { Info } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Card } from '@/components/ui/Card';
import styles from '../CloudSyncPage.module.css';

export function CloudSyncInfoCard() {
  const { t } = useTranslation(['settings']);
  return (
    <Card className={styles.infoCard}>
      <h2 className={styles.sectionTitle}>
        <Info size={20} style={{ marginRight: 8 }} />
        {t('settings:cloud_sync_info_title')}
      </h2>
      <ul className={styles.infoList}>
        <li>{t('settings:cloud_sync_info_1')}</li>
        <li>{t('settings:cloud_sync_info_2')}</li>
        <li>{t('settings:cloud_sync_info_3')}</li>
        <li>{t('settings:cloud_sync_info_4')}</li>
        <li>{t('settings:cloud_sync_info_5')}</li>
      </ul>
    </Card>
  );
}
