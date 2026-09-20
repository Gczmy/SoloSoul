import { memo, useEffect, useRef, type CSSProperties, type MouseEvent } from 'react';
import styles from './SelectCheckbox.module.css';

interface SelectCheckboxProps {
  checked: boolean;
  /** 兼容旧的行选择回调；新调用优先使用 onChange。 */
  onClick?: (e: MouseEvent) => void;
  /** Boolean change handler. Preferred for form-like usage. */
  onChange?: (checked: boolean) => void;
  /** Size in pixels. Default 14 (matches GlobalAttachmentManager). */
  size?: number;
  /** Border radius in pixels. Default 3. */
  borderRadius?: number;
  /** Visual indeterminate state (rendered as a horizontal dash). */
  indeterminate?: boolean;
  /** Disable interaction and dim the checkbox. */
  disabled?: boolean;
  /** 未包裹在文本 label 中时必须提供可访问名称。 */
  'aria-label'?: string;
  'aria-labelledby'?: string;
  'aria-describedby'?: string;
  id?: string;
  name?: string;
}

/**
 * 原生复选框负责 label、键盘和混合状态；视觉尺寸与触控目标独立。
 * Android 使用 20px 标记 / 48px 触控目标，iOS 使用 44px 触控目标。
 */
export const SelectCheckbox = memo(function SelectCheckbox({
  checked,
  onClick,
  onChange,
  size = 14,
  borderRadius = 3,
  indeterminate = false,
  disabled = false,
  'aria-label': ariaLabel,
  'aria-labelledby': ariaLabelledBy,
  'aria-describedby': ariaDescribedBy,
  id,
  name,
}: SelectCheckboxProps) {
  const inputRef = useRef<HTMLInputElement>(null);
  useEffect(() => {
    if (inputRef.current) inputRef.current.indeterminate = indeterminate;
  }, [checked, indeterminate]);

  const handleClick = (e: MouseEvent) => {
    // 有自己的回调时，不再触发外层行的选择/导航；disabled 也不能借父行切换。
    if (disabled || onChange || onClick) e.stopPropagation();
    if (disabled) return;
    onClick?.(e);
  };

  return (
    <span
      className={styles.control}
      data-ui-checkbox
      onClick={handleClick}
      style={
        {
          '--checkbox-size': `${size}px`,
          '--checkbox-radius': `${borderRadius}px`,
        } as CSSProperties
      }
    >
      <input
        ref={inputRef}
        id={id}
        name={name}
        type="checkbox"
        data-testid="select-checkbox"
        className={styles.input}
        checked={checked}
        disabled={disabled}
        readOnly={!onChange}
        aria-label={ariaLabel}
        aria-labelledby={ariaLabelledBy}
        aria-describedby={ariaDescribedBy}
        aria-checked={indeterminate ? 'mixed' : checked}
        onChange={(event) => {
          if (!disabled) onChange?.(event.target.checked);
        }}
        onKeyDown={(event) => {
          if (event.key === ' ' || event.key === 'Enter') event.stopPropagation();
        }}
      />
      <span className={styles.visual} aria-hidden="true" data-checkbox-visual>
        <svg
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="3"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          {indeterminate ? <path d="M5 12h14" /> : <polyline points="20 6 9 17 4 12" />}
        </svg>
      </span>
    </span>
  );
});
