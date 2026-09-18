import { useTranslation } from 'react-i18next';
import { Card } from '@/components/ui/Card';
import { LoadingPlaceholder } from '@/components/ui/LoadingPlaceholder';
import { ReleaseNotesMarkdown } from '@/components/ui/ReleaseNotesMarkdown';
import { AlertTriangle, Download, RefreshCw } from 'lucide-react';
import { DownloadProgressBar } from '@/components/ui/DownloadProgressBar';
import { UpdateTransferStatus } from '@/components/ui/UpdateTransferStatus';
import type { UpdateTransferInfo } from '@/lib/updater';
import { ICON_SIZE } from '@/lib/constants';
import type { AppInfo, VersionInfo } from '@/hooks/useUpdateChecker';
import type { ApkDownloadProgress, UpdateProgress } from '@/lib/updater';

function friendlyPlatform(os: string, _arch: string): string {
  return os === 'macos' ? 'macOS' : os === 'windows' ? 'Windows' : os === 'linux' ? 'Linux' : os;
}

interface UpdateInfoCardProps {
  loading: boolean;
  info: AppInfo | null;
  versionInfo: VersionInfo | null;
  checking: boolean;
  downloading: boolean;
  downloaded?: boolean;
  cancelling?: boolean;
  installing?: boolean;
  downloadProgress: UpdateProgress | ApkDownloadProgress | null;
  downloadedBytes: number;
  totalBytes: number;
  downloadError: string | null;
  progressPercent: number;
  transfer?: UpdateTransferInfo;
  runCheck: () => void;
  handleUpdate: () => void;
  cancelDownload?: () => void;
}

/**
 * 版本/更新信息卡片（P224-④ 拆分）。
 * 数据与回调经 AboutPage 透传，纯展示组件。
 */
