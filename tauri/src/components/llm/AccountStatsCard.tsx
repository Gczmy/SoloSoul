import { memo } from 'react';
import { Card } from '@/components/ui/Card';
import { MessageSquare, Hash } from 'lucide-react';
import { formatTokens, type ModelUsage } from '@/lib/llm/statsApi';
import type { TFunction } from 'i18next';

interface AccountStatsCardProps {
  usageCount: number;
  totalTokens: number;
  modelUsages: ModelUsage[];
  t: TFunction;
}

export function AccountStatsCard({
  usageCount,
  totalTokens,
  modelUsages,
  t,
}: AccountStatsCardProps) {
  return (
    <Card>
      <div style={{ display: 'flex' }}>
        <StatColumn
          icon={<MessageSquare size={20} color="var(--accent-primary)" />}
          label={t('settings:llm_total_conversations')}
          value={usageCount.toString()}
          modelUsages={modelUsages}
          modelValue={(m) => m.count.toString()}
        />
        <div
          style={{ width: 1, background: 'var(--border-subtle)', margin: '0 12px', flexShrink: 0 }}
        />
        <StatColumn
          icon={<Hash size={20} color="var(--accent-primary)" />}
          label={t('settings:llm_total_tokens')}
          value={formatTokens(totalTokens)}
          modelUsages={modelUsages}
          modelValue={(m) => formatTokens(m.tokens)}
        />
      </div>
    </Card>
  );
}

const StatColumn = memo(function StatColumn({
  icon,
  label,
  value,
  modelUsages,
  modelValue,
}: {
  icon: React.ReactNode;
  label: string;
  value: string;
  modelUsages: ModelUsage[];
  modelValue: (m: ModelUsage) => string;
}) {
  const baseHeight = 92;
  const rowHeight = 16;
  const maxHeight = 140;
  const contentHeight = baseHeight + modelUsages.length * rowHeight;
  const tileHeight = contentHeight > maxHeight ? maxHeight : contentHeight;
  const needsScroll = contentHeight > maxHeight;

  return (
    <div
      style={{
        flex: 1,
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        height: tileHeight,
        overflow: 'hidden',
      }}
    >
      <div style={{ marginBottom: 4 }}>{icon}</div>
      <div
        style={{ fontSize: 'var(--text-page-title)', fontWeight: 700, color: 'var(--accent-primary)', lineHeight: 1.2 }}
      >
        {value}
      </div>
      <div style={{ fontSize: 'var(--text-caption)', color: 'var(--text-tertiary)', marginBottom: 4 }}>{label}</div>
      {modelUsages.length > 0 && (
        <div
          style={{
            width: '100%',
            overflow: needsScroll ? 'auto' : 'visible',
            fontSize: 'var(--text-badge)',
            color: 'var(--text-tertiary)',
            textAlign: 'center',
          }}
        >
          {modelUsages.map((m) => (
            <div
              key={`${m.provider}/${m.model}`}
              style={{ whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}
            >
              {m.model} · {m.provider} · {modelValue(m)}
            </div>
          ))}
        </div>
      )}
    </div>
  );
});
