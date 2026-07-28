import { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { Dialog } from '@/components/ui/Dialog';
import { Button } from '@/components/ui/Button';
import { useSyncStore } from '@/stores/syncStore';
import type { SyncConflictSummary, SyncConflictDetail, SyncConflictStrategy } from '@/lib/ipc';
import styles from './SyncConflictDialog.module.css';

interface SyncConflictDialogProps {
  isOpen: boolean;
  conflicts: SyncConflictSummary[];
  detail: SyncConflictDetail | null;
  isLoading: boolean;
  onClose: () => void;
  onResolve: (conflictId: string, strategy: SyncConflictStrategy) => void;
}

function formatHlc(hlc: SyncConflictSummary['local_hlc']) {
  return `${hlc.wall_time_ms}/${hlc.counter}/${hlc.node_id.slice(0, 8)}`;
}

function formatJson(value: unknown) {
  return JSON.stringify(value, null, 2);
}

export function SyncConflictDialog({
  isOpen,
  conflicts,
  detail,
  isLoading,
  onClose,
  onResolve,
}: SyncConflictDialogProps) {
  const { t } = useTranslation(['settings', 'common']);
  const [selectedId, setSelectedId] = useState<string | null>(null);

  useEffect(() => {
    if (isOpen && conflicts.length > 0 && !selectedId) {
      setSelectedId(conflicts[0].id);
      void useSyncStore.getState().loadConflictDetail(conflicts[0].id);
    }
  }, [isOpen, conflicts, selectedId]);

  useEffect(() => {
    if (selectedId) {
      void useSyncStore.getState().loadConflictDetail(selectedId);
    }
  }, [selectedId]);

  const handleSelect = (id: string) => {
    setSelectedId(id);
  };

  const selectedConflict = conflicts.find((c) => c.id === selectedId);

  return (
    <Dialog isOpen={isOpen} onClose={onClose} title={t('settings:sync_conflicts_title', { defaultValue: 'Sync Conflicts' })}>
      <div className={styles.container}>
        {conflicts.length === 0 ? (
          <div className={styles.empty}>{t('settings:sync_no_conflicts', { defaultValue: 'No unresolved conflicts.' })}</div>
        ) : (
          <>
            <div className={styles.list}>
              {conflicts.map((c) => (
                <button
                  key={c.id}
                  type="button"
                  className={`${styles.item} ${selectedId === c.id ? styles.selected : ''}`}
                  onClick={() => handleSelect(c.id)}
                >
                  <div className={styles.itemTable}>{c.table}</div>
                  <div className={styles.itemRecord}>{c.record_id}</div>
                  <div className={styles.itemWinner}>{c.winner}</div>
                </button>
              ))}
            </div>
            {detail && selectedConflict && (
              <div className={styles.detail}>
                <div className={styles.detailHeader}>
                  <div>
                    <strong>{t('settings:sync_conflict_record', { defaultValue: 'Record' })}:</strong>{' '}
                    {detail.record_id}
                  </div>
                  <div>
                    <strong>{t('settings:sync_conflict_table', { defaultValue: 'Table' })}:</strong>{' '}
                    {detail.table}
                  </div>
                  <div>
                    <strong>{t('settings:sync_conflict_winner', { defaultValue: 'Winner' })}:</strong>{' '}
                    {detail.winner}
                  </div>
                  <div className={styles.hlcRow}>
                    <span>{t('settings:sync_conflict_local_hlc', { defaultValue: 'Local HLC' })}: {formatHlc(detail.local_hlc)}</span>
                    <span>{t('settings:sync_conflict_remote_hlc', { defaultValue: 'Remote HLC' })}: {formatHlc(detail.remote_hlc)}</span>
                  </div>
                </div>
                <div className={styles.diffGrid}>
                  <div className={styles.diffPane}>
                    <div className={styles.diffTitle}>{t('settings:sync_conflict_local', { defaultValue: 'Local' })}</div>
                    <pre className={styles.diffCode}>{formatJson(detail.local_data)}</pre>
                  </div>
                  <div className={styles.diffPane}>
                    <div className={styles.diffTitle}>{t('settings:sync_conflict_remote', { defaultValue: 'Remote' })}</div>
                    <pre className={styles.diffCode}>{formatJson(detail.remote_data)}</pre>
                  </div>
                </div>
                <div className={styles.actions}>
                  <Button
                    variant="secondary"
                    onClick={() => onResolve(detail.id, 'keep_local')}
                    disabled={isLoading}
                  >
                    {t('settings:sync_conflict_keep_local', { defaultValue: 'Keep Local' })}
                  </Button>
                  <Button
                    variant="primary"
                    onClick={() => onResolve(detail.id, 'keep_remote')}
                    disabled={isLoading}
                  >
                    {t('settings:sync_conflict_keep_remote', { defaultValue: 'Keep Remote' })}
                  </Button>
                  <Button
                    variant="tertiary"
                    onClick={() => onResolve(detail.id, 'dismiss')}
                    disabled={isLoading}
                  >
                    {t('settings:sync_conflict_dismiss', { defaultValue: 'Dismiss' })}
                  </Button>
                </div>
              </div>
            )}
          </>
        )}
      </div>
    </Dialog>
  );
}
