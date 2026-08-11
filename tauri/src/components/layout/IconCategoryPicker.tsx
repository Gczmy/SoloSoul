import { useTranslation } from 'react-i18next';
import type { LucideIcon } from 'lucide-react';
import {
  CUSTOM_ICON_MAP,
  ICON_CATEGORIES,
  CATEGORY_LABELS,
  ICON_CATEGORY_ORDER,
  type CustomIconId,
} from '@/lib/pageIcons';
import { ICON_SIZE } from '@/lib/constants';
import styles from './SideNavigation.module.css';

interface IconCategoryPickerProps {
  selectedIconId: CustomIconId;
  onSelect: (id: CustomIconId) => void;
  /** 'module'：CSS module 按钮（AddPageButton 弹出面板）；'inline'：内联瓦片（CustomPageEditPopover） */
  variant?: 'module' | 'inline';
  iconSize?: number;
}

/**
 * P011: 图标分类选择网格（AddPageButton / CustomPageEditPopover 原本各复制一份）。
 * 按分类顺序渲染「分类名 + 图标网格」，点击由调用方通过 onSelect 决定后续行为
 * （保持面板打开 / 关闭选择器）。
 */
export function IconCategoryPicker({
  selectedIconId,
  onSelect,
  variant = 'module',
  iconSize,
}: IconCategoryPickerProps) {
  const { t } = useTranslation();
  const size = iconSize ?? (variant === 'inline' ? ICON_SIZE.lg : ICON_SIZE.md);

  return (
    <>
      {ICON_CATEGORY_ORDER.map((cat) => {
        const categoryIcons = (
          Object.entries(CUSTOM_ICON_MAP) as [CustomIconId, LucideIcon][]
        ).filter(([id]) => ICON_CATEGORIES[id] === cat);
        if (categoryIcons.length === 0) return null;
        return (
          <div key={cat}>
            <div
              style={{
                fontSize: 'var(--text-badge)',
                fontWeight: 500,
                color: 'var(--text-tertiary)',
                padding: '2px 0 4px',
                borderBottom: '1px solid var(--border-subtle)',
                marginBottom: 4,
              }}
            >
              {t(`navigation:icon_category_${cat}`, CATEGORY_LABELS[cat])}
            </div>
            <div
              style={{
                display: 'grid',
                gridTemplateColumns: 'repeat(6, 1fr)',
                gap: 4,
              }}
            >
              {categoryIcons.map(([id, IconComp]) =>
                variant === 'module' ? (
                  <button
                    key={id}
                    onMouseDown={(e) => e.preventDefault()}
                    onClick={() => onSelect(id)}
                    className={`${styles.iconPickerBtn} ${
                      id === selectedIconId ? styles.iconPickerBtnSelected : ''
                    }`}
                    title={id}
                    aria-label={id}
                  >
                    <IconComp
                      size={size}
                      style={{
                        color:
                          id === selectedIconId
                            ? 'var(--accent-primary)'
                            : 'var(--text-secondary)',
                      }}
                    />
                  </button>
                ) : (
                  <button
                    key={id}
                    onClick={() => onSelect(id)}
                    className={
                      selectedIconId === id
                        ? 'interactive-tile selected-accent'
                        : 'interactive-tile'
                    }
                    style={{
                      width: 32,
                      height: 32,
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      borderRadius: 6,
                      borderWidth: selectedIconId === id ? 2 : 1,
                      borderStyle: 'solid',
                      cursor: 'pointer',
                    }}
                  >
                    <IconComp
                      size={size}
                      style={{
                        color:
                          selectedIconId === id
                            ? 'var(--accent-primary)'
                            : 'var(--text-secondary)',
                      }}
                    />
                  </button>
                ),
              )}
            </div>
          </div>
        );
      })}
    </>
  );
}
