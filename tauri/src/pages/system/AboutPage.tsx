import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { ShieldLogo } from '@/components/ui/ShieldLogo';
import { invoke } from '@tauri-apps/api/core';
import { ExternalLink, Code, Shield, Info, Download } from 'lucide-react';
import { LoadingPlaceholder } from '@/components/ui/LoadingPlaceholder';
import { checkForUpdate, downloadAndInstallUpdate, type UpdateProgress } from '@/lib/updater';
import { formatBytes } from '@/lib/format';

interface AppInfo {
  appName: string;
  version: string;
  os: string;
  arch: string;
}

interface VersionInfo {
  currentVersion: string;
  latestVersion: string | null;
  state: 'up-to-date' | 'available' | 'error';
  body?: string;
  error?: string;
}

function friendlyPlatform(os: string, _arch: string): string {
  return os === 'macos' ? 'macOS' : os === 'windows' ? 'Windows' : os === 'linux' ? 'Linux' : os;
}

export function AboutPage() {
  const navigate = useNavigate();
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [versionInfo, setVersionInfo] = useState<VersionInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const { t, i18n } = useTranslation(['settings', 'common']);
  const docLang = i18n.language?.startsWith('zh') ? 'zh-CN' : 'en-US';

  // 更新下载/安装状态
  const [downloading, setDownloading] = useState(false);
  const [downloadProgress, setDownloadProgress] = useState<UpdateProgress | null>(null);
  const [downloadedBytes, setDownloadedBytes] = useState(0);
  const [totalBytes, setTotalBytes] = useState(0);
  const [downloadError, setDownloadError] = useState<string | null>(null);

  useEffect(() => {
    Promise.all([
      invoke<AppInfo>('get_app_info'),
      checkForUpdate().then((result) => {
        if (result.kind === 'available') {
          return {
            currentVersion: '',
            latestVersion: result.info.version,
            state: 'available' as const,
            body: result.info.body,
          };
        }
        if (result.kind === 'error') {
          return {
            currentVersion: '',
            latestVersion: null,
            state: 'error' as const,
            error: result.message,
          };
        }
        return { currentVersion: '', latestVersion: null, state: 'up-to-date' as const };
      }),
    ])
      .then(([app, ver]) => {
        setInfo(app);
        setVersionInfo({ ...ver, currentVersion: app.version });
      })
      .catch(() => setLoading(false))
      .finally(() => setLoading(false));
  }, []);

  const handleUpdate = useCallback(async () => {
    setDownloading(true);
    setDownloadError(null);
    setDownloadProgress(null);
    setDownloadedBytes(0);
    setTotalBytes(0);
    try {
      await downloadAndInstallUpdate((progress) => {
        setDownloadProgress(progress);
        if (progress.event === 'Started') {
          setTotalBytes(progress.data.contentLength ?? 0);
        } else if (progress.event === 'Progress') {
          setDownloadedBytes((prev) => prev + (progress.data.chunkLength ?? 0));
        }
      });
      // 安装成功后会调用 relaunch()，通常不会执行到这里
    } catch (err) {
      setDownloading(false);
      setDownloadError(err instanceof Error ? err.message : String(err));
    }
  }, []);

  const links = [
    { labelKey: 'github_repo', url: 'https://github.com/Gczmy/SoloSoul', icon: <Code size={14} /> },
    {
      labelKey: 'privacy_policy',
      url: `https://github.com/Gczmy/SoloSoul/blob/master/docs/${docLang}/PRIVACY_POLICY.md`,
      icon: <Shield size={14} />,
    },
    {
      labelKey: 'terms_of_service',
      url: `https://github.com/Gczmy/SoloSoul/blob/master/docs/${docLang}/TERMS_OF_SERVICE.md`,
      icon: <Info size={14} />,
    },
  ];

  const progressPercent =
    totalBytes > 0 ? Math.min(Math.round((downloadedBytes / totalBytes) * 100), 100) : 0;

  return (
    <AppShell title={t('settings:about')} onBack={() => navigate('/settings')}>
      <div
        style={{
          maxWidth: 480,
          margin: '0 auto',
          display: 'flex',
          flexDirection: 'column',
          gap: 16,
          padding: '12px 0',
        }}
      >
        <div style={{ textAlign: 'center', padding: '20px 0' }}>
          <ShieldLogo
            size={72}
            style={{ margin: '0 auto 14px', boxShadow: '0 4px 16px rgba(0,0,0,0.1)' }}
          />
          <h1 style={{ fontSize: 24, fontWeight: 700, margin: 0, letterSpacing: '-0.02em' }}>
            SoloSoul
          </h1>
          <p
            style={{
              fontSize: 13,
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
                  <span style={{ fontSize: 13, color: 'var(--text-secondary)' }}>
                    {t('settings:version')}
                  </span>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                    <span style={{ fontSize: 14, fontWeight: 500, color: 'var(--text-primary)' }}>
                      v{info.version}
                    </span>
                    {versionInfo?.state === 'available' ? (
                      <span
                        style={{
                          fontSize: 11,
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
                          fontSize: 11,
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
                          fontSize: 11,
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
                          fontSize: 13,
                          fontWeight: 600,
                          display: 'flex',
                          alignItems: 'center',
                          gap: 6,
                        }}
                      >
                        <Download size={14} />v{info.version} → v{versionInfo.latestVersion}
                      </div>
                      {versionInfo.body && (
                        <div
                          style={{
                            fontSize: 12,
                            color: 'var(--text-secondary)',
                            whiteSpace: 'pre-wrap',
                            lineHeight: 1.5,
                          }}
                        >
                          {versionInfo.body}
                        </div>
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
                          <span style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>
                            {downloadProgress?.event === 'Finished'
                              ? t('settings:installing') || 'Installing...'
                              : `${formatBytes(downloadedBytes)} / ${formatBytes(totalBytes)} (${progressPercent}%)`}
                          </span>
                        </div>
                      ) : (
                        <button
                          onClick={handleUpdate}
                          onMouseEnter={(e) => {
                            e.currentTarget.style.background = 'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
                            e.currentTarget.style.borderColor = 'var(--accent-primary)';
                          }}
                          onMouseLeave={(e) => {
                            e.currentTarget.style.background = 'var(--bg-toolbar)';
                            e.currentTarget.style.borderColor = 'var(--border-subtle)';
                          }}
                          style={{
                            padding: '8px 16px',
                            borderRadius: 8,
                            border: '1px solid var(--border-subtle)',
                            background: 'var(--bg-toolbar)',
                            color: 'var(--text-primary)',
                            fontSize: 13,
                            fontWeight: 500,
                            cursor: 'pointer',
                            display: 'flex',
                            alignItems: 'center',
                            gap: 6,
                            alignSelf: 'flex-start',
                            transition: 'background 0.2s, border-color 0.2s',
                          }}
                        >
                          <Download size={14} />
                          {t('settings:update_now') || 'Update Now'}
                        </button>
                      )}
                      {downloadError && (
                        <div style={{ fontSize: 12, color: 'var(--error)' }}>{downloadError}</div>
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
                  <span style={{ fontSize: 13, color: 'var(--text-secondary)' }}>
                    {t('settings:platform')}
                  </span>
                  <span style={{ fontSize: 14, fontWeight: 500, color: 'var(--text-primary)' }}>
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
                  fontSize: 13,
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
                  <div style={{ height: 1, background: 'var(--border-subtle)', margin: '0 4px' }} />
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
                    fontSize: 14,
                    textDecoration: 'none',
                    transition: 'background 0.12s',
                  }}
                  onMouseEnter={(e) =>
                    (e.currentTarget.style.background = 'rgba(128,128,128,0.06)')
                  }
                  onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
                >
                  <span style={{ color: 'var(--text-tertiary)', display: 'flex' }}>
                    {link.icon}
                  </span>
                  <span style={{ flex: 1 }}>{t('settings:' + link.labelKey)}</span>
                  <ExternalLink size={12} style={{ color: 'var(--text-tertiary)', opacity: 0.5 }} />
                </a>
              </div>
            ))}
          </div>
        </Card>

        <div
          style={{
            textAlign: 'center',
            padding: '8px 0',
            fontSize: 11,
            color: 'var(--text-tertiary)',
            lineHeight: 1.8,
          }}
        >
          <div>Copyright &copy; {new Date().getFullYear()} SoloSoul</div>
          <div>MIT License &mdash; Open Source Software</div>
        </div>
      </div>
    </AppShell>
  );
}
