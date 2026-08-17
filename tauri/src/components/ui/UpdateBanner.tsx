import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Download, CheckCircle2, X, Info, AlertTriangle } from 'lucide-react';
import { formatBytes } from '@/lib/utils';
import { ICON_SIZE } from '@/lib/constants';
import { isMobilePlatformSync } from '@/lib/platform';
import { Dialog } from '@/components/ui/Dialog';

// P015-R2: SafeMarkdown（react-markdown 全家桶约 350K）由静态导入改为按需动态加载——
// UpdateBanner 被入口 AppRoutes 静态引用，原静态导入把整个 markdown 栈打进入口 chunk，
// 每次启动（含登录页）都需解析。动态化后 markdown 仅在真正打开 release notes 时拉取。
type SafeMarkdownComponent = typeof import('@/components/ui/SafeMarkdown')['SafeMarkdown'];
const MARKDOWN_STYLE: React.CSSProperties = {
  fontSize: 'var(--text-body-sm)',
  color: 'var(--text-secondary)',
  lineHeight: 1.6,
  maxHeight: 420,
  overflowY: 'auto',
};

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
  /** 最新版本 release notes（available 状态展示「查看更新内容」按钮） */
  releaseNotes?: string | null;
  /** P012: APK 校验和不可用原因（Android）；available 状态时在横幅下方渲染警告条 */
  checksumWarning?: string | null;
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
  releaseNotes,
  checksumWarning,
  onUpdate,
  onInstall,
  onSkip,
  onClose,
}: UpdateBannerProps) {
  const { t } = useTranslation('common');
  const [notesOpen, setNotesOpen] = useState(false);
  const [MarkdownRenderer, setMarkdownRenderer] = useState<SafeMarkdownComponent | null>(null);
  // 移动端仅显示图标按钮（竖屏空间有限），桌面端图标 + 文字。
  const isMobile = isMobilePlatformSync();

  // 仅在用户真正打开 release notes 时才拉取 markdown 栈（横幅常驻期间不提前加载）；
  // 模块缓存保证重复打开零成本，加载失败静默降级为纯文本。
  useEffect(() => {
    if (!notesOpen) return;
    let mounted = true;
    import('@/components/ui/SafeMarkdown')
      .then((m) => {
        if (mounted) setMarkdownRenderer(() => m.SafeMarkdown);
      })
      .catch(() => {
        // 加载失败静默降级：release notes 以纯文本展示，不阻塞横幅
      });
    return () => {
      mounted = false;
    };
  }, [notesOpen]);

  return (
    <div
      style={{
        background: 'var(--accent-primary)',
        color: 'white',
        fontSize: 'var(--text-body-sm)',
        boxShadow: 'var(--shadow-md)',
        position: 'relative',
      }}
    >
      {/* P012: 主横幅行（原内容）——warning 条存在时整体降为一行，不破坏布局 */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          gap: 12,
          padding: '10px 16px',
        }}
      >
        {state === 'available' && (
          <>
            <span style={{ fontWeight: 500, whiteSpace: 'nowrap' }}>
              {t('update_available', { version })}
            </span>
            <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
              {/* 查看更新内容：仅在刚开始提醒安装时展示（进入下载进度条后该分支不再渲染） */}
              {releaseNotes && (
                <button
                  onClick={() => setNotesOpen(true)}
                  aria-label={t('view_release_notes')}
                  title={isMobile ? t('view_release_notes') : undefined}
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    gap: 4,
                    /* 移动端仅图标：固定正方形 + 零内边距，图标精确居中；桌面端图标 + 文字 */
                    ...(isMobile ? { width: 30, height: 30, padding: 0 } : { padding: '5px 10px' }),
                    borderRadius: 6,
                    border: '1px solid rgba(255,255,255,0.35)',
                    background: 'transparent',
                    color: 'white',
                    fontSize: 'var(--text-caption)',
                    fontWeight: 500,
                    cursor: 'pointer',
                    whiteSpace: 'nowrap',
                  }}
                >
                  <Info size={ICON_SIZE.xs} />
                  {!isMobile && t('view_release_notes')}
                </button>
              )}
              <button
                onClick={onUpdate}
                aria-label={isMobile ? t('update_now') : undefined}
                title={isMobile ? t('update_now') : undefined}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  gap: 4,
                  /* 移动端仅图标：固定正方形 + 零内边距，图标精确居中；桌面端图标 + 文字 */
                  ...(isMobile ? { width: 30, height: 30, padding: 0 } : { padding: '5px 10px' }),
                  borderRadius: 6,
                  border: 'none',
                  background: 'rgba(255,255,255,0.2)',
                  color: 'white',
                  fontSize: 'var(--text-caption)',
                  fontWeight: 500,
                  cursor: 'pointer',
                  whiteSpace: 'nowrap',
                }}
              >
                <Download size={ICON_SIZE.xs} />
                {!isMobile && t('update_now')}
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
                    whiteSpace: 'nowrap',
                  }}
                >
                  {t('skip')}
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
                /* 数字等宽（tabular-nums）+ 足够最小宽度 + 右对齐：
                 下载数字位数变化（22.7→5.1→54.0）时宽度恒定，进度条与左侧文字不抖动。
                 注意：direction 必须保持默认 LTR——RTL 会对「27.0 MB / 44.2 MB」这类
                 数字+单位文本做 bidi 重排（显示成 MB 27.0），右对齐由 textAlign 承担即可。 */
                fontVariantNumeric: 'tabular-nums',
                minWidth: 96,
                textAlign: 'right',
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

        {/* 最新版本 release notes 展示卡片（仅在 available 状态可打开） */}
      </div>
      {/* P012: 校验和不可用警告条——available 状态时显示，避免用户盲点下载后
          才看到下载命令 fail-closed 的泛化错误 */}
      {state === 'available' && checksumWarning && (
        <div
          style={{
            display: 'flex',
            alignItems: 'flex-start',
            justifyContent: 'center',
            gap: 6,
            padding: '0 16px 10px',
            fontSize: 'var(--text-badge)',
            lineHeight: 1.4,
            textAlign: 'center',
            color: '#fff',
            opacity: 0.95,
          }}
        >
          <AlertTriangle size={12} style={{ flexShrink: 0, marginTop: 2 }} />
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
            <MarkdownRenderer className="release-notes-md" style={MARKDOWN_STYLE}>
              {releaseNotes}
            </MarkdownRenderer>
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
