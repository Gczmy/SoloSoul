import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { AppShell } from '@/components/layout/AppShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { Card } from '@/components/ui/Card';
import { Input } from '@/components/ui/Input';
import { LoadingPlaceholder } from '@/components/ui/LoadingPlaceholder';
import { useToastError } from '@/hooks/useToastError';
import { invoke } from '@tauri-apps/api/core';
import { HardDrive, RotateCcw, Trash2, Plus } from 'lucide-react';
import { ICON_SIZE } from '@/lib/iconSizes';


interface BackupInfo {
  id: string;
  name: string;
  created_at: string;
  size_bytes: number;
  object_count: number;
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export function BackupConfigPage() {
  const navigate = useNavigate();
  const { onError, onSuccess } = useToastError();
  const { t } = useTranslation(['settings', 'common']);
  const [backups, setBackups] = useState<BackupInfo[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [backupName, setBackupName] = useState('');
  const [isCreating, setIsCreating] = useState(false);
  const [restoringId, setRestoringId] = useState<string | null>(null);

  const loadBackups = useCallback(async () => {
    setIsLoading(true);
    try {
      const list = await invoke<BackupInfo[]>('backup_list');
      setBackups(list);
    } catch (e) {
      onError(e, t('common:backups_load_failed'));
    } finally {
      setIsLoading(false);
    }
  }, [onError, t]);


  useEffect(() => {
    loadBackups();
  }, [loadBackups]);


  const handleCreate = async () => {
    if (!backupName.trim()) return;
    setIsCreating(true);
    try {
      const result = await invoke<BackupInfo>('backup_create', { name: backupName.trim() });
      onSuccess(`Backup "${result.name}" created (${formatSize(result.size_bytes)})`);
      setBackupName('');
      loadBackups();
    } catch (e) {
      onError(e, t('common:backup_failed'));
    } finally {
      setIsCreating(false);
    }
  };

  const handleRestore = async (id: string) => {
    setRestoringId(id);
    try {
      const count = await invoke<number>('backup_restore', { backupId: id });
      onSuccess(`Restored ${count} object(s) from backup`);
      loadBackups();
    } catch (e) {
      onError(e, t('common:restore_failed'));
    } finally {
      setRestoringId(null);
    }
  };

  const handleDelete = async (id: string) => {
    try {
      await invoke('backup_delete', { backupId: id });
      onSuccess(t('common:backup_deleted'));
      loadBackups();
    } catch (e) {
      onError(e, t('common:delete_failed'));
    }
  };

  return (
    <AppShell title={t('settings:backup_restore')} onBack={() => navigate('/settings')}>
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
          <p style={{ fontSize: 'var(--text-body-sm)', color: 'var(--text-secondary)', margin: '0 0 12px' }}>
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
              onMouseEnter={(e) => {
                if (backupName.trim() && !isCreating) {
                  e.currentTarget.style.background = 'color-mix(in srgb, var(--accent-primary) 12%, transparent)';
                  e.currentTarget.style.borderColor = 'var(--accent-primary)';
                  e.currentTarget.style.color = 'var(--accent-primary)';
                }
              }}
              onMouseLeave={(e) => {
                if (backupName.trim() && !isCreating) {
                  e.currentTarget.style.background = 'var(--bg-toolbar)';
                  e.currentTarget.style.borderColor = 'var(--border-subtle)';
                  e.currentTarget.style.color = 'var(--text-primary)';
                }
              }}
              style={{
                padding: '8px 16px',
                borderRadius: 8,
                border: '1px solid var(--border-subtle)',
                background: 'var(--bg-toolbar)',
                color: 'var(--text-primary)',
                fontSize: 'var(--text-body-sm)',
                fontWeight: 500,
                cursor: !backupName.trim() || isCreating ? 'default' : 'pointer',
                opacity: !backupName.trim() || isCreating ? 0.5 : 1,
                transition: 'all 0.15s ease',
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
                <><Plus size={ICON_SIZE.sm} />{t('settings:create')}</>
              )}
            </button>
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
              <p style={{ fontSize: 'var(--text-sm)', margin: 0 }}>{t('settings:no_backups_yet')}</p>
              <p style={{ fontSize: 'var(--text-caption)', margin: '4px 0 0' }}>{t('settings:create_first_backup')}</p>
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
                    <p style={{ margin: 0, fontSize: 'var(--text-sm)', fontWeight: 500 }}>{backup.name}</p>
                    <p style={{ margin: '2px 0 0', fontSize: 'var(--text-caption)', color: 'var(--text-tertiary)' }}>
                      {new Date(backup.created_at).toLocaleString()} &middot;{' '}
                      {formatSize(backup.size_bytes)} &middot;{' '}
                      {t('settings:objects_count', { n: backup.object_count })}
                    </p>
                  </div>
                  <div style={{ display: 'flex', gap: 4 }}>
                    <button
                      onClick={() => handleRestore(backup.id)}
                      disabled={restoringId === backup.id}
                      title={t('settings:restore_title')}
                      onMouseEnter={(e) => {
                        if (restoringId !== backup.id) {
                          e.currentTarget.style.background = 'color-mix(in srgb, var(--accent-primary) 12%, transparent)';
                          e.currentTarget.style.borderColor = 'var(--accent-primary)';
                          e.currentTarget.style.color = 'var(--accent-primary)';
                        }
                      }}
                      onMouseLeave={(e) => {
                        if (restoringId !== backup.id) {
                          e.currentTarget.style.background = 'var(--bg-toolbar)';
                          e.currentTarget.style.borderColor = 'var(--border-subtle)';
                          e.currentTarget.style.color = 'var(--text-secondary)';
                        }
                      }}
                      style={{
                        width: 32,
                        height: 32,
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        borderRadius: 8,
                        border: '1px solid var(--border-subtle)',
                        background: 'var(--bg-toolbar)',
                        color: 'var(--text-secondary)',
                        cursor: restoringId === backup.id ? 'default' : 'pointer',
                        opacity: restoringId === backup.id ? 0.5 : 1,
                        transition: 'all 0.15s ease',
                        padding: 0,
                      }}
                    >
                      <RotateCcw size={ICON_SIZE.sm} />
                    </button>
                    <button
                      onClick={() => handleDelete(backup.id)}
                      title={t('settings:delete_title')}
                      onMouseEnter={(e) => {
                        e.currentTarget.style.background = 'rgba(231,76,60,0.1)';
                        e.currentTarget.style.borderColor = 'rgba(231,76,60,0.3)';
                        e.currentTarget.style.color = '#e74c3c';
                      }}
                      onMouseLeave={(e) => {
                        e.currentTarget.style.background = 'var(--bg-toolbar)';
                        e.currentTarget.style.borderColor = 'var(--border-subtle)';
                        e.currentTarget.style.color = 'var(--accent-danger, #ef4444)';
                      }}
                      style={{
                        width: 32,
                        height: 32,
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        borderRadius: 8,
                        border: '1px solid var(--border-subtle)',
                        background: 'var(--bg-toolbar)',
                        color: 'var(--accent-danger, #ef4444)',
                        cursor: 'pointer',
                        transition: 'all 0.15s ease',
                        padding: 0,
                      }}
                    >
                      <Trash2 size={ICON_SIZE.sm} />
                    </button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </Card>

        {/* Info */}
        <p style={{ fontSize: 'var(--text-caption)', color: 'var(--text-tertiary)', textAlign: 'center' }}>
          {t('settings:backups_stored_locally')}
          <br />
          {t('settings:export_hint')}
        </p>
      </PageContainer>
    </AppShell>
  );
}
