import type { ReactNode } from 'react';

interface GuideTableProps {
  children?: ReactNode;
}

export function GuideTable({ children }: GuideTableProps) {
  return (
    <div
      style={{
        overflowX: 'auto',
        margin: '12px 0',
        border: '1px solid var(--border-subtle)',
        borderRadius: 10,
      }}
    >
      <table
        style={{
          width: '100%',
          borderCollapse: 'collapse',
          fontSize: 14,
          lineHeight: 1.5,
          color: 'var(--text-primary)',
        }}
      >
        {children}
      </table>
    </div>
  );
}
