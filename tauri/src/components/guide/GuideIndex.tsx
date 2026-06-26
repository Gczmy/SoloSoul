import { useMemo, type ReactNode } from 'react';
import type { GuideIndexEntry, GuideCategoryMeta } from '@/lib/guideApi';
import { Card } from '@/components/ui/Card';

interface GuideIndexProps {
  guides: GuideIndexEntry[];
  categories: GuideCategoryMeta[];
  language: string;
  onSelect: (guideId: string) => void;
  extraItems?: Record<string, ReactNode>;
}

function resolveTitle(title: { zh: string; en: string }, language: string): string {
  return language.startsWith('zh') ? title.zh : title.en;
}

export function GuideIndex({ guides, categories, language, onSelect, extraItems }: GuideIndexProps) {
  const grouped = useMemo(() => {
    const sortedCats = [...categories].sort((a, b) => a.order - b.order);
    return sortedCats
      .map((cat) => ({
        ...cat,
        items: guides.filter((g) => g.category === cat.id).sort((a, b) => a.order - b.order),
      }))
      .filter((g) => g.items.length > 0 || extraItems?.[g.id]);
  }, [guides, categories, extraItems]);

  if (grouped.length === 0) {
    return (
      <Card>
        <div style={{ textAlign: 'center', padding: '32px 16px', color: 'var(--text-secondary)' }}>
          <p style={{ fontSize: 'var(--text-body)' }}>{resolveTitle({ zh: '暂无帮助文档', en: 'No guides available' }, language)}</p>
        </div>
      </Card>
    );
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--card-gap-lg)' }}>
      {grouped.map((group) => (
        <div key={group.id}>
          <h3
            style={{
              fontSize: 'var(--text-body-sm)',
              fontWeight: 600,
              color: 'var(--text-secondary)',
              textTransform: 'uppercase',
              letterSpacing: '0.05em',
              marginBottom: 10,
              paddingLeft: 4,
            }}
          >
            {resolveTitle(group.title, language)}
          </h3>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--card-gap-sm)' }}>
            {group.items.map((guide) => (
              <Card key={guide.id} interactive onClick={() => onSelect(guide.id)}>
                <div
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'space-between',
                  }}
                >
                  <span style={{ fontSize: 'var(--text-body)', fontWeight: 500 }}>
                    {resolveTitle(guide.title, language)}
                  </span>
                  <span
                    style={{
                      color: 'var(--text-tertiary)',
                      fontSize: 'var(--text-md)',
                    }}
                  >
                    ›
                  </span>
                </div>
              </Card>
            ))}
            {extraItems?.[group.id]}
          </div>
        </div>
      ))}
    </div>
  );
}
