import { useNavigate } from 'react-router-dom';
import React from 'react';
import { useTranslation } from 'react-i18next';
import { PAGE_ICON_MAP, resolveCustomIcon } from '@/lib/pageIcons';
import type { CustomPage } from '@/stores/settingsStore';

const CATEGORY_TYPES = ['identity', 'travel', 'financial', 'professional', 'document'] as const;
const CATEGORY_ICONS: Record<string, typeof PAGE_ICON_MAP.profile> = {
  identity: PAGE_ICON_MAP.profile,
  travel: PAGE_ICON_MAP.travel,
  financial: PAGE_ICON_MAP.financial,
  professional: PAGE_ICON_MAP.professional,
  document: PAGE_ICON_MAP.document,
};

interface WorkspaceCategoryTabsProps {
  sectionFilter: string;
  pageId?: string;
  customPages: CustomPage[];
  activeCustomPages: CustomPage[];
  className?: string;
}

export function WorkspaceCategoryTabs({
  sectionFilter,
  pageId,
  customPages: _customPages,
  activeCustomPages,
  className,
}: WorkspaceCategoryTabsProps) {
  const navigate = useNavigate();
  const { t } = useTranslation(['common', 'navigation', 'editor']);

  const tabStyle = (isActive: boolean): React.CSSProperties => ({
    padding: '6px 14px',
    borderRadius: 8,
    borderWidth: 1,
    borderStyle: 'solid',
    // 外发光 ring 会被 overflow 容器（移动端 .tabs 的横向滚动、页面滚动区）截断，
    // 导致第一行/最左选项边框粗细不一；inset 内描边永远画在元素内部，不会被裁剪
    boxShadow: isActive ? 'inset 0 0 0 1px var(--accent-primary)' : 'none',
    fontSize: 'var(--text-body-sm)',
    cursor: 'pointer',
    display: 'flex',
    alignItems: 'center',
    gap: 4,
  });

  return (
    <div className={className} style={{ display: 'flex', gap: 8, flexWrap: 'wrap' }}>
      {CATEGORY_TYPES.map((catType) => {
        const isActive = !pageId && sectionFilter === catType;
        return (
          <button
            key={catType}
            data-active={isActive ? 'true' : 'false'}
            onClick={() => navigate(`/workspace?section=${catType}`)}
            className={`interactive-toolbar ${isActive ? 'selected-accent' : ''}`}
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
            className={`interactive-toolbar ${isActive ? 'selected-accent' : ''}`}
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
          className="interactive-clear"
          style={{
            padding: '6px 14px',
            borderRadius: 8,
            borderWidth: 1,
            borderStyle: 'solid',
            fontSize: 'var(--text-body-sm)',
            cursor: 'pointer',
          }}
        >
          {t('clear')}
        </button>
      )}
    </div>
  );
}
