import { memo } from 'react';
import { useTranslation } from 'react-i18next';
import { Circle, CircleDot, Lock } from 'lucide-react';

export type SensitivityLevel = 'public' | 'internal' | 'sensitive' | 'critical';

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
    case 'sensitive':
      // Filled dot
      return <CircleDot size={size} strokeWidth={2} />;
    case 'critical':
      // Lock icon
      return <Lock size={size} strokeWidth={2.5} />;
  }
}

export function getSensitivityStyle(level: SensitivityLevel) {
  return STYLES[level] || STYLES.internal;
}

export const SensitivityBadge = memo(function SensitivityBadge({ level }: { level: SensitivityLevel }) {
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
        padding: '2px 6px',
        borderRadius: 4,
        background: s.bg,
        color: s.fg,
        border: `1px solid ${s.fg}`,
        lineHeight: 1.3,
      }}
    >
      <SensitivityIcon level={level} />
      {label}
    </span>
  );
});
