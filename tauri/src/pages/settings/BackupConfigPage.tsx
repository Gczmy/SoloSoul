import { useState, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { useShallow } from 'zustand/react/shallow';
import { PageShell } from '@/components/layout/PageShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { Card } from '@/components/ui/Card';
import { Input } from '@/components/ui/Input';
import { LoadingPlaceholder } from '@/components/ui/LoadingPlaceholder';
import { useToastError } from '@/hooks/useToastError';
import { useConfirm } from '@/hooks/useConfirm';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import type { BackupInfo } from '@/types/backup';
import { usePrefetchData } from '@/lib/prefetch/usePrefetchData';
import { prefetchRegistry } from '@/lib/prefetch/registry';
import { HardDrive, RotateCcw, Plus, Bell, Info } from 'lucide-react';
import { DeleteButton } from '@/components/ui/DeleteButton';
import { ICON_SIZE } from '@/lib/constants';
import { formatBytes } from '@/lib/utils';
import { useSettingsStore } from '@/stores/settingsStore';
import { useAuthStore } from '@/stores/authStore';
import { PageGuideButton } from '@/components/guide/PageGuideButton';

export function BackupConfigPage() {
  const navigate = useNavigate();
  const { onError, onSuccess } = useToastError();
  const { t } = useTranslation(['settings', 'common']);
  const { requestConfirm, dialog: confirmDialog } = useConfirm();
  // Prefetch Runtime: 备份列表共享缓存（预热后直接渲染）；变更操作后 reload 刷新
  const { data: backups, reload } = usePrefetchData(prefetchRegistry.backups);
  const [backupName, setBackupName] = useState('');
  const [isCreating, setIsCreating] = useState(false);
  const [restoringId, setRestoringId] = useState<string | null>(null);

  const currentAccount = useAuthStore((s) => s.currentAccount);
  // P022: useShallow 字段级选择——避免 store 无关字段翻转时整页重渲染
  const { settings, updateSetting } = useSettingsStore(
    useShallow((s) => ({ settings: s.settings, updateSetting: s.updateSetting })),
  );

  const backupGuidePages = useMemo(
    () => [
      {
        icon: Info,
        title: t('common:guide_backup_title', { defaultValue: 'Backup & Restore Guide' }),
        steps: [
          {
            icon: HardDrive,
            title: t('common:guide_backup_step1_title', { defaultValue: 'Create Backup' }),
            description:
              t('common:guide_backup_step1_desc', { defaultValue: 'Create a local backup of your current profile. Backups are stored on this device.' }),
          },
          {
            icon: RotateCcw,
            title: t('common:guide_backup_step2_title', { defaultValue: 'Restore Backup' }),
            description:
              t('common:guide_backup_step2_desc', { defaultValue: 'Select a backup and restore it. Restoring may overwrite existing data in the current profile.' }),
          },
          {
            icon: Bell,
            title: t('common:guide_backup_step3_title', { defaultValue: 'Manage Backups' }),
            description:
              t('common:guide_backup_step3_desc', { defaultValue: 'View backup details, delete old backups, or recover from a previous state.' }),
          },
        ],
        helpLinks: [
          {
            title: t('common:guide_help_backup_restore', { defaultValue: 'Backup & Restore' }),
            description:
              t('common:guide_help_backup_restore_desc', { defaultValue: 'Create and restore local profile backups' }),
            href: '/help?id=backup_restore',
          },
        ],
      },
    ],
    [t],
  );

  // 数据未就绪（含加载失败）视为加载中，与旧「初始 isLoading=true」语义一致
  const isLoading = backups === null;

  const handleCreate = async () => {
    if (!backupName.trim()) return;
    setIsCreating(true);
    try {
      const result = await invoke<BackupInfo>('backup_create', { name: backupName.trim() });
      onSuccess(t('settings:backup_created', { name: result.name, size: formatBytes(result.size_bytes) }));
      setBackupName('');
      reload();
    } catch (e) {
      onError(e, t('common:backup_failed'));
    } finally {
      setIsCreating(false);
    }
  };

  const handleRestore = (id: string, name: string) => {
    requestConfirm(
      t('settings:backup_restore_confirm_title'),
      t('settings:backup_restore_confirm_body', { name }),
      async () => {
        setRestoringId(id);
        try {
          const count = await invoke<number>('backup_restore', { backupId: id });
          onSuccess(t('settings:restored_from_backup', { count }));
          reload();
        } catch (e) {
          onError(e, t('common:restore_failed'));
        } finally {
          setRestoringId(null);
        }
      },
      { confirmLabel: t('common:restore'), cancelLabel: t('common:cancel') },
    );
  };

  const handleDelete = (id: string, name: string) => {
    requestConfirm(
      t('settings:backup_delete_confirm_title'),
      t('settings:backup_delete_confirm_body', { name }),
      async () => {
        try {
          await invoke('backup_delete', { backupId: id });
          onSuccess(t('common:backup_deleted'));
          reload();
        } catch (e) {
          onError(e, t('common:delete_failed'));
        }
      },
      { confirmLabel: t('common:delete'), cancelLabel: t('common:cancel') },
    );
  };

  return (
    <PageShell
      title={t('settings:backup_restore')}
      onBack={() => navigate('/settings')}
      actions={<PageGuideButton pages={backupGuidePages} />}
    >
      <PageContainer variant="xs" gap="section">
        {/* Create Backup */}
        <Card>
          <h3
            style={{
              fontSize: 'var(--text-sm)',
              fontWeight: 600,
              margin: '0 0 4px',
              display: 'flex',
              alignItems: 'center',
              gap: 6,
            }}
          >
            <HardDrive size={ICON_SIZE.md} />
            {t('settings:create_backup_title')}
          </h3>
          <p
            style={{
              fontSize: 'var(--text-body-sm)',
              color: 'var(--text-secondary)',
              margin: '0 0 12px',
            }}
          >
            {t('settings:backup_hint')}
          </p>
          <div style={{ display: 'flex', gap: 8 }}>
            <Input
              value={backupName}
              onChange={(e) => setBackupName(e.target.value)}
              placeholder={t('settings:backup_name_placeholder')}
              style={{ flex: 1 }}
            />
            <button
              onClick={handleCreate}
              disabled={!backupName.trim() || isCreating}
              className="interactive-toolbar"
              style={{
                padding: '8px 16px',
                borderRadius: 8,
                borderWidth: 1,
                borderStyle: 'solid',
                fontSize: 'var(--text-body-sm)',
                fontWeight: 500,
                cursor: !backupName.trim() || isCreating ? 'default' : 'pointer',
                opacity: !backupName.trim() || isCreating ? 0.5 : 1,
                fontFamily: 'inherit',
                display: 'inline-flex',
                alignItems: 'center',
                gap: 6,
                whiteSpace: 'nowrap',
              }}
            >
              {isCreating ? (
                t('common:loading', { defaultValue: '...' })
              ) : (
                <>
                  <Plus size={ICON_SIZE.sm} />
                  {t('settings:create')}
                </>
              )}
            </button>
          </div>
        </Card>

        {/* Backup Reminder */}
        <Card>
          <h3 style={{ fontSize: 'var(--text-sm)', fontWeight: 600, margin: '0 0 4px' }}>
            <Bell size={ICON_SIZE.md} style={{ marginRight: 6, verticalAlign: 'text-bottom' }} />
            {t('settings:backup_reminder')}
          </h3>
          <p
            style={{
              fontSize: 'var(--text-body-sm)',
              color: 'var(--text-secondary)',
              margin: '0 0 12px',
            }}
          >
            {t('settings:backup_reminder_desc')}
          </p>
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
            <span style={{ fontSize: 'var(--text-sm)' }}>{t('settings:backup_reminder_days')}</span>
            <select
              value={settings.backupReminderDays}
              onChange={(e) => {
                const value = parseInt(e.target.value, 10);
                if (currentAccount?.id) {
                  updateSetting(currentAccount.id, 'backupReminderDays', value);
                }
              }}
              style={{
                padding: '6px 10px',
                borderRadius: 8,
                border: '1px solid var(--border-subtle)',
                background: 'var(--bg-toolbar)',
                color: 'var(--text-primary)',
                fontFamily: 'inherit',
                fontSize: 'var(--text-body-sm)',
              }}
            >
              <option value="0">{t('settings:backup_reminder_off')}</option>
              <option value="1">{t('settings:backup_reminder_1d')}</option>
              <option value="3">{t('settings:backup_reminder_3d')}</option>
              <option value="7">{t('settings:backup_reminder_7d')}</option>
              <option value="15">{t('settings:backup_reminder_15d')}</option>
              <option value="30">{t('settings:backup_reminder_30d')}</option>
            </select>
          </div>
        </Card>

        {/* Backup List */}
        <Card>
          <h3 style={{ fontSize: 'var(--text-sm)', fontWeight: 600, margin: '0 0 4px' }}>
            {t('settings:saved_backups')}
          </h3>

          {isLoading ? (
            <LoadingPlaceholder variant="elevated" minHeight={120} />
          ) : backups.length === 0 ? (
            <div style={{ padding: 24, textAlign: 'center', color: 'var(--text-tertiary)' }}>
              <HardDrive size={ICON_SIZE['2xl']} style={{ marginBottom: 8, opacity: 0.4 }} />
              <p style={{ fontSize: 'var(--text-sm)', margin: 0 }}>
                {t('settings:no_backups_yet')}
              </p>
              <p style={{ fontSize: 'var(--text-caption)', margin: '4px 0 0' }}>
                {t('settings:create_first_backup')}
              </p>
            </div>
          ) : (
            <div style={{ marginTop: 4 }}>
              {backups.map((backup) => (
                <div
                  key={backup.id}
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: 12,
                    padding: '10px 0',
                    borderBottom: '1px solid var(--border-subtle)',
                  }}
                >
                  <div style={{ flex: 1, minWidth: 0 }}>
                    <p style={{ margin: 0, fontSize: 'var(--text-sm)', fontWeight: 500 }}>
                      {backup.name}
                    </p>
                    <p
                      style={{
                        margin: '2px 0 0',
                        fontSize: 'var(--text-caption)',
                        color: 'var(--text-tertiary)',
                      }}
                    >
                      {new Date(backup.created_at).toLocaleString()} &middot;{' '}
                      {formatBytes(backup.size_bytes)} &middot;{' '}
                      {t('settings:objects_count', { n: backup.object_count })}
                    </p>
                  </div>
                  <div style={{ display: 'flex', gap: 4 }}>
                    <button
                      onClick={() => handleRestore(backup.id, backup.name)}
                      disabled={restoringId === backup.id}
                      title={t('settings:restore_title')}
                      className="interactive-toolbar"
                      style={{
                        width: 32,
                        height: 32,
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        borderRadius: 8,
                        borderWidth: 1,
                        borderStyle: 'solid',
                        cursor: restoringId === backup.id ? 'default' : 'pointer',
                        opacity: restoringId === backup.id ? 0.5 : 1,
                        padding: 0,
                      }}
                    >
                      <RotateCcw size={ICON_SIZE.sm} />
                    </button>
                    <DeleteButton
                      onClick={() => handleDelete(backup.id, backup.name)}
                      title={t('settings:delete_title')}
                      iconOnly
                    />
                  </div>
                </div>
              ))}
            </div>
          )}
        </Card>

        {confirmDialog}

        {/* Info */}
        <p
          style={{
            fontSize: 'var(--text-caption)',
            color: 'var(--text-tertiary)',
            textAlign: 'center',
          }}
        >
          {t('settings:backups_stored_locally')}
          <br />
          {t('settings:export_hint')}
        </p>
      </PageContainer>
    </PageShell>
  );
}
