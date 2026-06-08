import { Card } from '@/components/ui/Card';
import { MessageSquare, Hash, Download, Clock } from 'lucide-react';
import { formatTokens, type ModelUsage } from '@/lib/llm/statsApi';

interface StatsGridProps {
  usageCount: number;
  totalTokens: number;
  promptTokens: number;
  completionTokens: number;
  modelUsages: ModelUsage[];
  lastLoadTime?: string;
  lastUsedTime?: string;
}

export function StatsGrid({
  usageCount,
  totalTokens,
  modelUsages,
  lastLoadTime,
  lastUsedTime,
}: StatsGridProps) {
  const tiles = [
    {
      icon: <MessageSquare size={16} color="var(--accent-primary)" />,
      label: '对话数',
      value: usageCount.toString(),
      modelValue: (m: ModelUsage) => m.count.toString(),
    },
    {
      icon: <Hash size={16} color="var(--accent-primary)" />,
      label: 'Token 消耗',
      value: formatTokens(totalTokens),
      modelValue: (m: ModelUsage) => formatTokens(m.tokens),
    },
    {
      icon: <Download size={16} color="var(--accent-primary)" />,
      label: '最后加载',
      value: formatDate(lastLoadTime),
      modelValue: (m: ModelUsage) => formatDate(m.lastUsedTime),
    },
    {
      icon: <Clock size={16} color="var(--accent-primary)" />,
      label: '最后使用',
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

function StatTile({
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
      <div style={{ display: 'flex', flexDirection: 'column', height: tileHeight, overflow: 'hidden' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 2 }}>
          {icon}
          <span style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>{label}</span>
        </div>
        <div style={{ fontSize: 18, fontWeight: 700, color: 'var(--text-primary)', lineHeight: 1.2 }}>{value}</div>
        {modelUsages.length > 0 && (
          <div style={{ marginTop: 4, overflow: needsScroll ? 'auto' : 'visible', fontSize: 10, color: 'var(--text-tertiary)' }}>
            {modelUsages.map((m) => (
              <div key={`${m.provider}/${m.model}`} style={{ whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
                {m.model} · {m.provider} · {modelValue(m)}
              </div>
            ))}
          </div>
        )}
      </div>
    </Card>
  );
}

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
