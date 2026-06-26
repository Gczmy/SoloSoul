import React, { memo } from 'react';

interface SelectCheckboxProps {
  checked: boolean;
  /** Click handler. If omitted, the click event will bubble naturally to parent elements. */
  onClick?: (e: React.MouseEvent) => void;
  /** Size in pixels. Default 16. */
  size?: number;
  /** Border radius in pixels. Default 4. */
  borderRadius?: number;
}

/**
 * A small checkbox component used for batch selection of rows.
 * Uses the project's accent color when checked with an SVG checkmark.
 */
export const SelectCheckbox = memo(function SelectCheckbox({
  checked,
  onClick,
  size = 16,
  borderRadius = 4,
}: SelectCheckboxProps) {
  return (
    <div
      onClick={onClick ?? undefined}
      style={{
        width: size,
        height: size,
        borderRadius,
        border: checked ? 'none' : `1px solid var(--border-subtle)`,
        background: checked ? 'var(--accent-primary)' : 'transparent',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        cursor: 'pointer',
        flexShrink: 0,
        transition: 'all 0.15s ease',
      }}
    >
      {checked && (
        <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="white" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round">
          <polyline points="20 6 9 17 4 12" />
        </svg>
      )}
    </div>
  );
});
