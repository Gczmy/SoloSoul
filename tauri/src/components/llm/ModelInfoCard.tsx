import { Card } from '@/components/ui/Card';
import { Cpu } from 'lucide-react';

interface ModelInfoCardProps {
  providerName: string;
  modelName: string;
  providerType: string;
  isOnline?: boolean | null;
}

export function ModelInfoCard({ providerName, modelName, providerType, isOnline }: ModelInfoCardProps) {
  let statusLabel = '未加载';
  let statusColor = 'var(--text-tertiary)';
  let bgColor = 'rgba(128,128,128,0.08)';

  if (isOnline === true) {
    statusLabel = '就绪';
    statusColor = '#27ae60';
    bgColor = 'rgba(39,174,96,0.12)';
  } else if (isOnline === false) {
    statusLabel = '离线';
    statusColor = '#e74c3c';
    bgColor = 'rgba(231,76,60,0.12)';
  } else if (isOnline === null) {
    statusLabel = '检测中';
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
        <InfoRow label="模型" value={modelName} />
        <InfoRow label="Provider" value={providerType} />
      </div>
    </Card>
  );
}

function InfoRow({ label, value }: { label: string; value: string }) {
  return (
    <div style={{ display: 'flex', fontSize: 13 }}>
      <span style={{ color: 'var(--text-tertiary)', minWidth: 60 }}>{label}</span>
      <span style={{ color: 'var(--text-primary)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
        {value}
      </span>
    </div>
  );
}
