import { ArrowRight } from 'lucide-react';
import { ICON_SIZE } from '@/lib/iconSizes';

export interface GuideCardItem {
  title: string;
  href: string;
  desc: string;
}

interface GuideCardsProps {
  items: GuideCardItem[];
  onLinkClick?: (href: string) => void;
}

export function GuideCards({ items, onLinkClick }: GuideCardsProps) {
  return (
    <div
      style={{
        display: 'grid',
        gridTemplateColumns: 'repeat(auto-fill, minmax(220px, 1fr))',
        gap: 12,
        margin: '16px 0',
      }}
    >
      {items.map((item) => (
        <button
          key={item.href}
          onClick={() => onLinkClick?.(item.href)}
          style={{
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'flex-start',
            gap: 6,
            padding: '16px',
            borderRadius: 12,
            border: '1px solid var(--border-subtle)',
            background: 'var(--bg-elevated)',
            color: 'var(--text-primary)',
            textAlign: 'left',
            cursor: 'pointer',
            transition: 'transform 0.2s ease, box-shadow 0.2s ease, border-color 0.2s ease',
            boxShadow: '0 1px 2px rgba(0,0,0,0.04)',
          }}
          onMouseEnter={(e) => {
            e.currentTarget.style.transform = 'translateY(-3px)';
            e.currentTarget.style.boxShadow = '0 8px 20px rgba(0,0,0,0.08)';
            e.currentTarget.style.borderColor = 'var(--accent-primary)';
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.transform = 'translateY(0)';
            e.currentTarget.style.boxShadow = '0 1px 2px rgba(0,0,0,0.04)';
            e.currentTarget.style.borderColor = 'var(--border-subtle)';
          }}
        >
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between',
              width: '100%',
              fontSize: 'var(--text-card-title)',
              fontWeight: 600,
              color: 'var(--text-primary)',
            }}
          >
            <span>{item.title}</span>
            <ArrowRight
              size={ICON_SIZE.md}
              style={{ color: 'var(--accent-primary)', flexShrink: 0 }}
            />
          </div>
          <div
            style={{
              fontSize: 'var(--text-body-sm)',
              color: 'var(--text-secondary)',
              lineHeight: 1.5,
            }}
          >
            {item.desc}
          </div>
        </button>
      ))}
    </div>
  );
}
