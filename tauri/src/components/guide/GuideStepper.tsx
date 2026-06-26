import { ChevronRight } from 'lucide-react';

interface GuideStepperProps {
  title?: string;
  children: React.ReactNode;
}

export function GuideStepper({ title, children }: GuideStepperProps) {
  return (
    <div
      style={{
        border: '1px solid var(--border-subtle)',
        borderRadius: 12,
        background: 'var(--bg-elevated)',
        margin: '16px 0',
      }}
    >
      {title && (
        <div
          style={{
            padding: '12px 16px',
            borderBottom: '1px solid var(--border-subtle)',
            fontSize: 'var(--text-body)',
            fontWeight: 600,
            color: 'var(--text-primary)',
            display: 'flex',
            alignItems: 'center',
            gap: 8,
          }}
        >
          <ChevronRight size={16} style={{ color: 'var(--accent-primary)' }} />
          {title}
        </div>
      )}
      <div style={{ padding: '12px 16px' }}>{children}</div>
    </div>
  );
}
