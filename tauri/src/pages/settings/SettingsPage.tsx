import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { Shield, Eye, HardDrive, Upload, Trash2, Disc, ClipboardList, Bug, Info, Palette } from 'lucide-react';

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

export function SettingsPage() {
  const navigate = useNavigate();
  const { t } = useTranslation(['settings', 'common']);
  const [vaultSize, setVaultSize] = useState<string | null>(null);

  useEffect(() => {
    invoke<{ total_size_bytes: number }>('get_vault_stats')
      .then((s) => setVaultSize(formatBytes(s.total_size_bytes)))
      .catch(() => setVaultSize(null));
  }, []);

  const settingGroups = [
    {
      title: t('settings:groups.appearance'),
      items: [
        { label: t('settings:items.theme_appearance'), icon: Palette, path: '/settings/appearance' },
      ],
    },
    {
      title: t('settings:groups.security'),
      items: [
        { label: t('settings:items.security_settings'), icon: Shield, path: '/settings/security' },
        { label: t('settings:items.sensitivity_settings'), icon: Eye, path: '/settings/sensitivity' },
      ],
    },
    {
      title: t('settings:groups.data'),
      items: [
        { label: t('settings:items.data_management'), icon: HardDrive, path: '/settings/data', badge: vaultSize },
        { label: t('settings:items.export_import'), icon: Upload, path: '/settings/export-import' },
        { label: t('settings:items.trash'), icon: Trash2, path: '/settings/trash' },
        { label: t('settings:items.backup_restore'), icon: Disc, path: '/settings/backup' },
        { label: t('settings:items.operation_log'), icon: ClipboardList, path: '/settings/operation-log' },
      ],
    },
    {
      title: t('settings:groups.system'),
      items: [
        { label: t('settings:items.debug_log'), icon: Bug, path: '/debug-log' },
        { label: t('settings:items.about'), icon: Info, path: '/about' },
      ],
    },
  ];

  return (
    <AppShell title={t('settings:title')}>
      <div style={{ maxWidth: 600, margin: '0 auto', display: 'flex', flexDirection: 'column', gap: 24 }}>
        {settingGroups.map((group) => (
          <div key={group.title}>
            <h3 style={{
              fontSize: 13, fontWeight: 600, color: 'var(--text-secondary)',
              textTransform: 'uppercase', letterSpacing: '0.05em', marginBottom: 8, paddingLeft: 4,
            }}>
              {group.title}
            </h3>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
              {group.items.map((item) => (
                <Card key={item.label} interactive onClick={() => navigate(item.path)}>
                  <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
                      <item.icon size={20} />
                      <span style={{ fontSize: 14, fontWeight: 500 }}>{item.label}</span>
                    </div>
                    <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                      {item.badge && (
                        <span style={{ fontSize: 11, color: 'var(--text-tertiary)', background: 'var(--bg-toolbar)', padding: '2px 6px', borderRadius: 4 }}>
                          {item.badge}
                        </span>
                      )}
                      <span style={{ color: 'var(--text-tertiary)', fontSize: 18 }}>›</span>
                    </div>
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
