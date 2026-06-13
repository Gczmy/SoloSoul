import { Card } from '@/components/ui/Card';
import { Cpu } from 'lucide-react';
import type { TFunction } from 'i18next';
import styles from './ModelInfoCard.module.css';

interface ModelInfoCardProps {
  providerName: string;
  modelName: string;
  apiType: string;
  isOnline?: boolean | null;
  t: TFunction;
}

export function ModelInfoCard({
  providerName,
  modelName,
  apiType,
  isOnline,
  t,
}: ModelInfoCardProps) {
  let statusLabel = t('settings:llm_status_not_loaded');
  let statusColor = 'var(--text-tertiary)';
  let bgColor = 'rgba(128,128,128,0.08)';

  if (isOnline === true) {
    statusLabel = t('settings:llm_status_ready');
    statusColor = '#27ae60';
    bgColor = 'rgba(39,174,96,0.12)';
  } else if (isOnline === false) {
    statusLabel = t('settings:llm_status_offline');
    statusColor = '#e74c3c';
    bgColor = 'rgba(231,76,60,0.12)';
  } else if (isOnline === null) {
    statusLabel = t('settings:llm_status_checking');
    statusColor = 'var(--text-tertiary)';
    bgColor = 'rgba(128,128,128,0.08)';
  }

  return (
    <Card>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <Cpu size={20} color="var(--accent-primary)" />
          <span style={{ fontSize: 14, fontWeight: 600 }}>{providerName}</span>
        </div>
        <span
          style={{
            fontSize: 11,
            fontWeight: 600,
            color: statusColor,
            background: bgColor,
            padding: '2px 8px',
            borderRadius: 8,
          }}
        >
          {statusLabel}
        </span>
      </div>
      <div style={{ marginTop: 12, display: 'flex', flexDirection: 'column', gap: 4 }}>
        <InfoRow label={t('settings:llm_info_model')} value={modelName} />
        <InfoRow label={t('settings:llm_provider_name')} value={providerName} />
        <InfoRow label={t('settings:llm_api_type')} value={apiType} />
      </div>
    </Card>
  );
}

function InfoRow({ label, value }: { label: string; value: string }) {
  return (
    <div className={styles.infoRow}>
      <span className={styles.infoLabel}>{label}</span>
      <span className={styles.infoValue} title={value}>
        {value}
      </span>
    </div>
  );
}
