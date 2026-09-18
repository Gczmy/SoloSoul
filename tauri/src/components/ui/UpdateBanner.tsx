import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Download, CheckCircle2, X, Info, AlertTriangle } from 'lucide-react';
import { formatBytes } from '@/lib/utils';
import { ICON_SIZE } from '@/lib/constants';
import { isMobilePlatformSync } from '@/lib/platform';
import { Dialog } from '@/components/ui/Dialog';
import { ReleaseNotesMarkdown } from '@/components/ui/ReleaseNotesMarkdown';
import styles from './NotificationBanner.module.css';
import { UpdateTransferStatus } from './UpdateTransferStatus';
import type { UpdateTransferInfo } from '@/lib/updater';

// 与关于页、强制更新共用渲染器；正文仍仅在打开弹窗时挂载。
// 生产包已通过其他入口包含此模块，额外动态导入只会增加失败后显示源码的分支。
const MARKDOWN_STYLE: React.CSSProperties = {
  fontSize: 'var(--text-body-sm)',
  color: 'var(--text-secondary)',
  lineHeight: 1.6,
  maxHeight: 420,
  overflowY: 'auto',
};

export type UpdateBannerState =
  | 'available'
  | 'downloading'
  | 'cancelling'
  | 'downloaded'
  | 'installing'
  | 'error';

interface UpdateBannerProps {
  version: string;
  state: UpdateBannerState;
  downloadedBytes: number;
  totalBytes: number;
  /** Android 下载进度百分比（0–100），totalBytes 为 0 时作为回退显示 */
  progressPercent?: number;
  transfer?: UpdateTransferInfo;
  error?: string;
  /** 强制更新时隐藏「跳过」与关闭按钮 */
  mandatory?: boolean;
  /** 最新版本 release notes（available 状态展示「查看更新内容」按钮） */
  releaseNotes?: string | null;
  /** P012: APK 校验和不可用原因（Android）；available 状态时在横幅下方渲染警告条 */
  checksumWarning?: string | null;
  onUpdate: () => void;
  onCancel: () => void;
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
  transfer,
  error,
  mandatory,
  releaseNotes,
  checksumWarning,
  onUpdate,
  onCancel,
  onInstall,
  onSkip,
  onClose,
}: UpdateBannerProps) {
  const { t } = useTranslation('common');
  const [notesOpen, setNotesOpen] = useState(false);
  // 移动端仅显示图标按钮（竖屏空间有限），桌面端图标 + 文字。
  const isMobile = isMobilePlatformSync();

  const downloadInProgress = state === 'downloading' || state === 'cancelling';
  const progressValue = Math.min(
    100,
    Math.max(0, totalBytes > 0 ? (downloadedBytes / totalBytes) * 100 : (progressPercent ?? 0)),
  );
  const canClose = !mandatory && !downloadInProgress && state !== 'installing';

  return (
    <div
      className={styles.banner}
      data-notification-banner="update"
      data-mobile={isMobile ? 'true' : undefined}
      data-tone={state === 'error' ? 'error' : state === 'downloaded' ? 'success' : undefined}
    >
      <div className={styles.row}>
        <div className={styles.content}>
          {state === 'available' && (
            <span className={styles.message}>{t('update_available', { version })}</span>
          )}

          {downloadInProgress && (
            <>
              <span className={styles.message}>
                {state === 'cancelling'
                  ? t('update_cancelling')
                  : t('update_downloading', { version })}
              </span>
              {state === 'downloading' && (
                <UpdateTransferStatus transfer={transfer} className={styles.transferStatus} />
              )}
              <div className={styles.progressGroup}>
                <div
                  className={styles.progressTrack}
                  role="progressbar"
                  aria-label={t('update_downloading', { version })}
                  aria-valuemin={0}
                  aria-valuemax={100}
                  aria-valuenow={progressValue}
                >
                  <div className={styles.progressFill} style={{ width: `${progressValue}%` }} />
                </div>
                <span className={styles.progressCount}>
                  {totalBytes > 0
                    ? `${formatBytes(downloadedBytes)} / ${formatBytes(totalBytes)}`
                    : `${progressValue}%`}
                </span>
              </div>
            </>
          )}

          {state === 'installing' && (
            <span className={styles.message} role="status">
              {t('update_installing')}
            </span>
          )}

          {state === 'downloaded' && (
            <div className={styles.summary}>
              <CheckCircle2 size={ICON_SIZE.md} className={styles.statusIcon} />
              <span className={styles.message}>{t('update_downloaded')}</span>
            </div>
          )}

          {state === 'error' && (
            <div className={styles.messageGroup}>
              <span className={styles.message}>{t('update_error', { version })}</span>
              {error && (
                <span className={styles.errorText} title={error}>
                  {error}
                </span>
              )}
            </div>
          )}
        </div>

        <div className={styles.actions}>
          {state === 'available' && (
            <>
              {releaseNotes && (
                <button
                  type="button"
                  className={`${styles.button} ${isMobile ? styles.iconButton : ''}`}
                  onClick={() => setNotesOpen(true)}
                  aria-label={t('view_release_notes')}
                  title={isMobile ? t('view_release_notes') : undefined}
                >
                  <Info size={ICON_SIZE.xs} />
                  {!isMobile && t('view_release_notes')}
                </button>
              )}
              <button
                type="button"
                className={`${styles.button} ${styles.primaryButton} ${isMobile ? styles.iconButton : ''}`}
                onClick={onUpdate}
                aria-label={t('update_now')}
                title={isMobile ? t('update_now') : undefined}
              >
                <Download size={ICON_SIZE.xs} />
                {!isMobile && t('update_now')}
              </button>
              {!mandatory && (
                <button type="button" className={styles.button} onClick={onSkip}>
                  {t('skip')}
                </button>
              )}
            </>
          )}

          {downloadInProgress && (
            <button
              type="button"
              className={styles.button}
              onClick={onCancel}
              disabled={state === 'cancelling'}
            >
              {t('cancel_download')}
            </button>
          )}

          {state === 'downloaded' && (
            <button
              type="button"
              className={`${styles.button} ${styles.primaryButton}`}
              onClick={onInstall}
            >
              {t('install_update')}
            </button>
          )}

          {state === 'error' && (
            <button
              type="button"
              className={`${styles.button} ${styles.primaryButton}`}
              onClick={onUpdate}
            >
              {t('retry')}
            </button>
          )}

          {canClose && (
            <button
              type="button"
              className={`${styles.button} ${styles.iconButton} ${styles.closeButton}`}
              onClick={onClose}
              aria-label={t('close')}
            >
              <X size={ICON_SIZE.md} />
            </button>
          )}
        </div>
      </div>

      {state === 'available' && checksumWarning && (
        <div className={styles.warning} role="alert">
          <AlertTriangle size={ICON_SIZE.xs} />
          <span>{checksumWarning}</span>
        </div>
      )}

      {notesOpen && releaseNotes && (
        <Dialog
          isOpen={notesOpen}
          onClose={() => setNotesOpen(false)}
          title={t('release_notes_title', { version })}
          dialogStyle={{ width: 'min(480px, calc(100% - 32px))', minWidth: 0, maxWidth: 480 }}
          priority="default"
        >
          <ReleaseNotesMarkdown style={MARKDOWN_STYLE}>{releaseNotes}</ReleaseNotesMarkdown>
        </Dialog>
      )}
    </div>
  );
}
