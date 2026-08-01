import type { CSSProperties, ReactNode } from 'react';

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
 * 基态与 hover 颜色经内联 CSS 变量（--transfer-*）驱动 `interactive-transfer`
 * 工具类（:hover:not(:disabled)），替代手写 onMouseEnter/Leave 内联改样式；
 * busy/warning 条件态逻辑保留在组件内（busy 时 hover 变量与基值相同 → 视觉无变化）。
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

  const baseBg = busy
    ? 'color-mix(in srgb, var(--accent-primary) 10%, transparent)'
    : isWarning
      ? 'color-mix(in srgb, var(--bg-elevated) 85%, var(--warning-subtle) 15%)'
      : 'var(--bg-toolbar)';
  const baseBorder = isWarning ? 'var(--warning)' : 'var(--border-subtle)';
  const baseColor = busy
    ? 'var(--accent-primary)'
    : isWarning
      ? 'var(--warning)'
      : 'var(--text-primary)';
  // hover 仅对 enabled（非 disabled 非 busy）生效；busy 时 hover 变量等于基值
  const hoverBg = enabled
    ? isWarning
      ? 'color-mix(in srgb, var(--bg-elevated) 70%, var(--warning-subtle) 30%)'
      : 'color-mix(in srgb, var(--accent-primary) 10%, transparent)'
    : baseBg;
  const hoverBorder = enabled ? (isWarning ? 'var(--warning)' : 'var(--accent-primary)') : baseBorder;
  const hoverColor = enabled ? (isWarning ? 'var(--warning)' : 'var(--accent-primary)') : baseColor;

  const style = {
    fontSize: 'var(--text-caption)',
    padding: '6px 12px',
    borderRadius: 6,
    borderStyle: 'solid',
    borderWidth: 1,
    cursor: disabled ? 'default' : 'pointer',
    fontFamily: 'inherit',
    fontWeight: 500,
    opacity: disabled ? 0.5 : 1,
    transition: 'all 0.15s ease',
    '--transfer-bg': baseBg,
    '--transfer-border': baseBorder,
    '--transfer-color': baseColor,
    '--transfer-hover-bg': hoverBg,
    '--transfer-hover-border': hoverBorder,
    '--transfer-hover-color': hoverColor,
  } as CSSProperties;

  return (
    <button
      type="button"
      onClick={onClick}
      disabled={disabled}
      className="interactive-transfer"
      style={style}
    >
      {children}
    </button>
  );
}
