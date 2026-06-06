import { useTranslation } from 'react-i18next';
import type { SensitivityLevel } from '@/stores/sensitivityStore';

// §12 敏感度等级系统重构 + §9.4 视觉规范 — 唯一真理来源
const STYLES: Record<SensitivityLevel, { bg: string; fg: string; dot: string }> = {
  public:    { bg: 'rgba(61,139,94,0.10)',   fg: '#3D8B5E', dot: '○' },
  internal:  { bg: 'rgba(74,144,217,0.10)',  fg: '#4A90D9', dot: '●' },
  sensitive: { bg: 'rgba(212,133,10,0.10)',  fg: '#D4850A', dot: '●' },
  critical:  { bg: 'rgba(192,57,43,0.10)',   fg: '#C0392B', dot: '🔒' },
};

export function getSensitivityStyle(level: SensitivityLevel) {
  return STYLES[level] || STYLES.internal;
}

export function SensitivityBadge({ level }: { level: SensitivityLevel }) {
  const { t } = useTranslation('sensitivity');
  const s = getSensitivityStyle(level);
  const label = t(level);
  return (
    <span
      title={`${t('sensitivity_label')}: ${label}`}
      style={{
        display: 'inline-flex', alignItems: 'center', gap: 3,
        fontSize: 10, fontWeight: 600, padding: '2px 6px', borderRadius: 4,
        background: s.bg, color: s.fg,
        border: `1px solid ${s.fg}`,
        lineHeight: 1.3,
      }}
    >
      <span style={{ fontSize: level === 'critical' ? 9 : 7, lineHeight: 1 }}>{s.dot}</span>
      {label}
    </span>
  );
}
