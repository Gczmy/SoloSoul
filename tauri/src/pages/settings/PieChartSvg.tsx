import { memo } from 'react';
import { formatBytes } from '@/lib/utils';

// ── Color palette for the pie chart ──────────────────────────
export const PIE_COLORS = [
  '#5b7c99', // profiles
  '#4a9eff', // objects
  '#e68a00', // trash
  '#7b61ff', // snapshots
  '#2e7d32', // attachments
  '#d32f2f', // AI conversations
];

export interface PieSlice {
  key: string;
  value: number;
  color: string;
  label: string;
}

export const PieChartSvg = memo(function PieChartSvg({
  slices,
  size,
}: {
  slices: PieSlice[];
  size: number;
}) {
  const total = slices.reduce((s, p) => s + p.value, 0);
  if (total === 0) return null;
  const cx = size / 2;
  const cy = size / 2;
  const r = size / 2 - 4;
  let cumulative = 0;

  const arcs = slices.map((slice) => {
    const sliceAngle = (slice.value / total) * 360;
    const startAngle = (cumulative / total) * 360;
    cumulative += slice.value;

    const startRad = ((startAngle - 90) * Math.PI) / 180;
    const endRad = ((startAngle + sliceAngle - 90) * Math.PI) / 180;

    const x1 = cx + r * Math.cos(startRad);
    const y1 = cy + r * Math.sin(startRad);
    const x2 = cx + r * Math.cos(endRad);
    const y2 = cy + r * Math.sin(endRad);

    const largeArc = sliceAngle > 180 ? 1 : 0;

    const path =
      sliceAngle >= 360
        ? `M ${cx} ${cy - r} A ${r} ${r} 0 1 1 ${cx - 0.01} ${cy - r} Z`
        : `M ${cx} ${cy} L ${x1} ${y1} A ${r} ${r} 0 ${largeArc} 1 ${x2} ${y2} Z`;

    return (
      <path
        key={slice.key}
        d={path}
        fill={slice.color}
        stroke="var(--bg-primary)"
        strokeWidth={1}
      />
    );
  });

  return (
    <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`} style={{ display: 'block' }}>
      {arcs}
      {total > 0 && (
        <text
          x={cx}
          y={cy}
          textAnchor="middle"
          dominantBaseline="central"
          fontSize={13}
          fontWeight={600}
          fill="var(--text-primary)"
        >
          {formatBytes(total)}
        </text>
      )}
    </svg>
  );
});
