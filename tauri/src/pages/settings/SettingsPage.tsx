import { useNavigate } from 'react-router-dom';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { Shield, Eye, HardDrive, Upload, Trash2, Disc, ClipboardList, Bug, Info } from 'lucide-react';

const settingGroups = [
  {
    title: 'Security',
    items: [
      { label: 'Security Settings', icon: Shield, path: '/settings/security' },
      { label: 'Sensitivity Settings', icon: Eye, path: '/settings/sensitivity' },
    ],
  },
  {
    title: 'Data',
    items: [
      { label: 'Data Management', icon: HardDrive, path: '/settings/data' },
      { label: 'Export & Import', icon: Upload, path: '/settings/export-import' },
      { label: 'Trash', icon: Trash2, path: '/settings/trash' },
      { label: 'Backup & Restore', icon: Disc, path: '/settings/backup' },
      { label: 'Operation Log', icon: ClipboardList, path: '/settings/operation-log' },
    ],
  },
  {
    title: 'System',
    items: [
      { label: 'Debug Log', icon: Bug, path: '/debug-log' },
      { label: 'About', icon: Info, path: '/about' },
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
                      <item.icon size={20} />
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
