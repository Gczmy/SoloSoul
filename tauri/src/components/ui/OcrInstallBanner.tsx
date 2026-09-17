import { useState, useEffect, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { Loader2, AlertTriangle, RotateCcw, CheckCircle, X } from 'lucide-react';
import { ICON_SIZE } from '@/lib/constants';
import { isMobilePlatformSync } from '@/lib/platform';
import styles from './NotificationBanner.module.css';

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

  const progressValue = Math.min(100, Math.max(0, progress));

  return (
    <div
      className={styles.banner}
      data-notification-banner="ocr"
      data-mobile={isMobilePlatformSync() ? 'true' : undefined}
      data-tone={isError ? 'error' : isCompleted ? 'success' : undefined}
    >
      <div className={styles.row}>
        <div className={styles.content}>
          <div className={styles.summary}>
            {isError ? (
              <AlertTriangle size={ICON_SIZE.md} className={styles.statusIcon} />
            ) : isCompleted ? (
              <CheckCircle size={ICON_SIZE.md} className={styles.statusIcon} />
            ) : (
              <Loader2 size={ICON_SIZE.md} className={`${styles.statusIcon} ${styles.spinner}`} />
            )}
            <div className={styles.messageGroup}>
              <span className={styles.message}>
                {isError
                  ? t('first_install_error')
                  : isCompleted
                    ? t('first_install_completed')
                    : t('first_install_banner', { progress: progressValue })}
                {isCompleted && (
                  <span className={styles.countdown}>
                    ({t('auto_close_countdown', { seconds: remainingSeconds })})
                  </span>
                )}
              </span>
              {isError && error && (
                <span className={styles.errorText} title={error}>
                  {error}
                </span>
              )}
            </div>
          </div>

          {!isError && !isCompleted && (
            <div className={styles.progressGroup}>
              <div
                className={styles.progressTrack}
                role="progressbar"
                aria-label={t('first_install_banner', { progress: progressValue })}
                aria-valuemin={0}
                aria-valuemax={100}
                aria-valuenow={progressValue}
              >
                <div className={styles.progressFill} style={{ width: `${progressValue}%` }} />
              </div>
            </div>
          )}
        </div>

        <div className={styles.actions}>
          {isError && (
            <button
              type="button"
              className={`${styles.button} ${styles.primaryButton}`}
              onClick={onRetry}
            >
              <RotateCcw size={ICON_SIZE.xs} /> {t('first_install_retry')}
            </button>
          )}
          <button
            type="button"
            className={`${styles.button} ${styles.iconButton} ${styles.closeButton}`}
            aria-label={t('close', { ns: 'common' })}
            title={t('close', { ns: 'common' })}
            onClick={() => {
              if (intervalRef.current) {
                window.clearInterval(intervalRef.current);
                intervalRef.current = null;
              }
              onClose?.();
            }}
          >
            <X size={ICON_SIZE.sm} />
          </button>
        </div>
      </div>
    </div>
  );
}
