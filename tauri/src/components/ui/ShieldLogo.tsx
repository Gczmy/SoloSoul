import type { CSSProperties } from 'react';

interface ShieldLogoProps {
  size?: number;
  style?: CSSProperties;
}

export function ShieldLogo({ size = 32, style }: ShieldLogoProps) {
  const borderRadius = size <= 32 ? 8 : size <= 48 ? 12 : 16;
  return (
    <div
      style={{
        width: size,
        height: size,
        borderRadius,
        background: 'linear-gradient(135deg, var(--accent-primary), var(--accent-warm))',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        color: 'white',
        flexShrink: 0,
        ...style,
      }}
    >
      <svg
        width="62%"
        height="62%"
        viewBox="0 0 1024 1024"
        fill="none"
        xmlns="http://www.w3.org/2000/svg"
        style={{ display: 'block' }}
      >
        <g transform="matrix(0.8 0 0 0.8 128 897)">
          <path
            d="M480-80q-139-35-229.5-159.5T160-516v-244l320-120 320 120v244q0 152-90.5 276.5T480-80Zm0-84q104-33 172-132t68-220v-189l-240-90-240 90v189q0 121 68 220t172 132Zm0-316Z"
            fill="currentColor"
          />
        </g>
      </svg>
    </div>
  );
}
