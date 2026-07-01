import { memo } from 'react';
import { Card } from '@/components/ui/Card';
import { MessageSquare, Hash, Download, Clock } from 'lucide-react';
import { formatTokens, type ModelUsage } from '@/lib/llm/statsApi';
import type { TFunction } from 'i18next';
import { ICON_SIZE } from '@/lib/iconSizes';

interface StatsGridProps {
  usageCount: number;
  totalTokens: number;
  promptTokens: number;
  completionTokens: number;
  modelUsages: ModelUsage[];
  lastLoadTime?: string;
  lastUsedTime?: string;
  t: TFunction;
}

export function StatsGrid({
  usageCount,
  totalTokens,
  modelUsages,
  lastLoadTime,
  lastUsedTime,
  t,
}: StatsGridProps) {
  const tiles = [
    {
      icon: <MessageSquare size={ICON_SIZE.md} color="var(--accent-primary)" />,
      label: t('settings:llm_conversations'),
      value: usageCount.toString(),
      modelValue: (m: ModelUsage) => m.count.toString(),
    },
    {
      icon: <Hash size={ICON_SIZE.md} color="var(--accent-primary)" />,
      label: t('settings:llm_token_usage'),
      value: formatTokens(totalTokens),
      modelValue: (m: ModelUsage) => formatTokens(m.tokens),
    },
    {
      icon: <Download size={ICON_SIZE.md} color="var(--accent-primary)" />,
      label: t('settings:llm_last_load'),
      value: formatDate(lastLoadTime),
      modelValue: (m: ModelUsage) => formatDate(m.lastUsedTime),
    },
    {
      icon: <Clock size={ICON_SIZE.md} color="var(--accent-primary)" />,
      label: t('settings:llm_last_use'),
      value: formatDate(lastUsedTime),
      modelValue: (m: ModelUsage) => formatDate(m.lastUsedTime),
    },
  ];

  return (
    <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 12 }}>
      {tiles.map((tile) => (
        <StatTile key={tile.label} {...tile} modelUsages={modelUsages} />
      ))}
    </div>
  );
}

const StatTile = memo(function StatTile({
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
  const baseHeight = 84;
  const rowHeight = 16;
  const maxHeight = 140;
  const contentHeight = baseHeight + modelUsages.length * rowHeight;
  const tileHeight = contentHeight > maxHeight ? maxHeight : contentHeight;
  const needsScroll = contentHeight > maxHeight;

  return (
    <Card>
      <div
        style={{ display: 'flex', flexDirection: 'column', height: tileHeight, overflow: 'hidden' }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 2 }}>
          {icon}
          <span style={{ fontSize: 'var(--text-caption)', color: 'var(--text-tertiary)' }}>
            {label}
          </span>
        </div>
        <div
          style={{
            fontSize: 'var(--text-md)',
            fontWeight: 700,
            color: 'var(--text-primary)',
            lineHeight: 1.2,
          }}
        >
          {value}
        </div>
        {modelUsages.length > 0 && (
          <div
            style={{
              marginTop: 4,
              overflow: needsScroll ? 'auto' : 'visible',
              fontSize: 'var(--text-badge)',
              color: 'var(--text-tertiary)',
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
    </Card>
  );
});

function formatDate(iso?: string): string {
  if (!iso) return '—';
  try {
    const d = new Date(iso);
    const m = String(d.getMonth() + 1).padStart(2, '0');
    const day = String(d.getDate()).padStart(2, '0');
    const h = String(d.getHours()).padStart(2, '0');
    const min = String(d.getMinutes()).padStart(2, '0');
    return `${m}-${day} ${h}:${min}`;
  } catch {
    return '—';
  }
}
