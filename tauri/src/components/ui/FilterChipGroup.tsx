import type { ButtonHTMLAttributes, CSSProperties, ReactNode } from 'react';
import styles from './FilterChipGroup.module.css';

/** 单选格式与多选标签共用的选择按钮；选中状态同时提供给辅助技术和平台样式。 */
export function FilterChip({
  selected,
  children,
  className = '',
  style,
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement> & { selected: boolean }) {
  return (
    <button
      {...props}
      type="button"
      data-ui-choice
      aria-pressed={selected}
      className={`interactive-toolbar ${styles.choice} ${className}`}
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        justifyContent: 'center',
        gap: 6,
        maxWidth: '100%',
        padding: '5px 12px',
        borderRadius: 6,
        borderWidth: 1,
        borderStyle: 'solid',
        fontSize: 'var(--text-body-sm)',
        fontWeight: 500,
        fontFamily: 'inherit',
        whiteSpace: 'normal',
        overflowWrap: 'anywhere',
        cursor: 'pointer',
        ...style,
      }}
    >
      {children}
    </button>
  );
}

export interface FilterChipOption<T extends string = string> {
  /** 选项值；null 表示「全部」类选项（如 OperationLog 页的 all 筛选） */
  id: T | null;
  label: ReactNode;
  /** 可选透传给按钮的 data-testid（测试依赖场景，如 page-filter-travel） */
  testId?: string;
}

interface FilterChipGroupProps<T extends string = string> {
  options: FilterChipOption<T>[];
  value: T | null;
  onChange: (id: T | null) => void;
  /** 点击已激活项时取消选中（onChange(null)），OperationLog 页的筛选语义 */
  toggle?: boolean;
  /** 字号档：sm = var(--text-sm)，caption = var(--text-caption) */
  size?: 'sm' | 'caption';
  /** 圆角，默认 6 */
  radius?: number;
  /** 项间距，默认 6 */
  gap?: number;
  /** 字重，默认 500 */
  fontWeight?: number;
  /** 容器额外样式（可覆盖 display/flexWrap 等） */
  style?: CSSProperties;
}

/**
 * P049: 统一筛选 chip 按钮组。原 5 处手写「isActive 三态 style + hover 双事件 + map」
 * 重复块收敛于此。激活态使用描边与淡色底，文字保持易读的主题前景。
 */
export function FilterChipGroup<T extends string = string>({
  options,
  value,
  onChange,
  toggle = false,
  size = 'sm',
  radius = 6,
  gap = 6,
  fontWeight = 500,
  style,
}: FilterChipGroupProps<T>) {
  return (
    <div style={{ display: 'flex', gap, flexWrap: 'wrap', ...style }}>
      {options.map((opt) => {
        const isActive = value === opt.id;
        return (
          <FilterChip
            key={opt.id ?? 'all'}
            selected={isActive}
            data-testid={opt.testId}
            onClick={() => {
              if (toggle && isActive) {
                onChange(null);
              } else {
                onChange(opt.id);
              }
            }}
            style={{
              borderRadius: radius,
              fontSize: size === 'caption' ? 'var(--text-caption)' : 'var(--text-sm)',
              fontWeight,
            }}
          >
            {opt.label}
          </FilterChip>
        );
      })}
    </div>
  );
}
