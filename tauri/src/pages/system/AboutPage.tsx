import { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { invoke } from '@tauri-apps/api/core';
import { ExternalLink, Code, Shield, Info } from 'lucide-react';

interface AppInfo {
  appName: string;
  version: string;
  os: string;
  arch: string;
}

interface VersionInfo {
  currentVersion: string;
  latestVersion: string | null;
  hasUpdate: boolean;
}

function friendlyPlatform(os: string, arch: string): string {
  const osName = os === 'macos' ? 'macOS' : os === 'windows' ? 'Windows' : os === 'linux' ? 'Linux' : os;
  const archName = arch === 'aarch64' ? 'Apple Silicon' : arch === 'x86_64' ? 'Intel' : arch;
  return osName + ' (' + archName + ')';
}

export function AboutPage() {
  const navigate = useNavigate();
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [versionInfo, setVersionInfo] = useState<VersionInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const { t } = useTranslation(['settings', 'common']);

  useEffect(() => {
    Promise.all([
      invoke<AppInfo>('get_app_info'),
      invoke<VersionInfo>('check_version'),
    ])
      .then(([app, ver]) => {
        setInfo(app);
        setVersionInfo(ver);
      })
      .catch(() => setLoading(false))
      .finally(() => setLoading(false));
  }, []);

  const links = [
    { labelKey: 'github_repo', url: 'https://github.com/Gczmy/SoloSoul', icon: <Code size={14} /> },
    { labelKey: 'privacy_policy', url: 'https://github.com/Gczmy/SoloSoul/blob/main/docs/PRIVACY_POLICY.md', icon: <Shield size={14} /> },
    { labelKey: 'terms_of_service', url: 'https://github.com/Gczmy/SoloSoul/blob/main/docs/TERMS_OF_SERVICE.md', icon: <Info size={14} /> },
  ];

  return (
    <AppShell title={t('settings:about')} onBack={() => navigate('/settings')}>
      <div style={{ maxWidth: 480, margin: '0 auto', display: 'flex', flexDirection: 'column', gap: 16, padding: '12px 0' }}>
        <div style={{ textAlign: 'center', padding: '20px 0' }}>
          <div style={{
            width: 72, height: 72, borderRadius: 20,
            background: 'linear-gradient(135deg, var(--accent-primary) 0%, #8b5cf6 100%)',
            display: 'flex', alignItems: 'center', justifyContent: 'center',
            margin: '0 auto 14px', fontSize: 32, fontWeight: 700, color: 'white',
            boxShadow: '0 4px 16px rgba(0,0,0,0.1)',
          }}>
            S
          </div>
          <h1 style={{ fontSize: 24, fontWeight: 700, margin: 0, letterSpacing: '-0.02em' }}>SoloSoul</h1>
          <p style={{ fontSize: 13, color: 'var(--text-tertiary)', margin: '6px 0 0', maxWidth: 280, marginLeft: 'auto', marginRight: 'auto', lineHeight: 1.5 }}>
            {t('common:slogan')}
          </p>
        </div>

        <Card>
          <div style={{ padding: '2px 0' }}>
            {loading ? (
              <div style={{ textAlign: 'center', padding: 16, color: 'var(--text-tertiary)', fontSize: 13 }}>
                {t('settings:loading')}
              </div>
            ) : info ? (
              <div style={{ display: 'flex', flexDirection: 'column', gap: 0 }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '12px 0' }}>
                  <span style={{ fontSize: 13, color: 'var(--text-secondary)' }}>{t('settings:version')}</span>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                    <span style={{ fontSize: 14, fontWeight: 500, color: 'var(--text-primary)' }}>v{info.version}</span>
                    {versionInfo?.hasUpdate ? (
                      <span style={{ fontSize: 11, padding: '2px 8px', borderRadius: 10, background: 'rgba(230,126,34,0.15)', color: '#e67e22', fontWeight: 500 }}>
                        {t('settings:update_available', { version: versionInfo.latestVersion || '' })}
                      </span>
                    ) : versionInfo ? (
                      <span style={{ fontSize: 11, padding: '2px 8px', borderRadius: 10, background: 'rgba(39,174,96,0.12)', color: '#27ae60', fontWeight: 500 }}>
                        {t('settings:latest_version')}
                      </span>
                    ) : null}
                  </div>
                </div>
                <div style={{ height: 1, background: 'var(--border-subtle)' }} />
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '12px 0' }}>
                  <span style={{ fontSize: 13, color: 'var(--text-secondary)' }}>{t('settings:platform')}</span>
                  <span style={{ fontSize: 14, fontWeight: 500, color: 'var(--text-primary)' }}>{friendlyPlatform(info.os, info.arch)}</span>
                </div>
              </div>
            ) : (
              <div style={{ textAlign: 'center', padding: 16, color: 'var(--text-tertiary)', fontSize: 13 }}>
                {t('settings:could_not_load')}
              </div>
            )}
          </div>
        </Card>

        <Card>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
            {links.map((link, i) => (
              <div key={link.url}>
                {i > 0 && <div style={{ height: 1, background: 'var(--border-subtle)', margin: '0 4px' }} />}
                <a href={link.url} target="_blank" rel="noopener noreferrer"
                  style={{ display: 'flex', alignItems: 'center', gap: 10, padding: '12px 4px', borderRadius: 8, color: 'var(--text-primary)', fontSize: 14, textDecoration: 'none', transition: 'background 0.12s' }}
                  onMouseEnter={(e) => (e.currentTarget.style.background = 'rgba(128,128,128,0.06)')}
                  onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
                >
                  <span style={{ color: 'var(--text-tertiary)', display: 'flex' }}>{link.icon}</span>
                  <span style={{ flex: 1 }}>{t('settings:' + link.labelKey)}</span>
                  <ExternalLink size={12} style={{ color: 'var(--text-tertiary)', opacity: 0.5 }} />
                </a>
              </div>
            ))}
          </div>
        </Card>

        <div style={{ textAlign: 'center', padding: '8px 0', fontSize: 11, color: 'var(--text-tertiary)', lineHeight: 1.8 }}>
          <div>Copyright &copy; {new Date().getFullYear()} SoloSoul</div>
          <div>MIT License &mdash; Open Source Software</div>
        </div>
      </div>
    </AppShell>
  );
}
