import { useState } from 'react';

interface WarningCancelButtonProps {
  onClick: () => void;
  children: string;
}

export function WarningCancelButton({ onClick, children }: WarningCancelButtonProps) {
  const [hovered, setHovered] = useState(false);
  return (
    <button
      type="button"
      onClick={onClick}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      style={{
        padding: '6px 12px',
        fontSize: 13,
        borderRadius: 6,
        border: '1px solid var(--warning)',
        background: hovered
          ? 'color-mix(in srgb, var(--bg-elevated) 70%, var(--warning-subtle) 30%)'
          : 'color-mix(in srgb, var(--bg-elevated) 85%, var(--warning-subtle) 15%)',
        color: 'var(--warning)',
        cursor: 'pointer',
        fontWeight: 500,
        transition: 'background 0.15s',
        fontFamily: 'inherit',
      }}
    >
      {children}
    </button>
  );
}
