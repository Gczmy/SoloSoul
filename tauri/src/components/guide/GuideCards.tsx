import { ArrowRight } from 'lucide-react';
import { ICON_SIZE } from '@/lib/constants';

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
          className="interactive-lift-strong"
          style={{
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'flex-start',
            gap: 6,
            padding: '16px',
            borderRadius: 12,
            borderWidth: 1,
            borderStyle: 'solid',
            background: 'var(--bg-elevated)',
            color: 'var(--text-primary)',
            textAlign: 'left',
            cursor: 'pointer',
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
