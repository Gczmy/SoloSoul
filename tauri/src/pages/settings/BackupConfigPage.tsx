import { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { useToastError } from '@/hooks/useToastError';
import { invoke } from '@tauri-apps/api/core';
import {
  HardDrive, RotateCcw, Trash2, Plus,
} from 'lucide-react';

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

  useEffect(() => {
    loadBackups();
  }, []);

  const loadBackups = async () => {
    setIsLoading(true);
    try {
      const list = await invoke<BackupInfo[]>('backup_list');
      setBackups(list);
    } catch (e) {
      onError(e, t('common:backups_load_failed'));
    } finally {
      setIsLoading(false);
    }
  };

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
      <div style={{ maxWidth: 560, margin: '0 auto', display: 'flex', flexDirection: 'column', gap: 16 }}>
        {/* Create Backup */}
        <Card>
          <h3 style={{ fontSize: 14, fontWeight: 600, margin: '0 0 4px', display: 'flex', alignItems: 'center', gap: 6 }}>
            <HardDrive size={16} />
            {t('settings:create_backup_title')}
          </h3>
          <p style={{ fontSize: 13, color: 'var(--text-secondary)', margin: '0 0 12px' }}>
            {t('settings:backup_hint')}
          </p>
          <div style={{ display: 'flex', gap: 8 }}>
            <Input
              value={backupName}
              onChange={(e) => setBackupName(e.target.value)}
              placeholder={t('settings:backup_name_placeholder')}
              style={{ flex: 1 }}
            />
            <Button onClick={handleCreate} loading={isCreating} disabled={!backupName.trim()}>
              <Plus size={14} />
              {t('settings:create')}
            </Button>
          </div>
        </Card>

        {/* Backup List */}
        <Card>
          <h3 style={{ fontSize: 14, fontWeight: 600, margin: '0 0 4px' }}>
            {t('settings:saved_backups')}
          </h3>

          {isLoading ? (
            <div style={{ padding: 24, textAlign: 'center', color: 'var(--text-tertiary)', fontSize: 13 }}>
              {t('settings:loading')}
            </div>
          ) : backups.length === 0 ? (
            <div style={{ padding: 24, textAlign: 'center', color: 'var(--text-tertiary)' }}>
              <HardDrive size={24} style={{ marginBottom: 8, opacity: 0.4 }} />
              <p style={{ fontSize: 14, margin: 0 }}>{t('settings:no_backups_yet')}</p>
              <p style={{ fontSize: 12, margin: '4px 0 0' }}>
                {t('settings:create_first_backup')}
              </p>
            </div>
          ) : (
            <div style={{ marginTop: 4 }}>
              {backups.map((backup) => (
                <div
                  key={backup.id}
                  style={{
                    display: 'flex', alignItems: 'center', gap: 12,
                    padding: '10px 0', borderBottom: '1px solid var(--border-subtle)',
                  }}
                >
                  <div style={{ flex: 1, minWidth: 0 }}>
                    <p style={{ margin: 0, fontSize: 14, fontWeight: 500 }}>{backup.name}</p>
                    <p style={{ margin: '2px 0 0', fontSize: 12, color: 'var(--text-tertiary)' }}>
                      {new Date(backup.created_at).toLocaleString()} &middot;{' '}
                      {formatSize(backup.size_bytes)} &middot;{' '}
                      {t('settings:objects_count', { n: backup.object_count })}
                    </p>
                  </div>
                  <div style={{ display: 'flex', gap: 4 }}>
                    <Button
                      variant="secondary"
                      size="sm"
                      onClick={() => handleRestore(backup.id)}
                      loading={restoringId === backup.id}
                      title={t('settings:restore_title')}
                    >
                      <RotateCcw size={14} />
                    </Button>
                    <Button
                      variant="tertiary"
                      size="sm"
                      onClick={() => handleDelete(backup.id)}
                      title={t('settings:delete_title')}
                      style={{ color: 'var(--accent-danger, #ef4444)' }}
                    >
                      <Trash2 size={14} />
                    </Button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </Card>

        {/* Info */}
        <p style={{ fontSize: 12, color: 'var(--text-tertiary)', textAlign: 'center' }}>
          {t('settings:backups_stored_locally')}
          <br />
          {t('settings:export_hint')}
        </p>
      </div>
    </AppShell>
  );
}
