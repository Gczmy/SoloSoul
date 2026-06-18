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
        border: '1px solid #ffcc80',
        background: hovered ? '#ffffff' : 'rgba(255, 255, 255, 0.85)',
        color: '#663c00',
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
