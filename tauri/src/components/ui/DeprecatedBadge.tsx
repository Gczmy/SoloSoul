import { useTranslation } from 'react-i18next';

export function DeprecatedBadge() {
  const { t } = useTranslation('common');
  return (
    <span
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        gap: 3,
        fontSize: 'var(--text-badge)',
        fontWeight: 600,
        padding: '2px 6px',
        borderRadius: 4,
        background: 'rgba(128,128,128,0.10)',
        color: '#888888',
        border: '1px solid #888888',
        lineHeight: 1.3,
      }}
    >
      {t('deprecated')}
    </span>
  );
}
