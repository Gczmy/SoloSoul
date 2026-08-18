import { memo } from 'react';
import { useTranslation } from 'react-i18next';
import { Circle, CircleDot, Lock, TriangleAlert } from 'lucide-react';
import type { SensitivityLevel } from '@/types/template';

export type { SensitivityLevel };

// §12 敏感度等级系统重构 + §9.4 视觉规范 — 唯一真理来源
// Icons use Lucide SVG (no emoji, per §9.4)
const STYLES: Record<SensitivityLevel, { bg: string; fg: string }> = {
  public: { bg: 'rgba(61,139,94,0.10)', fg: '#3D8B5E' },
  internal: { bg: 'rgba(74,144,217,0.10)', fg: '#4A90D9' },
  sensitive: { bg: 'rgba(212,133,10,0.10)', fg: '#D4850A' },
  critical: { bg: 'rgba(192,57,43,0.10)', fg: '#C0392B' },
};

function SensitivityIcon({ level, size = 10 }: { level: SensitivityLevel; size?: number }) {
  switch (level) {
    case 'public':
      // Hollow ring (outline circle)
      return <Circle size={size} strokeWidth={2} />;
    case 'internal':
      // Filled dot
      return <CircleDot size={size} strokeWidth={2} />;
    case 'sensitive':
      // Warning triangle
      return <TriangleAlert size={size} strokeWidth={2} />;
    case 'critical':
      // Lock icon
      return <Lock size={size} strokeWidth={2.5} />;
  }
}

export function getSensitivityStyle(level: SensitivityLevel) {
  return STYLES[level] || STYLES.internal;
}

export const SensitivityBadge = memo(function SensitivityBadge({
  level,
  showText = true,
}: {
  level: SensitivityLevel;
  /** 仅图标模式（title 悬停提示保留）——窄屏多徽章行防溢出用 */
  showText?: boolean;
}) {
  const { t } = useTranslation('sensitivity');
  const s = getSensitivityStyle(level);
  const label = t(level);
  return (
    <span
      title={`${t('sensitivity_label')}: ${label}`}
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        gap: 3,
        fontSize: 'var(--text-badge)',
        fontWeight: 600,
        padding: showText ? '2px 6px' : '2px 4px',
        borderRadius: 4,
        background: s.bg,
        color: s.fg,
        border: `1px solid ${s.fg}`,
        lineHeight: 1.3,
        whiteSpace: 'nowrap',
      }}
    >
      <SensitivityIcon level={level} />
      {showText && label}
    </span>
  );
});
