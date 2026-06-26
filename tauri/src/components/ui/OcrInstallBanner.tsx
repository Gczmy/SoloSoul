import { useState, useEffect, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { Loader2, AlertTriangle, RotateCcw, CheckCircle, X } from 'lucide-react';
import { ICON_SIZE } from '@/lib/iconSizes';


export type OcrInstallPhase = 'installing' | 'completed' | 'error';

interface OcrInstallBannerProps {
  phase: OcrInstallPhase;
  progress: number;
  error: string | null;
  onRetry: () => void;
  /** 完成后自动消失的秒数，默认 5 */
  autoDismissSeconds?: number;
  /** 是否已被用户主动关闭 */
  onClose?: () => void;
}

export function OcrInstallBanner({
  phase,
  progress,
  error,
  onRetry,
  autoDismissSeconds = 5,
  onClose,
}: OcrInstallBannerProps) {
  const { t } = useTranslation('ocr');
  const [remainingSeconds, setRemainingSeconds] = useState(autoDismissSeconds);
  const intervalRef = useRef<number | null>(null);
  const onCloseRef = useRef(onClose);
  onCloseRef.current = onClose;

  // 完成后启动自动消失计时器（onClose 使用 ref 避免父组件 inline 函数导致 timer 重置）
  useEffect(() => {
    if (phase === 'completed' && onCloseRef.current) {
      setRemainingSeconds(autoDismissSeconds);
      const interval = window.setInterval(() => {
        setRemainingSeconds((prev) => {
          if (prev <= 1) {
            window.clearInterval(interval);
            intervalRef.current = null;
            onCloseRef.current?.();
            return 0;
          }
          return prev - 1;
        });
      }, 1000);
      intervalRef.current = interval;
      return () => {
        window.clearInterval(interval);
        intervalRef.current = null;
      };
    }
    return () => {
      if (intervalRef.current) {
        window.clearInterval(intervalRef.current);
        intervalRef.current = null;
      }
    };
  }, [phase, autoDismissSeconds]);

  const isError = phase === 'error' || error !== null;
  const isCompleted = phase === 'completed';

  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        gap: 8,
        padding: '10px 16px',
        background: isError
          ? 'var(--color-error-bg, #fdeaea)'
          : isCompleted
            ? 'var(--color-success-bg, #e8f5e9)'
            : 'var(--accent-primary)',
        color: isError
          ? 'var(--color-error-text, #c0392b)'
          : isCompleted
            ? 'var(--color-success-text, #2e7d32)'
            : 'white',
        fontSize: 'var(--text-body-sm)',
        boxShadow: 'var(--shadow-md)',
      }}
    >
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          {isError ? (
            <AlertTriangle size={ICON_SIZE.md} />
          ) : isCompleted ? (
            <CheckCircle size={ICON_SIZE.md} />
          ) : (
            <Loader2 size={ICON_SIZE.md} className="spin" />
          )}
          <span style={{ fontWeight: 500 }}>
            {isError
              ? t('first_install_error')
              : isCompleted
                ? t('first_install_completed')
                : t('first_install_banner', { progress })}
          </span>
          {isCompleted && (
            <span style={{ fontSize: 'var(--text-badge)', opacity: 0.8 }}>
              ({t('auto_close_countdown', { seconds: remainingSeconds })})
            </span>
          )}
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          {isError && (
            <button
              type="button"
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
                fontSize: 'var(--text-caption)',
                fontWeight: 500,
                cursor: 'pointer',
              }}
            >
              <RotateCcw size={ICON_SIZE.xs} /> {t('first_install_retry')}
            </button>
          )}
          <button
            type="button"
            aria-label={t('close', { ns: 'common' })}
            onClick={() => {
              if (intervalRef.current) {
                window.clearInterval(intervalRef.current);
                intervalRef.current = null;
              }
              onClose?.();
            }}
            title={t('close', { ns: 'common' })}
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              width: 24,
              height: 24,
              borderRadius: 6,
              border: 'none',
              background: 'rgba(0,0,0,0.08)',
              color: 'inherit',
              cursor: 'pointer',
              padding: 0,
            }}
          >
            <X size={ICON_SIZE.sm} />
          </button>
        </div>
      </div>

      {!isError && !isCompleted && (
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
