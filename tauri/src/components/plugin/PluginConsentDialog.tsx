import { useTranslation } from 'react-i18next';
import { Shield, Eye, Lock } from 'lucide-react';
import styles from './PluginConsentDialog.module.css';
import type { ConsentRequestEvent } from '@/lib/plugin';

interface PluginConsentDialogProps {
  pluginName: string;
  requests: ConsentRequestEvent[];
  onApprove: (requestId: string) => void;
  onDeny: (requestId: string) => void;
}

const ICONS: Record<string, typeof Eye> = {
  public: Eye,
  internal: Eye,
  private: Eye,
  sensitive: Lock,
  restricted: Lock,
  critical: Shield,
};

export function PluginConsentDialog({
  pluginName,
  requests,
  onApprove,
  onDeny,
}: PluginConsentDialogProps) {
  const { t } = useTranslation('plugin');

  return (
    <div className={styles.overlay}>
      <div className={styles.dialog}>
        <h3 className={styles.title}>{t('consent_title', { defaultValue: 'Plugin Requests Data Access' })}</h3>
        <p className={styles.subtitle}>
          {t('consent_subtitle', { pluginName, defaultValue: `${pluginName} requests access to:` })}
        </p>

        <div className={styles.list}>
          {requests.map((req) => {
            const Icon = ICONS[req.sensitivityLevel] ?? Lock;
            return (
              <div key={req.requestId} className={styles.request}>
                <Icon size={18} className={styles.icon} />
                <div className={styles.meta}>
                  <span className={styles.fieldLabel}>{req.fieldLabel}</span>
                  <span className={styles.fieldId}>{req.fieldId}</span>
                </div>
                <span className={`${styles.badge} ${styles[req.sensitivityLevel]}`}>
                  {req.sensitivityLevel}
                </span>
              </div>
            );
          })}
        </div>

        <div className={styles.actions}>
          <button className={styles.denyBtn} onClick={() => requests.forEach((r) => onDeny(r.requestId))}>
            {t('deny_all', { defaultValue: 'Deny All' })}
          </button>
          <button
            className={styles.approveBtn}
            onClick={() => requests.forEach((r) => onApprove(r.requestId))}
          >
            {t('approve_all', { defaultValue: 'Approve All' })}
          </button>
        </div>
      </div>
    </div>
  );
}
