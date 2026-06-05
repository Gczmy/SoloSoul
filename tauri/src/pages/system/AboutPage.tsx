import { useState, useEffect } from 'react';
import { AppShell } from '@/components/layout/AppShell';
import { GlassCard } from '@/components/liquid-glass/GlassCard';
import { invoke } from '@tauri-apps/api/core';
import { ExternalLink, Code2 } from 'lucide-react';
import pkg from '../../../package.json';

interface AppInfo {
  appName: string;
  version: string;
  buildNumber: string;
  os: string;
  arch: string;
}

export function AboutPage() {
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    invoke<AppInfo>('get_app_info')
      .then(setInfo)
      .catch(() => setLoading(false))
      .finally(() => setLoading(false));
  }, []);

  const links = [
    { label: 'GitHub Repository', url: 'https://github.com/Gczmy/SoloSoul', icon: <Code2 size={14} /> },
    { label: 'Privacy Policy', url: 'https://github.com/Gczmy/SoloSoul/blob/main/docs/PRIVACY_POLICY.md', icon: <ExternalLink size={14} /> },
    { label: 'Terms of Service', url: 'https://github.com/Gczmy/SoloSoul/blob/main/docs/TERMS_OF_SERVICE.md', icon: <ExternalLink size={14} /> },
  ];

  return (
    <AppShell title="About" onBack={() => window.history.back()}>
      <div style={{ maxWidth: 480, margin: '0 auto', display: 'flex', flexDirection: 'column', gap: 16, alignItems: 'center' }}>
        {/* Logo + Name */}
        <div style={{ textAlign: 'center', marginTop: 24 }}>
          <div style={{
            width: 64, height: 64, borderRadius: 16,
            background: 'linear-gradient(135deg, var(--accent-primary), var(--accent-secondary, #8b5cf6))',
            display: 'flex', alignItems: 'center', justifyContent: 'center',
            margin: '0 auto 12px', fontSize: 28, fontWeight: 700, color: 'white',
          }}>
            S
          </div>
          <h1 style={{ fontSize: 22, fontWeight: 700, margin: 0 }}>SoloSoul</h1>
          <p style={{ fontSize: 13, color: 'var(--text-secondary)', margin: '4px 0 0' }}>
            Local Digital Twin & Universal Identity Engine
          </p>
        </div>

        {/* Version Info */}
        <div style={{ width: '100%' }}>
          <GlassCard>
            {loading ? (
              <div style={{ textAlign: 'center', padding: 16, color: 'var(--text-tertiary)', fontSize: 13 }}>
                Loading...
              </div>
            ) : info ? (
              <div style={{ display: 'flex', flexDirection: 'column', gap: 8, fontSize: 14 }}>
                <Row label="Version" value={`v${info.version}`} />
                <Row label="Build" value={info.buildNumber} />
                <Row label="Platform" value={`${info.os} (${info.arch})`} />
                <Row label="Frontend" value={`React ${pkg.dependencies?.react?.replace('^', '') || '19'}`} />
              </div>
            ) : (
              <div style={{ textAlign: 'center', padding: 16, color: 'var(--text-tertiary)', fontSize: 13 }}>
                Could not load app info
              </div>
            )}
          </GlassCard>
        </div>

        {/* Links */}
        <div style={{ width: '100%' }}>
          <GlassCard>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
            {links.map((link) => (
              <a
                key={link.url}
                href={link.url}
                target="_blank"
                rel="noopener noreferrer"
                style={{
                  display: 'flex', alignItems: 'center', gap: 8,
                  padding: '10px 12px', borderRadius: 8,
                  color: 'var(--accent-primary)', fontSize: 14,
                  textDecoration: 'none', transition: 'background 0.15s',
                }}
                onMouseEnter={(e) => (e.currentTarget.style.background = 'var(--bg-subtle, rgba(128,128,128,0.06))')}
                onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
              >
                {link.icon}
                <span style={{ flex: 1 }}>{link.label}</span>
                <ExternalLink size={12} style={{ color: 'var(--text-tertiary)' }} />
              </a>
            ))}
          </div>
          </GlassCard>
        </div>

        {/* License */}
        <p style={{ fontSize: 12, color: 'var(--text-tertiary)', textAlign: 'center' }}>
          Copyright &copy; {new Date().getFullYear()} SoloSoul Team. All rights reserved.
          <br />
          MIT License — Open Source Software
        </p>
      </div>
    </AppShell>
  );
}

function Row({ label, value }: { label: string; value: string }) {
  return (
    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
      <span style={{ color: 'var(--text-secondary)' }}>{label}</span>
      <span style={{ fontWeight: 500 }}>{value}</span>
    </div>
  );
}
