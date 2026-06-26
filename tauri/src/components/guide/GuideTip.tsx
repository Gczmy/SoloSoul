import { Info, AlertTriangle, Lightbulb } from 'lucide-react';

interface GuideTipProps {
  type?: 'tip' | 'warning' | 'info';
  children: React.ReactNode;
}

const STYLES = {
  tip: {
    border: '1px solid rgba(76, 175, 80, 0.22)',
    background: 'rgba(76, 175, 80, 0.08)',
    icon: <Lightbulb size={16} style={{ color: '#5cb85c', flexShrink: 0 }} />,
  },
  warning: {
    border: '1px solid rgba(211, 84, 79, 0.22)',
    background: 'rgba(211, 84, 79, 0.08)',
    icon: <AlertTriangle size={16} style={{ color: '#d3544f', flexShrink: 0 }} />,
  },
  info: {
    border: '1px solid rgba(91, 124, 153, 0.22)',
    background: 'rgba(91, 124, 153, 0.08)',
    icon: <Info size={16} style={{ color: '#6b8fa8', flexShrink: 0 }} />,
  },
};

export function GuideTip({ type = 'info', children }: GuideTipProps) {
  const style = STYLES[type];
  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'flex-start',
        gap: 10,
        padding: '12px 14px',
        borderRadius: 10,
        border: style.border,
        background: style.background,
        margin: '12px 0',
        fontSize: 'var(--text-body)',
        lineHeight: 1.6,
        color: 'var(--text-primary)',
      }}
    >
      <div style={{ marginTop: 3, flexShrink: 0 }}>{style.icon}</div>
      <div style={{ flex: 1 }}>{children}</div>
    </div>
  );
}
