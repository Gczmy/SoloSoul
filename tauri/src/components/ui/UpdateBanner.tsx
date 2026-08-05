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
  /** Android 下载进度百分比（0–100），totalBytes 为 0 时作为回退显示 */
  progressPercent?: number;
  error?: string;
  /** 强制更新时隐藏「跳过」与关闭按钮 */
  mandatory?: boolean;
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
  progressPercent,
  error,
  mandatory,
  onUpdate,
  onInstall,
  onSkip,
  onClose,
}: UpdateBannerProps) {
  const { t } = useTranslation('common');

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
            {!mandatory && (
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
            )}
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
              background: 'rgba(255,255,255,0.3)',
              overflow: 'hidden',
            }}
          >
            <div
              style={{
                width: `${Math.min(100, Math.max(0, totalBytes > 0 ? (downloadedBytes / totalBytes) * 100 : (progressPercent ?? 0)))}%`,
                height: '100%',
                borderRadius: 3,
                background: 'linear-gradient(90deg, rgba(255,255,255,0.95), #ffe9c4)',
                transition: 'width 0.2s ease',
              }}
            />
          </div>
          <span
            style={{
              fontSize: 'var(--text-caption)',
              whiteSpace: 'nowrap',
              /* 数字等宽（tabular-nums）+ 足够最小宽度 + RTL 右对齐：
                 下载数字位数变化（22.7→5.1→54.0）时宽度恒定，进度条与左侧文字不抖动 */
              fontVariantNumeric: 'tabular-nums',
              minWidth: 96,
              textAlign: 'right',
              direction: 'rtl',
            }}
          >
            {totalBytes > 0
              ? `${formatBytes(downloadedBytes)} / ${formatBytes(totalBytes)}`
              : `${progressPercent ?? 0}%`}
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

      {state !== 'downloading' && !mandatory && (
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
