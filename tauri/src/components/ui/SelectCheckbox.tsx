import React, { memo, useMemo } from 'react';

interface SelectCheckboxProps {
  checked: boolean;
  /** Click handler. If omitted, the click event will bubble naturally to parent elements. */
  onClick?: (e: React.MouseEvent) => void;
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
}

/**
 * A small checkbox component used for batch selection of rows and form toggles.
 * Uses the project's accent color when checked with an SVG checkmark.
 *
 * Default size/radius matches the GlobalAttachmentManager attachment rows.
 */
export const SelectCheckbox = memo(function SelectCheckbox({
  checked,
  onClick,
  onChange,
  size = 14,
  borderRadius = 3,
  indeterminate = false,
  disabled = false,
}: SelectCheckboxProps) {
  const markSize = useMemo(() => Math.max(6, Math.round(size * 0.55)), [size]);
  const isActive = checked || indeterminate;

  const handleClick = (e: React.MouseEvent) => {
    if (disabled) return;
    onChange?.(!checked);
    onClick?.(e);
  };

  return (
    <div
      data-testid="select-checkbox"
      onClick={handleClick}
      style={{
        width: size,
        height: size,
        borderRadius,
        border: isActive ? 'none' : `1.5px solid var(--accent-primary)`,
        background: isActive ? 'var(--accent-primary)' : 'transparent',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        cursor: disabled ? 'default' : 'pointer',
        flexShrink: 0,
        boxSizing: 'border-box',
        opacity: disabled ? 0.5 : 1,
        transition: 'all 0.15s ease',
      }}
      role="checkbox"
      aria-checked={indeterminate ? 'mixed' : checked}
      aria-disabled={disabled}
    >
      {checked && !indeterminate && (
        <svg
          width={markSize}
          height={markSize}
          viewBox="0 0 24 24"
          fill="none"
          stroke="white"
          strokeWidth="3"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          <polyline points="20 6 9 17 4 12" />
        </svg>
      )}
      {indeterminate && (
        <div
          style={{
            width: markSize,
            height: 2,
            borderRadius: 1,
            background: 'white',
          }}
        />
      )}
    </div>
  );
});
