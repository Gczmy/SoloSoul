import { useTranslation } from 'react-i18next';
import { Download, CheckCircle2, X } from 'lucide-react';
import { formatBytes } from '@/lib/utils';
import { ICON_SIZE } from '@/lib/constants';

export type UpdateBannerState = 'available' | 'downloading' | 'downloaded' | 'error';

interface UpdateBannerProps {
  version: string;
  state: UpdateBannerState;
  downloadedBytes: number;
  totalBytes: number;
  error?: string;
  onUpdate: () => void;
  onInstall: () => void;
  onSkip: () => void;
  onClose: () => void;
}

export function UpdateBanner({
  version,
  state,
  downloadedBytes,
  totalBytes,
  error,
  onUpdate,
  onInstall,
  onSkip,
  onClose,
}: UpdateBannerProps) {
  const { t } = useTranslation('common');

  const progressPercent =
    totalBytes > 0 ? Math.min(Math.round((downloadedBytes / totalBytes) * 100), 100) : 0;

  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        gap: 12,
        padding: '10px 16px',
        background: 'var(--accent-primary)',
        color: 'white',
        fontSize: 'var(--text-body-sm)',
        boxShadow: 'var(--shadow-md)',
        position: 'relative',
      }}
    >
      {state === 'available' && (
        <>
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
                fontSize: 'var(--text-caption)',
                fontWeight: 500,
                cursor: 'pointer',
              }}
            >
              <Download size={ICON_SIZE.xs} /> {t('update_now')}
            </button>
            <button
              onClick={onSkip}
              style={{
                padding: '5px 10px',
                borderRadius: 6,
                border: '1px solid rgba(255,255,255,0.35)',
                background: 'transparent',
                color: 'white',
                fontSize: 'var(--text-caption)',
                cursor: 'pointer',
              }}
            >
              {t('skip_version')}
            </button>
          </div>
        </>
      )}

      {state === 'downloading' && (
        <>
          <span style={{ fontWeight: 500, whiteSpace: 'nowrap' }}>
            {t('update_downloading', { version })}
          </span>
          <div
            style={{
              flex: 1,
              maxWidth: 240,
              height: 6,
              borderRadius: 3,
              background: 'rgba(255,255,255,0.25)',
              overflow: 'hidden',
            }}
          >
            <div
              style={{
                width: `${progressPercent}%`,
                height: '100%',
                background: 'white',
                borderRadius: 3,
                transition: 'width 0.2s ease',
              }}
            />
          </div>
          <span
            style={{
              fontSize: 'var(--text-caption)',
              whiteSpace: 'nowrap',
              minWidth: 90,
              textAlign: 'right',
            }}
          >
            {formatBytes(downloadedBytes)} / {formatBytes(totalBytes)}
          </span>
        </>
      )}

      {state === 'downloaded' && (
        <>
          <CheckCircle2 size={ICON_SIZE.md} />
          <span style={{ fontWeight: 500 }}>{t('update_downloaded')}</span>
          <button
            onClick={onInstall}
            style={{
              padding: '5px 10px',
              borderRadius: 6,
              border: 'none',
              background: 'rgba(255,255,255,0.2)',
              color: 'white',
              fontSize: 'var(--text-caption)',
              fontWeight: 500,
              cursor: 'pointer',
            }}
          >
            {t('install_update')}
          </button>
        </>
      )}

      {state === 'error' && (
        <>
          <span style={{ fontWeight: 500 }}>{t('update_error', { version })}</span>
          {error && (
            <span
              style={{
                fontSize: 'var(--text-badge)',
                opacity: 0.9,
                maxWidth: 300,
                overflow: 'hidden',
                textOverflow: 'ellipsis',
              }}
            >
              {error}
            </span>
          )}
          <button
            onClick={onUpdate}
            style={{
              padding: '5px 10px',
              borderRadius: 6,
              border: 'none',
              background: 'rgba(255,255,255,0.2)',
              color: 'white',
              fontSize: 'var(--text-caption)',
              fontWeight: 500,
              cursor: 'pointer',
            }}
          >
            {t('retry')}
          </button>
        </>
      )}

      {state !== 'downloading' && (
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
          <X size={ICON_SIZE.md} />
        </button>
      )}
    </div>
  );
}
