import type { ReactNode } from 'react';

interface TransferButtonProps {
  onClick: () => void;
  disabled?: boolean;
  /** 忙碌/进行中：使用 accent 高亮态 */
  busy?: boolean;
  variant?: 'plain' | 'accent' | 'warning';
  children: ReactNode;
}

/**
 * 导出/导入共用按钮：统一 plain/accent/warning 三态样式与 hover 微交互。
 * 替代两处组件中重复的手写 hover（onMouseEnter/Leave 内联改样式）。
 */
export function TransferButton({
  onClick,
  disabled = false,
  busy = false,
  variant = 'plain',
  children,
}: TransferButtonProps) {
  const isWarning = variant === 'warning';
  const enabled = !disabled && !busy;

  const restStyle = {
    fontSize: 'var(--text-caption)',
    padding: '6px 12px',
    borderRadius: 6,
    border: `1px solid ${isWarning ? 'var(--warning)' : 'var(--border-subtle)'}`,
    background: busy
      ? 'color-mix(in srgb, var(--accent-primary) 10%, transparent)'
      : isWarning
        ? 'color-mix(in srgb, var(--bg-elevated) 85%, var(--warning-subtle) 15%)'
        : 'var(--bg-toolbar)',
    color: busy
      ? 'var(--accent-primary)'
      : isWarning
        ? 'var(--warning)'
        : 'var(--text-primary)',
    cursor: disabled ? 'default' : 'pointer',
    fontFamily: 'inherit',
    fontWeight: 500,
    opacity: disabled ? 0.5 : 1,
    transition: 'all 0.15s ease',
  } as const;

  return (
    <button
      type="button"
      onClick={onClick}
      disabled={disabled}
      style={restStyle}
      onMouseEnter={(e) => {
        if (!enabled) return;
        e.currentTarget.style.background = isWarning
          ? 'color-mix(in srgb, var(--bg-elevated) 70%, var(--warning-subtle) 30%)'
          : 'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
        e.currentTarget.style.borderColor = isWarning ? 'var(--warning)' : 'var(--accent-primary)';
        e.currentTarget.style.color = isWarning ? 'var(--warning)' : 'var(--accent-primary)';
      }}
      onMouseLeave={(e) => {
        e.currentTarget.style.background = busy
          ? 'color-mix(in srgb, var(--accent-primary) 10%, transparent)'
          : isWarning
            ? 'color-mix(in srgb, var(--bg-elevated) 85%, var(--warning-subtle) 15%)'
            : 'var(--bg-toolbar)';
        e.currentTarget.style.borderColor = isWarning ? 'var(--warning)' : 'var(--border-subtle)';
        e.currentTarget.style.color = busy
          ? 'var(--accent-primary)'
          : isWarning
            ? 'var(--warning)'
            : 'var(--text-primary)';
      }}
    >
      {children}
    </button>
  );
}
