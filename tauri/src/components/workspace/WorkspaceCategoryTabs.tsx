import { useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import React from 'react';
import { useTranslation } from 'react-i18next';
import { PAGE_ICON_MAP, resolveCustomIcon } from '@/lib/pageIcons';
import type { CustomPage } from '@/stores/settingsStore';

const CATEGORY_TYPES = ['identity', 'travel', 'financial', 'professional'] as const;
const CATEGORY_ICONS: Record<string, typeof PAGE_ICON_MAP.profile> = {
  identity: PAGE_ICON_MAP.profile,
  travel: PAGE_ICON_MAP.travel,
  financial: PAGE_ICON_MAP.financial,
  professional: PAGE_ICON_MAP.professional,
};

interface WorkspaceCategoryTabsProps {
  sectionFilter: string;
  pageId?: string;
  customPages: CustomPage[];
  activeCustomPages: CustomPage[];
}

export function WorkspaceCategoryTabs({
  sectionFilter,
  pageId,
  customPages,
  activeCustomPages,
}: WorkspaceCategoryTabsProps) {
  const navigate = useNavigate();
  const { t } = useTranslation(['common', 'navigation', 'editor']);

  const onTabEnter = useCallback((e: React.MouseEvent<HTMLButtonElement>) => {
    if (e.currentTarget.dataset.active === 'true') return;
    e.currentTarget.style.background =
      'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
    e.currentTarget.style.borderColor = 'var(--accent-primary)';
  }, []);
  const onTabLeave = useCallback((e: React.MouseEvent<HTMLButtonElement>) => {
    if (e.currentTarget.dataset.active === 'true') return;
    e.currentTarget.style.background = 'var(--bg-toolbar)';
    e.currentTarget.style.borderColor = 'var(--border-subtle)';
  }, []);
  const onClearEnter = useCallback((e: React.MouseEvent<HTMLButtonElement>) => {
    e.currentTarget.style.borderColor = 'var(--accent-primary)';
    e.currentTarget.style.color = 'var(--text-primary)';
    e.currentTarget.style.boxShadow =
      '0 0 0 2px color-mix(in srgb, var(--accent-primary) 10%, transparent)';
  }, []);
  const onClearLeave = useCallback((e: React.MouseEvent<HTMLButtonElement>) => {
    e.currentTarget.style.borderColor = 'var(--border-subtle)';
    e.currentTarget.style.color = 'var(--text-tertiary)';
    e.currentTarget.style.boxShadow = 'none';
  }, []);

  const tabStyle = (isActive: boolean): React.CSSProperties => ({
    padding: '6px 14px',
    borderRadius: 8,
    border: isActive ? '1px solid var(--accent-primary)' : '1px solid var(--border-subtle)',
    background: isActive
      ? 'color-mix(in srgb, var(--accent-primary) 10%, transparent)'
      : 'var(--bg-toolbar)',
    color: isActive ? 'var(--accent-primary)' : 'var(--text-primary)',
    boxShadow: isActive ? '0 0 0 1px var(--accent-primary)' : 'none',
    fontSize: 13,
    cursor: 'pointer',
    display: 'flex',
    alignItems: 'center',
    gap: 4,
    transition: 'background 0.2s, border-color 0.2s, color 0.2s, box-shadow 0.2s',
  });

  return (
    <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap' }}>
      {CATEGORY_TYPES.map((catType) => {
        const isActive = !pageId && sectionFilter === catType;
        return (
          <button
            key={catType}
            data-active={isActive ? 'true' : 'false'}
            onClick={() => navigate(`/workspace?section=${catType}`)}
            onMouseEnter={onTabEnter}
            onMouseLeave={onTabLeave}
            style={tabStyle(isActive)}
          >
            {React.createElement(CATEGORY_ICONS[catType], { size: 16 })}
            {t(`navigation:${catType}`, catType)}
          </button>
        );
      })}
      {activeCustomPages.map((page) => {
        const isActive = pageId === page.id;
        return (
          <button
            key={page.id}
            data-active={isActive ? 'true' : 'false'}
            onClick={() => navigate(`/workspace/custom/${page.id}`)}
            onMouseEnter={onTabEnter}
            onMouseLeave={onTabLeave}
            style={tabStyle(isActive)}
          >
            {React.createElement(resolveCustomIcon(page.iconId), { size: 16 })}
            {page.name}
          </button>
        );
      })}
      {(sectionFilter || pageId) && (
        <button
          onClick={() => navigate('/workspace')}
          onMouseEnter={onClearEnter}
          onMouseLeave={onClearLeave}
          style={{
            padding: '6px 14px',
            borderRadius: 8,
            border: '1px solid var(--border-subtle)',
            background: 'transparent',
            color: 'var(--text-tertiary)',
            fontSize: 13,
            cursor: 'pointer',
            transition: 'border-color 0.2s, box-shadow 0.2s, color 0.2s',
          }}
        >
          {t('clear')}
        </button>
      )}
    </div>
  );
}
