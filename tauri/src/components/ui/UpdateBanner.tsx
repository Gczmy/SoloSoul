import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Download, CheckCircle2, X, Info, AlertTriangle } from 'lucide-react';
import { formatBytes } from '@/lib/utils';
import { ICON_SIZE } from '@/lib/constants';
import { isMobilePlatformSync } from '@/lib/platform';
import { Dialog } from '@/components/ui/Dialog';
import styles from './NotificationBanner.module.css';
import { UpdateTransferStatus } from './UpdateTransferStatus';
import type { UpdateTransferInfo } from '@/lib/updater';

// P015-R2: 更新说明（react-markdown 全家桶约 350K）按需动态加载——
// UpdateBanner 被入口 AppRoutes 静态引用，原静态导入把整个 markdown 栈打进入口 chunk，
// 每次启动（含登录页）都需解析。动态化后 markdown 仅在真正打开 release notes 时拉取。
type ReleaseNotesComponent =
  (typeof import('@/components/ui/ReleaseNotesMarkdown'))['ReleaseNotesMarkdown'];
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
  const [MarkdownRenderer, setMarkdownRenderer] = useState<ReleaseNotesComponent | null>(null);
  // 移动端仅显示图标按钮（竖屏空间有限），桌面端图标 + 文字。
  const isMobile = isMobilePlatformSync();

  // 仅在用户真正打开 release notes 时才拉取 markdown 栈（横幅常驻期间不提前加载）；
  // 模块缓存保证重复打开零成本，加载失败静默降级为纯文本。
  useEffect(() => {
    if (!notesOpen) return;
    let mounted = true;
    import('@/components/ui/ReleaseNotesMarkdown')
      .then((m) => {
        if (mounted) setMarkdownRenderer(() => m.ReleaseNotesMarkdown);
      })
      .catch(() => {
        // 加载失败静默降级：release notes 以纯文本展示，不阻塞横幅
      });
    return () => {
      mounted = false;
    };
  }, [notesOpen]);

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
          dialogStyle={{ maxWidth: 480 }}
          priority="default"
        >
          {MarkdownRenderer ? (
            <MarkdownRenderer style={MARKDOWN_STYLE}>{releaseNotes}</MarkdownRenderer>
          ) : (
            <pre
              className="release-notes-md"
              style={{
                ...MARKDOWN_STYLE,
                whiteSpace: 'pre-wrap',
                wordBreak: 'break-word',
                margin: 0,
                fontFamily: 'inherit',
              }}
            >
              {releaseNotes}
            </pre>
          )}
        </Dialog>
      )}
    </div>
  );
}
