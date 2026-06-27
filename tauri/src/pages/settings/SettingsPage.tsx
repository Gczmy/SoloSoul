import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { AppShell } from '@/components/layout/AppShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { Card } from '@/components/ui/Card';
import { isDevOrDebug } from '@/lib/env';
import { formatBytes } from '@/lib/format';
import {
  Shield,
  HardDrive,
  Upload,
  Trash2,
  Disc,
  ClipboardList,
  Bug,
  Info,
  Palette,
  BookOpen,
  LayoutTemplate,
  Puzzle,
  Smartphone,
  Scan,
  Paperclip,
  Search,
} from 'lucide-react';

export function SettingsPage() {
  const navigate = useNavigate();
  const { t } = useTranslation(['settings', 'common']);
  const [vaultSize, setVaultSize] = useState<string | null>(null);

  useEffect(() => {
    invoke<{ totalSizeBytes: number }>('get_vault_stats')
      .then((s) => setVaultSize(formatBytes(s.totalSizeBytes)))
      .catch(() => setVaultSize(null));
  }, []);

  const settingGroups = [
    {
      title: t('settings:groups.appearance'),
      items: [
        {
          label: t('settings:items.theme_appearance'),
          icon: Palette,
          path: '/settings/appearance',
          desc: t('settings:desc.theme_appearance'),
        },
      ],
    },
    {
      title: t('settings:groups.security'),
      items: [
        {
          label: t('settings:items.security_settings'),
          icon: Shield,
          path: '/settings/security',
          desc: t('settings:desc.security_settings'),
        },
      ],
    },
    {
      title: t('settings:groups.data'),
      items: [
        {
          label: t('settings:items.data_management'),
          icon: HardDrive,
          path: '/settings/data',
          badge: vaultSize,
          desc: t('settings:desc.data_management'),
        },
        {
          label: t('settings:items.export_import'),
          icon: Upload,
          path: '/settings/export-import',
          desc: t('settings:desc.export_import'),
        },
        {
          label: t('settings:items.trash'),
          icon: Trash2,
          path: '/settings/trash',
          desc: t('settings:desc.trash'),
        },
        {
          label: t('settings:items.backup_restore'),
          icon: Disc,
          path: '/settings/backup',
          desc: t('settings:desc.backup_restore'),
        },
        {
          label: t('settings:items.operation_log'),
          icon: ClipboardList,
          path: '/settings/operation-log',
          desc: t('settings:desc.operation_log'),
        },
        {
          label: t('settings:items.global_attachments'),
          icon: Paperclip,
          path: '/settings/attachments',
          desc: t('settings:desc.global_attachments'),
        },
        {
          label: t('settings:items.templates') || '模板管理',
          icon: LayoutTemplate,
          path: '/settings/templates',
          desc: t('settings:desc.templates') || '管理自定义对象模板',
        },
        {
          label: t('settings:items.plugins') || '插件',
          icon: Puzzle,
          path: '/plugins',
          desc: t('settings:desc.plugins') || '管理本地插件市场',
        },
        ...(isDevOrDebug()
          ? [
              {
                label: t('settings:items.sync') || '设备同步',
                icon: Smartphone,
                path: '/sync',
                desc: t('settings:desc.sync') || '与其他设备同步数据',
              },
            ]
          : []),
      ],
    },
    {
      title: t('settings:groups.system'),
      items: [
        {
          label: t('settings:items.search') || '搜索',
          icon: Search,
          path: '/search',
          desc: t('settings:desc.search') || '全局搜索',
        },
        {
          label: t('settings:items.ocr') || 'OCR',
          icon: Scan,
          path: '/settings/ocr',
          desc: t('settings:desc.ocr') || 'Manage OCR models and preferences',
        },
        {
          label: t('settings:items.help_docs'),
          icon: BookOpen,
          path: '/help',
          desc: t('settings:desc.help_docs'),
        },
        {
          label: t('settings:items.debug_log'),
          icon: Bug,
          path: '/debug-log',
          desc: t('settings:desc.debug_log'),
        },
        {
          label: t('settings:items.about'),
          icon: Info,
          path: '/about',
          desc: t('settings:desc.about'),
        },
      ],
    },
  ];

  return (
    <AppShell title={t('settings:title')} onBack={() => navigate('/home')}>
      <PageContainer variant="small" gap="large">
        {settingGroups.map((group) => (
          <div key={group.title}>
            <h3
              style={{
                fontSize: 'var(--text-body-sm)',
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
                <Card
                  key={item.label}
                  interactive
                  onClick={() => navigate(item.path, { state: { from: '/settings' } })}
                >
                  <div
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'space-between',
                    }}
                  >
                    <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
                      <item.icon size={20} />
                      <div>
                        <span style={{ fontSize: 'var(--text-sm)', fontWeight: 500 }}>{item.label}</span>
                        {'desc' in item && item.desc && (
                          <div
                            style={{ fontSize: 'var(--text-badge)', color: 'var(--text-tertiary)', marginTop: 1 }}
                          >
                            {item.desc}
                          </div>
                        )}
                      </div>
                    </div>
                    <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                      {item.badge && (
                        <span
                          style={{
                            fontSize: 'var(--text-badge)',
                            color: 'var(--text-tertiary)',
                            background: 'var(--bg-toolbar)',
                            padding: '2px 6px',
                            borderRadius: 4,
                          }}
                        >
                          {item.badge}
                        </span>
                      )}
                      <span style={{ color: 'var(--text-tertiary)', fontSize: 'var(--text-md)' }}>›</span>
                    </div>
                  </div>
                </Card>
              ))}
            </div>
          </div>
        ))}
      </PageContainer>
    </AppShell>
  );
}
