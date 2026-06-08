import { Info, AlertTriangle, Lightbulb } from 'lucide-react';

interface GuideTipProps {
  type?: 'tip' | 'warning' | 'info';
  children: React.ReactNode;
}

const STYLES = {
  tip: {
    border: '1px solid rgba(39, 174, 96, 0.3)',
    background: 'rgba(39, 174, 96, 0.06)',
    icon: <Lightbulb size={16} style={{ color: '#27ae60', flexShrink: 0 }} />,
  },
  warning: {
    border: '1px solid rgba(231, 76, 60, 0.3)',
    background: 'rgba(231, 76, 60, 0.06)',
    icon: <AlertTriangle size={16} style={{ color: '#e74c3c', flexShrink: 0 }} />,
  },
  info: {
    border: '1px solid rgba(91, 124, 153, 0.3)',
    background: 'rgba(91, 124, 153, 0.06)',
    icon: <Info size={16} style={{ color: 'var(--accent-primary)', flexShrink: 0 }} />,
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
        fontSize: 14,
        lineHeight: 1.6,
        color: 'var(--text-primary)',
      }}
    >
      {style.icon}
      <div style={{ flex: 1 }}>{children}</div>
    </div>
  );
}
