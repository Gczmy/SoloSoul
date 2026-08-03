import { useTranslation } from 'react-i18next';
import { createPortal } from 'react-dom';
import { SafeMarkdown } from '@/components/ui/SafeMarkdown';
import { AlertTriangle, Download } from 'lucide-react';
import { formatBytes } from '@/lib/utils';
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
          background: 'var(--bg-card)',
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

        {/* Release notes */}
        {versionInfo?.body && (
          <SafeMarkdown
            className="release-notes-md release-notes-md-overlay"
            style={
              {
                fontSize: 'var(--text-caption)',
                color: 'var(--text-tertiary)',
                lineHeight: 1.5,
                padding: '8px 12px',
                borderRadius: 8,
                background: 'var(--bg-toolbar)',
                width: '100%',
                maxHeight: 120,
                overflowY: 'auto',
                boxSizing: 'border-box',
              } as React.CSSProperties
            }
          >
            {versionInfo.body}
          </SafeMarkdown>
        )}

        {/* 下载进度 */}
        {downloading && (
          <div
            style={{
              width: '100%',
              display: 'flex',
              flexDirection: 'column',
              gap: 4,
            }}
          >
            <div
              style={{
                width: '100%',
                height: 6,
                borderRadius: 3,
                background: 'var(--bg-toolbar)',
                overflow: 'hidden',
              }}
            >
              <div
                style={{
                  width: `${progressPercent}%`,
                  height: '100%',
                  background: 'var(--accent-primary)',
                  borderRadius: 3,
                  transition: 'width 0.2s ease',
                }}
              />
            </div>
            <span
              style={{
                fontSize: 'var(--text-badge)',
                color: 'var(--text-tertiary)',
                textAlign: 'center',
              }}
            >
              {`${formatBytes(downloadedBytes)} / ${formatBytes(totalBytes)} (${progressPercent}%)`}
            </span>
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
            {t('settings:update_now') || 'Update Now'}
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
            {t('settings:installing') || 'Installing...'}
          </p>
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
