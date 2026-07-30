import { useState, useEffect, useMemo } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { AppShell } from '@/components/layout/AppShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { Card } from '@/components/ui/Card';
import { formatBytes } from '@/lib/utils';
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

import styles from './SettingsPage.module.css';

export function SettingsPage() {
  const navigate = useNavigate();
  const { t } = useTranslation(['settings', 'common']);
  const [vaultSize, setVaultSize] = useState<string | null>(null);

  useEffect(() => {
    invoke<{ totalSizeBytes: number }>('get_vault_stats')
      .then((s) => setVaultSize(formatBytes(s.totalSizeBytes)))
      .catch(() => setVaultSize(null));
  }, []);

  const settingGroups = useMemo(() => [
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
        {
          label: t('settings:items.sync') || '设备同步',
          icon: Smartphone,
          path: '/sync',
          desc: t('settings:desc.sync') || '与其他设备同步数据',
        },
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
          path: '/ocr',
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
  ], [t, vaultSize]);

  return (
    <AppShell title={t('settings:title')} onBack={() => navigate('/home')}>
      <PageContainer variant="small" gap="large">
        {settingGroups.map((group) => (
          <div key={group.title} className={styles.settingGroup}>
            <h3 className={styles.groupTitle}>{group.title}</h3>
            <div className={styles.items}>
              {group.items.map((item) => (
                <Card
                  key={item.label}
                  interactive
                  onClick={() => navigate(item.path, { state: { from: '/settings' } })}
                >
                  <div className={styles.itemRow}>
                    <div className={styles.itemMain}>
                      <item.icon size={20} />
                      <div className={styles.itemText}>
                        <div className={styles.itemLabel}>{item.label}</div>
                        {'desc' in item && item.desc && (
                          <div className={styles.itemDesc}>{item.desc}</div>
                        )}
                      </div>
                    </div>
                    <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                      {item.badge && <span className={styles.itemBadge}>{item.badge}</span>}
                      <span className={styles.itemArrow}>›</span>
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
