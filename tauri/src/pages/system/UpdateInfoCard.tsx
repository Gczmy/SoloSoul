import { useTranslation } from 'react-i18next';
import { Card } from '@/components/ui/Card';
import { LoadingPlaceholder } from '@/components/ui/LoadingPlaceholder';
import { SafeMarkdown } from '@/components/ui/SafeMarkdown';
import { AlertTriangle, Download, RefreshCw } from 'lucide-react';
import { formatBytes } from '@/lib/utils';
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
  downloadProgress: UpdateProgress | ApkDownloadProgress | null;
  downloadedBytes: number;
  totalBytes: number;
  downloadError: string | null;
  progressPercent: number;
  runCheck: () => void;
  handleUpdate: () => void;
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
  downloadProgress,
  downloadedBytes,
  totalBytes,
  downloadError,
  progressPercent,
  runCheck,
  handleUpdate,
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
                      ? t('settings:update_checking') || 'Checking...'
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
                  {versionInfo.body && (
                    <SafeMarkdown
                      className="release-notes-md"
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
                    </SafeMarkdown>
                  )}

                  {/* 下载按钮或进度 */}
                  {downloading ? (
                    <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
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
                        }}
                      >
                        {'event' in (downloadProgress || {})
                          ? t('settings:installing') || 'Installing...'
                          : `${formatBytes(downloadedBytes)} / ${formatBytes(totalBytes)} (${progressPercent}%)`}
                      </span>
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
                      {t('settings:update_now') || 'Update Now'}
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
