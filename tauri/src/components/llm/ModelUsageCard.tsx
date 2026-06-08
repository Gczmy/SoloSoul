import { Card } from '@/components/ui/Card';
import { formatTokens, type ModelUsage } from '@/lib/llm/statsApi';

interface ModelUsageCardProps {
  perModel: ModelUsage[];
}

export function ModelUsageCard({ perModel }: ModelUsageCardProps) {
  const totalTokens = perModel.reduce((sum, m) => sum + m.tokens, 0);

  return (
    <Card>
      <div style={{ fontSize: 12, color: 'var(--text-tertiary)', marginBottom: 12 }}>
        共 {perModel.length} 个模型 · 累计 {formatTokens(totalTokens)} token
      </div>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
        {perModel.map((m) => {
          const ratio = totalTokens === 0 ? 0 : m.tokens / totalTokens;
          return (
            <div key={`${m.provider}/${m.model}`}>
              <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 4 }}>
                <span style={{ fontSize: 13, fontWeight: 600, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                  {m.model}
                </span>
                <span style={{ fontSize: 12, color: 'var(--text-tertiary)', flexShrink: 0, marginLeft: 8 }}>
                  {(ratio * 100).toFixed(1)}%
                </span>
              </div>
              <div style={{ height: 6, borderRadius: 3, background: 'var(--bg-toolbar)', overflow: 'hidden', marginBottom: 2 }}>
                <div
                  style={{
                    width: `${ratio * 100}%`,
                    height: '100%',
                    background: 'var(--accent-primary)',
                    borderRadius: 3,
                    transition: 'width 0.3s ease',
                  }}
                />
              </div>
              <div style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>
                {m.provider} · {formatTokens(m.tokens)} · {m.count} 次调用
              </div>
            </div>
          );
        })}
      </div>
    </Card>
  );
}
