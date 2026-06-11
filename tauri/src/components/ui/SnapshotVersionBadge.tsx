import { useTranslation } from 'react-i18next';

interface SnapshotVersionBadgeProps {
  index: number;
  total: number;
}

/**
 * 快照版本徽章 — 唯一真理来源
 * 显示当前版本 / 前一版本 / 初始版本 三种状态
 */
export function SnapshotVersionBadge({ index, total }: SnapshotVersionBadgeProps) {
  const { t } = useTranslation('common');

  if (index <= 1) {
    return (
      <span
        style={{
          padding: '3px 8px',
          borderRadius: 6,
          fontSize: 10,
          fontWeight: 600,
          background: index === 0 ? 'rgba(39,174,96,0.12)' : 'rgba(91,124,153,0.08)',
          color: index === 0 ? '#27ae60' : 'var(--accent-primary)',
        }}
      >
        {index === 0 ? t('current_version') : t('previous_version')}
      </span>
    );
  }

  if (index === total - 1 && total > 2) {
    return (
      <span
        style={{
          padding: '3px 8px',
          borderRadius: 6,
          fontSize: 10,
          fontWeight: 600,
          background: 'rgba(230,126,34,0.12)',
          color: '#e67e22',
        }}
      >
        {t('initial_version')}
      </span>
    );
  }

  return null;
}
