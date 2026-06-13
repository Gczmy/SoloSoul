import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { ExternalLink, Code, Shield, Info, Download } from 'lucide-react';
import { LoadingPlaceholder } from '@/components/ui/LoadingPlaceholder';

interface AppInfo {
  appName: string;
  version: string;
  os: string;
  arch: string;
}

interface UpdateAsset {
  name: string;
  downloadUrl: string;
  size: number;
}

interface VersionInfo {
  currentVersion: string;
  latestVersion: string | null;
  hasUpdate: boolean;
  assets: UpdateAsset[];
}

interface DownloadProgress {
  downloaded: number;
  total: number;
}

function friendlyPlatform(os: string, _arch: string): string {
  return os === 'macos' ? 'macOS' : os === 'windows' ? 'Windows' : os === 'linux' ? 'Linux' : os;
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export function AboutPage() {
  const navigate = useNavigate();
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [versionInfo, setVersionInfo] = useState<VersionInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const { t, i18n } = useTranslation(['settings', 'common']);
  const docLang = i18n.language?.startsWith('zh') ? 'zh-CN' : 'en-US';

  // Download state
  const [downloading, setDownloading] = useState(false);
  const [downloadProgress, setDownloadProgress] = useState<DownloadProgress | null>(null);
  const [downloadComplete, setDownloadComplete] = useState(false);

  useEffect(() => {
    Promise.all([invoke<AppInfo>('get_app_info'), invoke<VersionInfo>('check_version')])
      .then(([app, ver]) => {
        setInfo(app);
        setVersionInfo(ver);
      })
      .catch(() => setLoading(false))
      .finally(() => setLoading(false));
  }, []);

  // Listen for download progress events
  useEffect(() => {
    const unlisten = listen<DownloadProgress>('update-download-progress', (event) => {
      setDownloadProgress(event.payload);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const handleUpdate = useCallback(async () => {
    if (!versionInfo || versionInfo.assets.length === 0) return;
    const asset = versionInfo.assets[0];
    setDownloading(true);
    setDownloadProgress(null);
    setDownloadComplete(false);
    try {
      const path = await invoke<string>('download_update', {
        assetName: asset.name,
        assetUrl: asset.downloadUrl,
      });
      setDownloadComplete(true);
      // Open the downloaded file
      const { open } = await import('@tauri-apps/plugin-shell');
      await open(path);
      // Reset after a short delay
      setTimeout(() => {
        setDownloading(false);
        setDownloadProgress(null);
      }, 2000);
    } catch {
      setDownloading(false);
      setDownloadProgress(null);
    }
  }, [versionInfo]);

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

  const updateAsset = versionInfo?.assets?.[0];
  const progressPercent =
    downloadProgress && downloadProgress.total > 0
      ? Math.round((downloadProgress.downloaded / downloadProgress.total) * 100)
      : 0;

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
          <div
            style={{
              width: 72,
              height: 72,
              borderRadius: 16,
              background: 'linear-gradient(135deg, var(--accent-primary), var(--accent-warm))',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              margin: '0 auto 14px',
              fontSize: 34,
              fontWeight: 700,
              color: 'white',
              boxShadow: '0 4px 16px rgba(0,0,0,0.1)',
            }}
          >
            S
          </div>
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
                    {versionInfo?.hasUpdate ? (
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

                {/* Update card — shown when an update is available */}
                {versionInfo?.hasUpdate && updateAsset && (
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
                      <div style={{ fontSize: 12, color: 'var(--text-secondary)' }}>
                        {updateAsset.name} ({formatBytes(updateAsset.size)})
                      </div>

                      {/* Download button or progress */}
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
                            {downloadComplete
                              ? t('settings:download_complete') || 'Download complete'
                              : downloadProgress
                                ? `${formatBytes(downloadProgress.downloaded)} / ${formatBytes(downloadProgress.total)} (${progressPercent}%)`
                                : t('settings:downloading') || 'Downloading...'}
                          </span>
                        </div>
                      ) : (
                        <button
                          onClick={handleUpdate}
                          style={{
                            padding: '8px 16px',
                            borderRadius: 8,
                            border: 'none',
                            background: 'var(--accent-primary)',
                            color: 'white',
                            fontSize: 13,
                            fontWeight: 500,
                            cursor: 'pointer',
                            display: 'flex',
                            alignItems: 'center',
                            gap: 6,
                            alignSelf: 'flex-start',
                          }}
                        >
                          <Download size={14} />
                          {t('settings:update_now') || 'Update Now'}
                        </button>
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
