import { useNavigate } from 'react-router-dom';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';

const settingGroups = [
  {
    title: 'Security',
    items: [
      { label: 'Security Settings', icon: '🔒', path: '/settings/security' },
      { label: 'Sensitivity Settings', icon: '👁️', path: '/settings/sensitivity' },
    ],
  },
  {
    title: 'Data',
    items: [
      { label: 'Data Management', icon: '💾', path: '/settings/data' },
      { label: 'Export & Import', icon: '📤', path: '/settings/export-import' },
      { label: 'Trash', icon: '🗑️', path: '/settings/trash' },
      { label: 'Backup & Restore', icon: '💿', path: '/settings/backup' },
      { label: 'Operation Log', icon: '📋', path: '/settings/operation-log' },
    ],
  },
  {
    title: 'System',
    items: [
      { label: 'Debug Log', icon: '🐛', path: '/debug-log' },
      { label: 'About', icon: 'ℹ️', path: '/about' },
    ],
  },
];

export function SettingsPage() {
  const navigate = useNavigate();

  return (
    <AppShell title="Settings">
      <div
        style={{
          maxWidth: 600,
          margin: '0 auto',
          display: 'flex',
          flexDirection: 'column',
          gap: 24,
        }}
      >
        {settingGroups.map((group) => (
          <div key={group.title}>
            <h3
              style={{
                fontSize: 13,
                fontWeight: 600,
                color: 'var(--text-secondary)',
                textTransform: 'uppercase',
                letterSpacing: '0.05em',
                marginBottom: 8,
                paddingLeft: 4,
              }}
            >
              {group.title}
            </h3>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
              {group.items.map((item) => (
                <Card key={item.label} interactive onClick={() => navigate(item.path)}>
                  <div
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'space-between',
                    }}
                  >
                    <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
                      <span style={{ fontSize: 18 }}>{item.icon}</span>
                      <span style={{ fontSize: 14, fontWeight: 500 }}>{item.label}</span>
                    </div>
                    <span style={{ color: 'var(--text-tertiary)', fontSize: 18 }}>›</span>
                  </div>
                </Card>
              ))}
            </div>
          </div>
        ))}
      </div>
    </AppShell>
  );
}
