import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { createPortal } from 'react-dom';
import { AppShell } from '@/components/layout/AppShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { Card } from '@/components/ui/Card';
import { ShieldLogo } from '@/components/ui/ShieldLogo';
import { open } from '@tauri-apps/plugin-shell';
import { ExternalLink, Code, Shield, Info, Download, AlertTriangle, RefreshCw } from 'lucide-react';
import { LoadingPlaceholder } from '@/components/ui/LoadingPlaceholder';
import { SafeMarkdown } from '@/components/ui/SafeMarkdown';
import { useUpdateChecker } from '@/hooks/useUpdateChecker';
import { formatBytes } from '@/lib/utils';
import { ICON_SIZE } from '@/lib/constants';

function friendlyPlatform(os: string, _arch: string): string {
  return os === 'macos' ? 'macOS' : os === 'windows' ? 'Windows' : os === 'linux' ? 'Linux' : os;
}

/**
 * 关于页：
 * - 更新检查/双平台下载安装状态机收敛于 useUpdateChecker hook
 * - 本组件仅负责翻译、导航与视图渲染
 */
export function AboutPage() {
  const navigate = useNavigate();
  const { t, i18n } = useTranslation(['settings', 'common']);
  const docLang = i18n.language?.startsWith('zh') ? 'zh-CN' : 'en-US';
  const updater = useUpdateChecker();
  const {
    info,
    versionInfo,
    loading,
    checking,
    downloading,
    downloadProgress,
    downloadedBytes,
    totalBytes,
    downloadError,
    progressPercent,
    isMandatory,
    runCheck,
    handleUpdate,
  } = updater;

  const links = [
    {
      labelKey: 'github_repo',
      url: 'https://github.com/Gczmy/SoloSoul',
      icon: <Code size={ICON_SIZE.sm} />,
    },
    {
      labelKey: 'privacy_policy',
      url: `https://github.com/Gczmy/SoloSoul/blob/main/docs/${docLang}/PRIVACY_POLICY.md`,
      icon: <Shield size={ICON_SIZE.sm} />,
    },
    {
      labelKey: 'terms_of_service',
      url: `https://github.com/Gczmy/SoloSoul/blob/main/docs/${docLang}/TERMS_OF_SERVICE.md`,
      icon: <Info size={ICON_SIZE.sm} />,
    },
  ];

  return (
    <>
      {/* 关于页面更新说明 Markdown 样式 */}
      <style>{`
        .release-notes-md h1,
        .release-notes-md h2,
        .release-notes-md h3 {
          font-size: var(--text-sm);
          font-weight: 600;
          margin: 8px 0 4px;
        }
        .release-notes-md h4,
        .release-notes-md h5,
        .release-notes-md h6 {
          font-size: var(--text-caption);
          font-weight: 600;
          margin: 6px 0 3px;
        }
        .release-notes-md p { margin: 0 0 6px; }
        .release-notes-md p:last-child { margin-bottom: 0; }
        .release-notes-md ul,
        .release-notes-md ol { margin: 4px 0; padding-left: 20px; }
        .release-notes-md li { margin: 2px 0; }
        .release-notes-md strong { font-weight: 600; }
        .release-notes-md code {
          font-family: 'Menlo', 'Monaco', 'Courier New', monospace;
          font-size: 12px;
          background: rgba(128,128,128,0.1);
          padding: 1px 4px;
          border-radius: 3px;
        }
        .release-notes-md pre {
          background: var(--bg-toolbar);
          padding: 8px 10px;
          border-radius: 6px;
          overflow-x: auto;
          margin: 6px 0;
        }
        .release-notes-md pre code {
          background: none;
          padding: 0;
        }
        .release-notes-md a {
          color: var(--accent-primary);
          text-decoration: none;
        }
        .release-notes-md a:hover { text-decoration: underline; }
        .release-notes-md blockquote {
          border-left: 3px solid var(--accent-primary);
          margin: 6px 0;
          padding-left: 10px;
          color: var(--text-secondary);
        }
        .release-notes-md hr {
          border: none;
          border-top: 1px solid var(--border-subtle);
          margin: 8px 0;
        }
        @keyframes about-retry-spin {
          to { transform: rotate(360deg); }
        }
        .about-retry-spin { animation: about-retry-spin 1s linear infinite; }
      `}</style>
      <AppShell
        title={t('settings:about')}
        onBack={isMandatory ? undefined : () => navigate('/settings')}
      >
        <PageContainer variant="form" gap="default">
          <div style={{ textAlign: 'center', padding: '20px 0' }}>
            <ShieldLogo
              size={ICON_SIZE['6xl']}
              style={{ margin: '0 auto 14px', boxShadow: '0 4px 16px rgba(0,0,0,0.1)' }}
            />
            <h1
              style={{
                fontSize: 'var(--text-xl)',
                fontWeight: 700,
                margin: 0,
                letterSpacing: '-0.02em',
              }}
            >
              SoloSoul
            </h1>
            <p
              style={{
                fontSize: 'var(--text-body-sm)',
                color: 'var(--text-tertiary)',
                margin: '6px 0 0',
                maxWidth: 280,
                marginLeft: 'auto',
                marginRight: 'auto',
                lineHeight: 1.5,
              }}
            >
              {t('common:slogan')}
            </p>
          </div>

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
                    <span
                      style={{ fontSize: 'var(--text-body-sm)', color: 'var(--text-secondary)' }}
                    >
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
                          <AlertTriangle
                            size={ICON_SIZE.xs}
                            style={{ marginTop: 2, flexShrink: 0 }}
                          />
                          <span>
                            {versionInfo.error || t('settings:update_check_failed')}
                          </span>
                        </div>
                        <button
                          type="button"
                          onClick={runCheck}
                          disabled={checking}
                          style={{
                            padding: '8px 16px',
                            borderRadius: 8,
                            border: '1px solid var(--border-subtle)',
                            background: 'var(--bg-toolbar)',
                            color: 'var(--text-primary)',
                            fontSize: 'var(--text-body-sm)',
                            fontWeight: 500,
                            fontFamily: 'inherit',
                            cursor: checking ? 'default' : 'pointer',
                            display: 'flex',
                            alignItems: 'center',
                            gap: 6,
                            alignSelf: 'flex-start',
                            opacity: checking ? 0.6 : 1,
                            transition: 'all 0.15s ease',
                          }}
                          onMouseEnter={(e) => {
                            if (checking) return;
                            e.currentTarget.style.background =
                              'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
                            e.currentTarget.style.borderColor = 'var(--accent-primary)';
                            e.currentTarget.style.color = 'var(--accent-primary)';
                          }}
                          onMouseLeave={(e) => {
                            if (checking) return;
                            e.currentTarget.style.background = 'var(--bg-toolbar)';
                            e.currentTarget.style.borderColor = 'var(--border-subtle)';
                            e.currentTarget.style.color = 'var(--text-primary)';
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
                          <Download size={ICON_SIZE.sm} />v{info.version} → v
                          {versionInfo.latestVersion}
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
                            onMouseEnter={(e) => {
                              e.currentTarget.style.background =
                                'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
                              e.currentTarget.style.borderColor = 'var(--accent-primary)';
                              e.currentTarget.style.color = 'var(--accent-primary)';
                            }}
                            onMouseLeave={(e) => {
                              e.currentTarget.style.background = 'var(--bg-toolbar)';
                              e.currentTarget.style.borderColor = 'var(--border-subtle)';
                              e.currentTarget.style.color = 'var(--text-primary)';
                            }}
                            style={{
                              padding: '8px 16px',
                              borderRadius: 8,
                              border: '1px solid var(--border-subtle)',
                              background: 'var(--bg-toolbar)',
                              color: 'var(--text-primary)',
                              fontSize: 'var(--text-body-sm)',
                              fontWeight: 500,
                              fontFamily: 'inherit',
                              cursor: 'pointer',
                              display: 'flex',
                              alignItems: 'center',
                              gap: 6,
                              alignSelf: 'flex-start',
                              transition: 'all 0.15s ease',
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
                    <span
                      style={{ fontSize: 'var(--text-body-sm)', color: 'var(--text-secondary)' }}
                    >
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

          <Card>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
              {links.map((link, i) => (
                <div key={link.url}>
                  {i > 0 && (
                    <div
                      style={{ height: 1, background: 'var(--border-subtle)', margin: '0 4px' }}
                    />
                  )}
                  <a
                    href={link.url}
                    target="_blank"
                    rel="noopener noreferrer"
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      gap: 10,
                      padding: '12px 4px',
                      borderRadius: 8,
                      color: 'var(--text-primary)',
                      fontSize: 'var(--text-sm)',
                      textDecoration: 'none',
                      transition: 'background 0.12s',
                    }}
                    onMouseEnter={(e) =>
                      (e.currentTarget.style.background = 'rgba(128,128,128,0.06)')
                    }
                    onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
                    onClick={(e) => {
                      e.preventDefault();
                      open(link.url).catch(() => {
                        // 兜底：如果 shell open 失败，仍尝试默认打开方式
                        window.open(link.url, '_blank', 'noopener,noreferrer');
                      });
                    }}
                  >
                    <span style={{ color: 'var(--text-tertiary)', display: 'flex' }}>
                      {link.icon}
                    </span>
                    <span style={{ flex: 1 }}>{t('settings:' + link.labelKey)}</span>
                    <ExternalLink
                      size={ICON_SIZE.xs}
                      style={{ color: 'var(--text-tertiary)', opacity: 0.5 }}
                    />
                  </a>
                </div>
              ))}
            </div>
          </Card>

          <div
            style={{
              textAlign: 'center',
              padding: '8px 0',
              fontSize: 'var(--text-badge)',
              color: 'var(--text-tertiary)',
              lineHeight: 1.8,
            }}
          >
            <div>Copyright &copy; {new Date().getFullYear()} SoloSoul</div>
            <div>MIT License &mdash; Open Source Software</div>
          </div>
        </PageContainer>

        {/* ── 强制更新全屏覆盖层 ── */}
        {isMandatory &&
          createPortal(
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
          )}
      </AppShell>
    </>
  );
}
