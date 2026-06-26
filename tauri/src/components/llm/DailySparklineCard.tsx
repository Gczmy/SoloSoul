import { memo } from 'react';
import { Card } from '@/components/ui/Card';
import { type DailyUsage } from '@/lib/llm/statsApi';
import type { TFunction } from 'i18next';

interface DailySparklineCardProps {
  daily: DailyUsage[];
  t: TFunction;
}

const CHART_COLORS = [
  'var(--accent-primary)',
  'var(--accent-warm)',
  '#e68a00',
  '#7b61ff',
  '#2e7d32',
  '#d32f2f',
  '#2980b9',
  '#b06b7a',
];

function niceMax(max: number): number {
  if (max <= 0) return 1;
  const exponent = Math.floor(Math.log10(max));
  const fraction = max / Math.pow(10, exponent);
  let nice: number;
  if (fraction <= 1) nice = 1;
  else if (fraction <= 2) nice = 2;
  else if (fraction <= 5) nice = 5;
  else nice = 10;
  return nice * Math.pow(10, exponent);
}

function formatY(value: number): string {
  if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(1)}M`;
  if (value >= 1_000) return `${(value / 1_000).toFixed(1)}K`;
  return value.toFixed(0);
}

export const DailySparklineCard = memo(function DailySparklineCard({ daily, t }: DailySparklineCardProps) {
  const sorted = [...daily].sort((a, b) => a.date.localeCompare(b.date));
  const last14 = sorted.length > 14 ? sorted.slice(sorted.length - 14) : sorted;
  if (last14.length === 0) return null;

  // Build series: prioritize per-model lines
  const allModels = Array.from(
    new Set(last14.flatMap((d) => Object.keys(d.perModelTokens))),
  ).sort();
  const series: { name: string; values: number[] }[] = [];
  if (allModels.length > 0) {
    for (const model of allModels) {
      series.push({
        name: model.split('/').pop() || model,
        values: last14.map((d) => d.perModelTokens[model] || 0),
      });
    }
  } else {
    series.push({
      name: t('settings:llm_all_models'),
      values: last14.map((d) => d.tokens),
    });
  }

  const allValues = series.flatMap((s) => s.values);
  const rawMax = allValues.length === 0 ? 1 : Math.max(...allValues);
  const yMax = niceMax(rawMax);

  // SVG layout
  const width = 600;
  const height = 200;
  const plotLeft = 56;
  const plotRight = width - 12;
  const plotTop = 8;
  const plotBottom = height - 28;
  const plotWidth = plotRight - plotLeft;
  const plotHeight = plotBottom - plotTop;
  const n = last14.length;

  function xForIndex(i: number): number {
    if (n <= 1) return plotLeft + plotWidth / 2;
    return plotLeft + i * (plotWidth / (n - 1));
  }

  function yForValue(v: number): number {
    return plotBottom - (v / yMax) * plotHeight;
  }

  // Y-axis ticks (5)
  const yTicks = 5;
  const gridLines = Array.from({ length: yTicks + 1 }, (_, i) => {
    const value = (i / yTicks) * yMax;
    const y = yForValue(value);
    return { value, y };
  });

  // X-axis labels (sparse, max 4-5)
  const xStep = Math.max(1, Math.ceil(n / 4));
  const xLabels = [];
  for (let i = 0; i < n; i += xStep) {
    const d = new Date(last14[i].date);
    xLabels.push({
      x: xForIndex(i),
      label: `${d.getMonth() + 1}/${d.getDate()}`,
    });
  }

  // Series polylines and points
  const seriesElements = series.map((s, si) => {
    const color = CHART_COLORS[si % CHART_COLORS.length];
    const points = s.values.map((v, i) => `${xForIndex(i)},${yForValue(v)}`).join(' ');
    const circles = s.values.map((v, i) => (
      <circle key={i} cx={xForIndex(i)} cy={yForValue(v)} r={2.5} fill={color} />
    ));
    return (
      <g key={s.name}>
        <polyline
          points={points}
          fill="none"
          stroke={color}
          strokeWidth={2}
          strokeLinecap="round"
          strokeLinejoin="round"
        />
        {circles}
      </g>
    );
  });

  return (
    <Card>
      <div style={{ padding: '4px 0' }}>
        <svg
          viewBox={`0 0 ${width} ${height}`}
          style={{ width: '100%', height: 'auto', display: 'block' }}
          preserveAspectRatio="xMidYMid meet"
        >
          {/* Grid lines */}
          {gridLines.map((tick, i) => (
            <g key={i}>
              <line
                x1={plotLeft}
                y1={tick.y}
                x2={plotRight}
                y2={tick.y}
                stroke="var(--border-subtle)"
                strokeOpacity={0.3}
                strokeWidth={0.5}
              />
              <text
                x={plotLeft - 4}
                y={tick.y}
                textAnchor="end"
                dominantBaseline="middle"
                fill="var(--text-tertiary)"
                fontSize={10}
              >
                {formatY(tick.value)}
              </text>
            </g>
          ))}

          {/* X labels */}
          {xLabels.map((xl, i) => (
            <text
              key={i}
              x={xl.x}
              y={plotBottom + 14}
              textAnchor="middle"
              fill="var(--text-tertiary)"
              fontSize={10}
            >
              {xl.label}
            </text>
          ))}

          {/* Series */}
          {seriesElements}
        </svg>

        {/* Legend */}
        <div style={{ display: 'flex', flexWrap: 'wrap', gap: '12px 16px', marginTop: 12 }}>
          {series.map((s, i) => {
            const color = CHART_COLORS[i % CHART_COLORS.length];
            return (
              <div key={s.name} style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                <div style={{ width: 10, height: 3, borderRadius: 2, background: color }} />
                <span style={{ fontSize: 'var(--text-caption)', color: 'var(--text-tertiary)' }}>{s.name}</span>
              </div>
            );
          })}
        </div>
      </div>
    </Card>
  );
});
