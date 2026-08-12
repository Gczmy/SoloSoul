import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { createPortal } from 'react-dom';
import { SafeMarkdown } from '@/components/ui/SafeMarkdown';
import { AlertTriangle, Download, Info } from 'lucide-react';
import { DownloadProgressBar } from '@/components/ui/DownloadProgressBar';
import { Dialog } from '@/components/ui/Dialog';
import { ICON_SIZE } from '@/lib/constants';
import { isMobilePlatformSync } from '@/lib/platform';
import type { AppInfo, VersionInfo } from '@/hooks/useUpdateChecker';

interface MandatoryUpdateOverlayProps {
  isMandatory: boolean;
  info: AppInfo | null;
  versionInfo: VersionInfo | null;
  downloading: boolean;
  downloadedBytes: number;
  totalBytes: number;
  progressPercent: number;
  downloadError: string | null;
  handleUpdate: () => void;
}

/**
 * 强制更新全屏覆盖层（P224-④ 拆分）。
 * isMandatory 为 false 时渲染 null；true 时以 Portal 挂到 document.body。
 */
export function MandatoryUpdateOverlay({
  isMandatory,
  info,
  versionInfo,
  downloading,
  downloadedBytes,
  totalBytes,
  progressPercent,
  downloadError,
  handleUpdate,
}: MandatoryUpdateOverlayProps) {
  const { t } = useTranslation(['settings', 'common']);
  const [notesOpen, setNotesOpen] = useState(false);
  // 移动端仅显示图标按钮，与更新横幅 UpdateBanner 交互一致。
  const isMobile = isMobilePlatformSync();
  if (!isMandatory) {
    return null;
  }
  return createPortal(
    <div
      style={{
        position: 'fixed',
        inset: 0,
        zIndex: 9999,
        background: 'var(--bg-overlay)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        padding: 24,
      }}
    >
      <div
        style={{
          // 卡片背景：--bg-elevated（全主题已定义；勿用未定义的 --bg-card，否则背景透明）
          background: 'var(--bg-elevated)',
          borderRadius: 16,
          padding: 32,
          maxWidth: 400,
          width: '100%',
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          gap: 16,
          boxShadow: '0 8px 32px rgba(0,0,0,0.2)',
        }}
      >
        {/* 警示图标 */}
        <div
          style={{
            width: 56,
            height: 56,
            borderRadius: '50%',
            background: 'rgba(231,76,60,0.12)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
          }}
        >
          <AlertTriangle size={28} style={{ color: '#e74c3c' }} />
        </div>

        {/* 标题 */}
        <h2
          style={{
            margin: 0,
            fontSize: 'var(--text-lg)',
            fontWeight: 700,
            textAlign: 'center',
          }}
        >
          {t('settings:mandatory_update_title', {
            defaultValue: '关键安全更新',
          })}
        </h2>

        <p
          style={{
            margin: 0,
            fontSize: 'var(--text-body-sm)',
            color: 'var(--text-secondary)',
            textAlign: 'center',
            lineHeight: 1.5,
          }}
        >
          {t('settings:mandatory_update_desc', {
            defaultValue: '当前版本存在重要安全修复，请立即更新以确保数据安全。',
          })}
        </p>

        {/* 版本号 */}
        <div
          style={{
            fontSize: 'var(--text-sm)',
            fontWeight: 600,
            color: 'var(--accent-primary)',
            padding: '4px 12px',
            borderRadius: 8,
            background: 'var(--bg-toolbar)',
          }}
        >
          v{info?.version ?? '?'} → v{versionInfo?.latestVersion ?? '?'}
        </div>

        {/* 查看更新内容：与更新横幅 UpdateBanner 交互一致——点击弹出完整 release notes 弹卡 */}
        {versionInfo?.body && (
          <button
            type="button"
            onClick={() => setNotesOpen(true)}
            aria-label={t('common:view_release_notes')}
            title={isMobile ? t('common:view_release_notes') : undefined}
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              gap: 6,
              padding: '6px 14px',
              borderRadius: 8,
              border: '1px solid var(--border-subtle)',
              background: 'transparent',
              color: 'var(--text-secondary)',
              fontSize: 'var(--text-caption)',
              fontWeight: 500,
              cursor: 'pointer',
              transition: 'background 0.15s ease, color 0.15s ease',
            }}
            onMouseEnter={(e) => {
              e.currentTarget.style.background = 'var(--bg-toolbar)';
              e.currentTarget.style.color = 'var(--text-primary)';
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.background = 'transparent';
              e.currentTarget.style.color = 'var(--text-secondary)';
            }}
          >
            <Info size={ICON_SIZE.xs} />
            {!isMobile && t('common:view_release_notes')}
          </button>
        )}

        {/* 下载进度（P043: 共享 DownloadProgressBar） */}
        {downloading && (
          <div style={{ width: '100%' }}>
            <DownloadProgressBar
              downloadedBytes={downloadedBytes}
              totalBytes={totalBytes}
              progressPercent={progressPercent}
            />
          </div>
        )}

        {/* 更新按钮或错误 */}
        {!downloading && (
          <button
            type="button"
            onClick={handleUpdate}
            style={{
              width: '100%',
              padding: '12px 24px',
              borderRadius: 10,
              border: 'none',
              background: 'var(--accent-primary)',
              color: '#fff',
              fontSize: 'var(--text-sm)',
              fontWeight: 600,
              fontFamily: 'inherit',
              cursor: 'pointer',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              gap: 8,
            }}
          >
            <Download size={18} />
            {t('settings:update_now', { defaultValue: 'Update Now' })}
          </button>
        )}

        {/* 下载完成后自动安装提示 */}
        {downloading && progressPercent >= 100 && (
          <p
            style={{
              margin: 0,
              fontSize: 'var(--text-caption)',
              color: 'var(--text-secondary)',
              textAlign: 'center',
            }}
          >
            {t('settings:installing', { defaultValue: 'Installing...' })}
          </p>
        )}

        {/* Release notes 弹卡：zIndex 高于遮罩本体（9999），Portal 到 body 后不被遮挡 */}
        {notesOpen && versionInfo?.body && (
          <Dialog
            isOpen={notesOpen}
            onClose={() => setNotesOpen(false)}
            title={t('common:release_notes_title', {
              version: versionInfo.latestVersion ?? '?',
            })}
            dialogStyle={{ maxWidth: 480 }}
            priority="default"
            zIndex={10000}
          >
            <SafeMarkdown
              className="release-notes-md"
              style={{
                fontSize: 'var(--text-body-sm)',
                color: 'var(--text-secondary)',
                lineHeight: 1.6,
                maxHeight: 420,
                overflowY: 'auto',
              }}
            >
              {versionInfo.body}
            </SafeMarkdown>
          </Dialog>
        )}

        {downloadError && (
          <div
            style={{
              fontSize: 'var(--text-caption)',
              color: 'var(--error)',
              textAlign: 'center',
              lineHeight: 1.4,
            }}
          >
            {downloadError.includes('NEED_INSTALL_UNKNOWN_APPS_PERMISSION')
              ? t('settings:need_install_unknown_apps', {
                  defaultValue:
                    '请在系统设置中为 SoloSoul 开启「安装未知应用」权限，然后重新点击更新。',
                })
              : downloadError}
          </div>
        )}
      </div>
    </div>,
    document.body,
  );
}
