/**
 * P007：云同步设置页——连接配置 section（连接器选择 + WebDAV 四字段）。
 * 自 CloudSyncPage.tsx 拆出的纯展示组件，状态由 useCloudSyncPage 提供。
 */
import { Shield } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Card } from '@/components/ui/Card';
import { CONNECTOR_OPTIONS } from './cloudSyncShared';
import styles from '../CloudSyncPage.module.css';

interface CloudSyncConnectionSectionProps {
  connectorType: string;
  onConnectorTypeChange: (v: string) => void;
  configJson: Record<string, unknown>;
  onConfigJson: (v: Record<string, unknown>) => void;
}

export function CloudSyncConnectionSection({
  connectorType,
  onConnectorTypeChange,
  configJson,
  onConfigJson,
}: CloudSyncConnectionSectionProps) {
  const { t } = useTranslation(['settings']);
  return (
    <Card className={styles.card}>
      <h2 className={styles.sectionTitle}>
        <Shield size={20} style={{ marginRight: 8 }} />
        {t('settings:cloud_sync_connection')}
      </h2>

      <div className={styles.fieldGroup}>
        <label className={styles.label}>{t('settings:cloud_sync_connector')}</label>
        <select
          value={connectorType}
          onChange={(e) => onConnectorTypeChange(e.target.value)}
          className={styles.select}
        >
          {CONNECTOR_OPTIONS.map((opt) => (
            <option key={opt.value} value={opt.value}>
              {opt.label}
            </option>
          ))}
        </select>
        <p className={styles.hint}>{t('settings:cloud_sync_connector_hint')}</p>
      </div>

      <div className={styles.fieldGroup}>
        <label className={styles.label}>{t('settings:cloud_sync_server_url')}</label>
        <input
          type="url"
          value={(configJson.baseUrl as string) || ''}
          onChange={(e) =>
            onConfigJson({ ...configJson, baseUrl: e.target.value })
          }
          placeholder="https://dav.jianguoyun.com/dav/"
          className={styles.input}
        />
      </div>

      <div className={styles.fieldGroup}>
        <label className={styles.label}>{t('settings:cloud_sync_username')}</label>
        <input
          type="text"
          value={(configJson.username as string) || ''}
          onChange={(e) =>
            onConfigJson({ ...configJson, username: e.target.value })
          }
          placeholder="user@example.com"
          className={styles.input}
        />
      </div>

      <div className={styles.fieldGroup}>
        <label className={styles.label}>{t('settings:cloud_sync_password')}</label>
        <input
          type="password"
          value={(configJson.password as string) || ''}
          onChange={(e) =>
            onConfigJson({ ...configJson, password: e.target.value })
          }
          placeholder="••••••••"
          className={styles.input}
          autoComplete="current-password"
        />
        <p className={styles.hint}>{t('settings:cloud_sync_password_hint')}</p>
      </div>

      <div className={styles.fieldGroup}>
        <label className={styles.label}>{t('settings:cloud_sync_root_prefix')}</label>
        <input
          type="text"
          value={(configJson.rootPrefix as string) || ''}
          onChange={(e) =>
            onConfigJson({ ...configJson, rootPrefix: e.target.value })
          }
          placeholder="/SoloSoul/"
          className={styles.input}
        />
      </div>
    </Card>
  );
}
