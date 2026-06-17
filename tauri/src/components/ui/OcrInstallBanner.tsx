import { useTranslation } from 'react-i18next';
import { Loader2, AlertTriangle, RotateCcw } from 'lucide-react';

interface OcrInstallBannerProps {
  progress: number;
  error: string | null;
  onRetry: () => void;
}

export function OcrInstallBanner({ progress, error, onRetry }: OcrInstallBannerProps) {
  const { t } = useTranslation('ocr');

  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        gap: 8,
        padding: '10px 16px',
        background: error ? 'var(--color-error-bg, #fdeaea)' : 'var(--accent-primary)',
        color: error ? 'var(--color-error-text, #c0392b)' : 'white',
        fontSize: 13,
        boxShadow: 'var(--shadow-md)',
      }}
    >
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          {error ? <AlertTriangle size={16} /> : <Loader2 size={16} className="spin" />}
          <span style={{ fontWeight: 500 }}>
            {error
              ? t('first_install_error')
              : t('first_install_banner', { progress })}
          </span>
        </div>
        {error && (
          <button
            onClick={onRetry}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 4,
              padding: '5px 10px',
              borderRadius: 6,
              border: 'none',
              background: 'rgba(192, 57, 43, 0.12)',
              color: 'var(--color-error-text, #c0392b)',
              fontSize: 12,
              fontWeight: 500,
              cursor: 'pointer',
            }}
          >
            <RotateCcw size={13} /> {t('first_install_retry')}
          </button>
        )}
      </div>

      {!error && (
        <div
          style={{
            width: '100%',
            height: 4,
            borderRadius: 2,
            background: 'rgba(255,255,255,0.25)',
            overflow: 'hidden',
          }}
        >
          <div
            style={{
              width: `${Math.min(100, Math.max(0, progress))}%`,
              height: '100%',
              background: 'white',
              borderRadius: 2,
              transition: 'width 0.2s ease',
            }}
          />
        </div>
      )}
    </div>
  );
}
