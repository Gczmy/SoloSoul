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

/** 字段值展示：字符串原样、其他标量转字符串、嵌套对象/数组紧凑 JSON（截断）。 */
function formatValue(value: unknown, maxLen = 220): string {
  if (value === null || value === undefined) {
    return '';
  }
  if (typeof value === 'string') {
    return value;
  }
  if (typeof value === 'number' || typeof value === 'boolean') {
    return String(value);
  }
  const json = JSON.stringify(value);
  return json.length > maxLen ? `${json.slice(0, maxLen)}…` : json;
}

/** 深度相等：JSON 序列化比较（冲突数据均为可序列化值）。 */
function valuesEqual(a: unknown, b: unknown): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}

/** 字段级 diff 行：合并本地/远程顶层键，逐字段比对。 */
function buildFieldRows(local: unknown, remote: unknown, remoteDeleted: boolean) {
  const l = local && typeof local === 'object' ? (local as Record<string, unknown>) : {};
  const r = remote && typeof remote === 'object' ? (remote as Record<string, unknown>) : {};
  const keys = Array.from(new Set([...Object.keys(l), ...Object.keys(r)]));
  return keys.map((key) => {
    const lv = key in l ? l[key] : undefined;
    const rv = key in r ? r[key] : undefined;
    const changed = remoteDeleted ? lv !== undefined : !valuesEqual(lv, rv);
    return { key, local: lv, remote: rv, changed };
  });
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

  const fieldRows =
    detail && selectedConflict
      ? buildFieldRows(detail.local_data, detail.remote_data, detail.remote_deleted)
      : [];

  return (
    <Dialog
      isOpen={isOpen}
      onClose={onClose}
      title={t('settings:sync_conflicts_title', { defaultValue: 'Sync Conflicts' })}
    >
      <div className={styles.container}>
        {conflicts.length === 0 ? (
          <div className={styles.empty}>
            {t('settings:sync_no_conflicts', { defaultValue: 'No unresolved conflicts.' })}
          </div>
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
                  <div className={styles.itemWinner}>
                    {t('settings:sync_conflict_winner', { defaultValue: 'Winner' })}:{' '}
                    {c.winner === 'local'
                      ? t('settings:sync_conflict_local', { defaultValue: 'Local' })
                      : t('settings:sync_conflict_remote', { defaultValue: 'Remote' })}
                  </div>
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
                  <div className={styles.winnerRow}>
                    <strong>{t('settings:sync_conflict_winner', { defaultValue: 'Winner' })}:</strong>{' '}
                    <span
                      className={`${styles.winnerBadge} ${
                        detail.winner === 'local' ? styles.winnerLocal : styles.winnerRemote
                      }`}
                    >
                      {detail.winner === 'local'
                        ? t('settings:sync_conflict_local', { defaultValue: 'Local' })
                        : t('settings:sync_conflict_remote', { defaultValue: 'Remote' })}
                    </span>
                    {detail.remote_deleted && (
                      <span className={styles.remoteDeletedBadge}>
                        {t('settings:sync_conflict_remote_deleted', {
                          defaultValue: 'Remote deleted',
                        })}
                      </span>
                    )}
                  </div>
                  <div className={styles.hlcRow}>
                    <span>
                      {t('settings:sync_conflict_local_hlc', { defaultValue: 'Local HLC' })}:{' '}
                      {formatHlc(detail.local_hlc)}
                    </span>
                    <span>
                      {t('settings:sync_conflict_remote_hlc', { defaultValue: 'Remote HLC' })}:{' '}
                      {formatHlc(detail.remote_hlc)}
                    </span>
                  </div>
                </div>

                {/* 字段级 diff */}
                <div className={styles.fieldDiff}>
                  <div className={styles.diffGrid}>
                    <div className={styles.diffTitle}>
                      {t('settings:sync_conflict_local', { defaultValue: 'Local' })}
                    </div>
                    <div className={styles.diffTitle}>
                      {t('settings:sync_conflict_remote', { defaultValue: 'Remote' })}
                    </div>
                  </div>
                  {fieldRows.length === 0 ? (
                    <div className={styles.fieldEmpty}>
                      {t('settings:sync_conflict_no_fields', {
                        defaultValue: 'No comparable fields.',
                      })}
                    </div>
                  ) : (
                    fieldRows.map((row) => (
                      <div
                        key={row.key}
                        className={`${styles.fieldRow} ${row.changed ? styles.fieldChanged : ''}`}
                      >
                        <div className={styles.fieldName} title={row.key}>
                          {row.key}
                          {row.changed && (
                            <span className={styles.changedBadge}>
                              {t('settings:sync_conflict_changed', { defaultValue: 'changed' })}
                            </span>
                          )}
                        </div>
                        <div className={styles.fieldValue}>
                          {row.local === undefined ? (
                            <span className={styles.fieldMissing}>
                              {t('settings:sync_conflict_missing', { defaultValue: '—' })}
                            </span>
                          ) : (
                            formatValue(row.local)
                          )}
                        </div>
                        <div className={styles.fieldValue}>
                          {detail.remote_deleted ? (
                            <span className={styles.fieldMissing}>
                              {t('settings:sync_conflict_remote_deleted', {
                                defaultValue: 'Remote deleted',
                              })}
                            </span>
                          ) : row.remote === undefined ? (
                            <span className={styles.fieldMissing}>
                              {t('settings:sync_conflict_missing', { defaultValue: '—' })}
                            </span>
                          ) : (
                            formatValue(row.remote)
                          )}
                        </div>
                      </div>
                    ))
                  )}
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
                    variant="secondary"
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
