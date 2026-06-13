import { useTranslation } from 'react-i18next';
import { Download, X } from 'lucide-react';

interface UpdateBannerProps {
  version: string;
  onUpdate: () => void;
  onSkip: () => void;
  onClose: () => void;
}

export function UpdateBanner({ version, onUpdate, onSkip, onClose }: UpdateBannerProps) {
  const { t } = useTranslation('common');

  return (
    <div
      style={{
        position: 'fixed',
        top: 0,
        left: 0,
        right: 0,
        zIndex: 1000,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        gap: 12,
        padding: '10px 16px',
        background: 'var(--accent-primary)',
        color: 'white',
        fontSize: 13,
        boxShadow: 'var(--shadow-md)',
      }}
    >
      <span style={{ fontWeight: 500 }}>{t('update_available', { version })}</span>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
        <button
          onClick={onUpdate}
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 4,
            padding: '5px 10px',
            borderRadius: 6,
            border: 'none',
            background: 'rgba(255,255,255,0.2)',
            color: 'white',
            fontSize: 12,
            fontWeight: 500,
            cursor: 'pointer',
          }}
        >
          <Download size={13} /> {t('update_now')}
        </button>
        <button
          onClick={onSkip}
          style={{
            padding: '5px 10px',
            borderRadius: 6,
            border: '1px solid rgba(255,255,255,0.35)',
            background: 'transparent',
            color: 'white',
            fontSize: 12,
            cursor: 'pointer',
          }}
        >
          {t('skip_version')}
        </button>
      </div>
      <button
        onClick={onClose}
        style={{
          position: 'absolute',
          right: 12,
          padding: 4,
          borderRadius: 6,
          border: 'none',
          background: 'transparent',
          color: 'white',
          cursor: 'pointer',
        }}
        aria-label={t('close')}
      >
        <X size={16} />
      </button>
    </div>
  );
}
