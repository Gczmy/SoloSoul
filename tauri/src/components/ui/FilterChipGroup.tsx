import type { CSSProperties, ReactNode } from 'react';

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
 * 重复块收敛于此。激活态：accent 边框 + 淡色底 + 阴影；非激活 hover：accent 描边预览。
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
          <button
            key={opt.id ?? 'all'}
            type="button"
            aria-pressed={isActive}
            data-testid={opt.testId}
            onClick={() => {
              if (toggle && isActive) {
                onChange(null);
              } else {
                onChange(opt.id);
              }
            }}
            onMouseEnter={(e) => {
              if (!isActive) {
                e.currentTarget.style.background =
                  'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
                e.currentTarget.style.borderColor = 'var(--accent-primary)';
              }
            }}
            onMouseLeave={(e) => {
              if (!isActive) {
                e.currentTarget.style.background = 'var(--bg-toolbar)';
                e.currentTarget.style.borderColor = 'var(--border-subtle)';
              }
            }}
            style={{
              padding: '5px 12px',
              borderRadius: radius,
              border: isActive
                ? '1px solid var(--accent-primary)'
                : '1px solid var(--border-subtle)',
              background: isActive
                ? 'color-mix(in srgb, var(--accent-primary) 10%, transparent)'
                : 'var(--bg-toolbar)',
              color: isActive ? 'var(--accent-primary)' : 'var(--text-primary)',
              boxShadow: isActive ? '0 0 0 1px var(--accent-primary)' : 'none',
              fontSize: size === 'caption' ? 'var(--text-caption)' : 'var(--text-sm)',
              fontWeight,
              cursor: 'pointer',
              transition: 'background 0.2s, border-color 0.2s, color 0.2s, box-shadow 0.2s',
            }}
          >
            {opt.label}
          </button>
        );
      })}
    </div>
  );
}