export function UpdateInfoCard({
  loading,
  info,
  versionInfo,
  checking,
  downloading,
  downloaded = false,
  cancelling = false,
  installing = false,
  downloadedBytes,
  totalBytes,
  downloadError,
  progressPercent,
  transfer,
  runCheck,
  handleUpdate,
  cancelDownload,
}: UpdateInfoCardProps) {
  const { t } = useTranslation(['settings', 'common']);
  return (
    <Card>
      <div style={{ padding: '2px 0' }}>
        {loading ? (
          <LoadingPlaceholder variant="elevated" minHeight={120} />
        ) : info ? (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 0 }}>
            <div
              style={{
                display: 'flex',
                justifyContent: 'space-between',
                alignItems: 'center',
                padding: '12px 0',
              }}
            >
              <span style={{ fontSize: 'var(--text-body-sm)', color: 'var(--text-secondary)' }}>
                {t('settings:version')}
              </span>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                <span
                  style={{
                    fontSize: 'var(--text-sm)',
                    fontWeight: 500,
                    color: 'var(--text-primary)',
                  }}
                >
                  v{info.version}
                </span>
                {versionInfo?.state === 'available' ? (
                  <span
                    style={{
                      fontSize: 'var(--text-badge)',
                      padding: '2px 8px',
                      borderRadius: 10,
                      background: 'rgba(230,126,34,0.15)',
                      color: '#e67e22',
                      fontWeight: 500,
                    }}
                  >
                    {t('settings:update_available', {
                      version: versionInfo.latestVersion || '',
                    })}
                  </span>
                ) : versionInfo?.state === 'error' ? (
                  <span
                    style={{
                      fontSize: 'var(--text-badge)',
                      padding: '2px 8px',
                      borderRadius: 10,
                      background: 'rgba(231,76,60,0.12)',
                      color: '#e74c3c',
                      fontWeight: 500,
                    }}
                    title={versionInfo.error}
                  >
                    {t('settings:update_check_failed')}
                  </span>
                ) : versionInfo ? (
                  <span
                    style={{
                      fontSize: 'var(--text-badge)',
                      padding: '2px 8px',
                      borderRadius: 10,
                      background: 'rgba(39,174,96,0.12)',
                      color: '#27ae60',
                      fontWeight: 500,
                    }}
                  >
                    {t('settings:latest_version')}
                  </span>
                ) : null}
              </div>
            </div>

            {/* 检查失败 — 显示错误详情与重试入口 */}
            {versionInfo?.state === 'error' && (
              <>
                <div style={{ height: 1, background: 'var(--border-subtle)' }} />
                <div
                  style={{
                    padding: '14px 0',
                    display: 'flex',
                    flexDirection: 'column',
                    gap: 10,
                  }}
                >
                  <div
                    style={{
                      fontSize: 'var(--text-caption)',
                      color: 'var(--error)',
                      display: 'flex',
                      alignItems: 'flex-start',
                      gap: 6,
                      lineHeight: 1.5,
                      wordBreak: 'break-word',
                    }}
                  >
                    <AlertTriangle size={ICON_SIZE.xs} style={{ marginTop: 2, flexShrink: 0 }} />
                    <span>{versionInfo.error || t('settings:update_check_failed')}</span>
                  </div>
                  <button
                    type="button"
                    onClick={runCheck}
                    disabled={checking}
                    className="interactive-toolbar"
                    style={{
                      padding: '8px 16px',
                      borderRadius: 8,
                      borderWidth: 1,
                      borderStyle: 'solid',
                      fontSize: 'var(--text-body-sm)',
                      fontWeight: 500,
                      fontFamily: 'inherit',
                      cursor: checking ? 'default' : 'pointer',
                      display: 'flex',
                      alignItems: 'center',
                      gap: 6,
                      alignSelf: 'flex-start',
                      opacity: checking ? 0.6 : 1,
                    }}
                  >
                    <RefreshCw
                      size={ICON_SIZE.sm}
                      className={checking ? 'about-retry-spin' : undefined}
                    />
                    {checking
                      ? t('settings:update_checking', { defaultValue: 'Checking...' })
                      : t('settings:update_check_retry')}
                  </button>
                </div>
              </>
            )}

            {/* 更新卡片 — 有可用更新时显示 */}
            {versionInfo?.state === 'available' && versionInfo.latestVersion && (
              <>
                <div style={{ height: 1, background: 'var(--border-subtle)' }} />
                <div
                  style={{
                    padding: '14px 0',
                    display: 'flex',
                    flexDirection: 'column',
                    gap: 10,
                  }}
                >
                  <div
                    style={{
                      fontSize: 'var(--text-body-sm)',
                      fontWeight: 600,
                      display: 'flex',
                      alignItems: 'center',
                      gap: 6,
                    }}
                  >
                    <Download size={ICON_SIZE.sm} />v{info.version} → v{versionInfo.latestVersion}
                  </div>
                  {versionInfo.checksumWarning && (
                    <div
                      style={{
                        display: 'flex',
                        alignItems: 'flex-start',
                        gap: 6,
                        fontSize: 'var(--text-badge)',
                        color: '#e67e22',
                        lineHeight: 1.5,
                      }}
                      role="alert"
                    >
                      <AlertTriangle size={14} style={{ flexShrink: 0, marginTop: 1 }} />
                      <span>{versionInfo.checksumWarning}</span>
                    </div>
                  )}
                  {versionInfo.body && (
                    <ReleaseNotesMarkdown
                      style={
                        {
                          fontSize: 'var(--text-caption)',
                          color: 'var(--text-secondary)',
                          lineHeight: 1.5,
                          maxHeight: 200,
                          overflowY: 'auto',
                        } as React.CSSProperties
                      }
                    >
                      {versionInfo.body}
                    </ReleaseNotesMarkdown>
                  )}

                  {/* 下载按钮或进度（P043: 共享 DownloadProgressBar） */}
                  {downloading ? (
                    <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
                      <DownloadProgressBar
                        downloadedBytes={downloadedBytes}
                        totalBytes={totalBytes}
                        progressPercent={progressPercent}
                        statusText={
                          installing
                            ? t('common:update_installing')
                            : cancelling
                              ? t('common:update_cancelling')
                              : undefined
                        }
                      />
                      {!installing && !cancelling && <UpdateTransferStatus transfer={transfer} />}
                      {!installing && cancelDownload && (
                        <button
                          type="button"
                          className="interactive-toolbar"
                          onClick={cancelDownload}
                          disabled={cancelling}
                          style={{
                            alignSelf: 'flex-start',
                            minHeight: 36,
                            padding: '8px 16px',
                            borderRadius: 8,
                            border: '1px solid var(--border-subtle)',
                            font: 'inherit',
                            cursor: cancelling ? 'default' : 'pointer',
                            opacity: cancelling ? 0.6 : 1,
                          }}
                        >
                          {t('common:cancel_download')}
                        </button>
                      )}
                    </div>
                  ) : (
                    <button
                      type="button"
                      onClick={handleUpdate}
                      className="interactive-toolbar"
                      style={{
                        padding: '8px 16px',
                        borderRadius: 8,
                        borderWidth: 1,
                        borderStyle: 'solid',
                        fontSize: 'var(--text-body-sm)',
                        fontWeight: 500,
                        fontFamily: 'inherit',
                        cursor: 'pointer',
                        display: 'flex',
                        alignItems: 'center',
                        gap: 6,
                        alignSelf: 'flex-start',
                      }}
                    >
                      <Download size={ICON_SIZE.sm} />
                      {downloaded
                        ? t('common:install_update', { defaultValue: '安装更新' })
                        : t('settings:update_now', { defaultValue: 'Update Now' })}
                    </button>
                  )}
                  {downloadError && (
                    <div style={{ fontSize: 'var(--text-caption)', color: 'var(--error)' }}>
                      {downloadError.includes('NEED_INSTALL_UNKNOWN_APPS_PERMISSION')
                        ? t('settings:need_install_unknown_apps', {
                            defaultValue:
                              '请在系统设置中为 SoloSoul 开启「安装未知应用」权限，然后重新点击更新。',
                          })
                        : downloadError}
                    </div>
                  )}
                </div>
              </>
            )}

            <div style={{ height: 1, background: 'var(--border-subtle)' }} />
            <div
              style={{
                display: 'flex',
                justifyContent: 'space-between',
                alignItems: 'center',
                padding: '12px 0',
              }}
            >
              <span style={{ fontSize: 'var(--text-body-sm)', color: 'var(--text-secondary)' }}>
                {t('settings:platform')}
              </span>
              <span
                style={{
                  fontSize: 'var(--text-sm)',
                  fontWeight: 500,
                  color: 'var(--text-primary)',
                }}
              >
                {friendlyPlatform(info.os, info.arch)}
              </span>
            </div>
          </div>
        ) : (
          <div
            style={{
              textAlign: 'center',
              padding: 16,
              color: 'var(--text-tertiary)',
              fontSize: 'var(--text-body-sm)',
            }}
          >
            {t('settings:could_not_load')}
          </div>
        )}
      </div>
    </Card>
  );
}
