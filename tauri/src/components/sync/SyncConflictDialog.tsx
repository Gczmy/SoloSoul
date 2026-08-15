import { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { Dialog } from '@/components/ui/Dialog';
import { Button } from '@/components/ui/Button';
import { SensitivityBadge } from '@/components/ui/SensitivityBadge';
import { ToggleSwitch } from '@/components/ui/ToggleSwitch';
import { useSyncStore } from '@/stores/syncStore';
import type { LucideIcon } from 'lucide-react';
import type { SyncConflictSummary, SyncConflictDetail, SyncConflictStrategy } from '@/lib/ipc';
import {
  conflictFieldLabel,
  conflictTableLabel,
  formatConflictValue,
  truncateConflictValue,
  buildDiffEntries,
  isSensitivityLevel,
  resolveConflictIcon,
  shouldOmitField,
} from '@/lib/conflictFieldMeta';
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

/** 深度相等：JSON 序列化比较（冲突数据均为可序列化值）。 */
function valuesEqual(a: unknown, b: unknown): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}

/** 图标字段的图标图案渲染（lucide 组件按小尺寸/细描边适配文本行）。 */
function DiffIcon({ icon }: { icon: LucideIcon }) {
  const Icon = icon;
  return <Icon size={14} strokeWidth={1.8} />;
}

/** 字段级 diff 行：合并本地/远程顶层键，逐字段比对；省略用户无法感知/修改的元数据行。
 * `onlyDifferences` 为 true 时仅保留有差异的行（「只看差异」模式）。 */
function buildFieldRows(
  local: unknown,
  remote: unknown,
  remoteDeleted: boolean,
  onlyDifferences: boolean,
) {
  const l = local && typeof local === 'object' ? (local as Record<string, unknown>) : {};
  const r = remote && typeof remote === 'object' ? (remote as Record<string, unknown>) : {};
  const keys = Array.from(new Set([...Object.keys(l), ...Object.keys(r)]));
  return keys
    .map((key) => {
      const lv = key in l ? l[key] : undefined;
      const rv = key in r ? r[key] : undefined;
      const changed = remoteDeleted ? lv !== undefined : !valuesEqual(lv, rv);
      return { key, local: lv, remote: rv, changed };
    })
    .filter((row) => !shouldOmitField(row.key, row.local, row.remote))
    .filter((row) => !onlyDifferences || row.changed);
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
  const [onlyDifferences, setOnlyDifferences] = useState(false);

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
  const selectedIndex = selectedConflict ? conflicts.findIndex((c) => c.id === selectedId) : -1;

  const fieldRows =
    detail && selectedConflict
      ? buildFieldRows(
          detail.local_data,
          detail.remote_data,
          detail.remote_deleted,
          onlyDifferences,
        )
      : [];

  // P027: 字段行抽为子组件（降低 JSX 嵌套深度）；diff 行渲染与计数逻辑保持一致
  const renderFieldRow = (row: { key: string; local: unknown; remote: unknown; changed: boolean }) => {
    if (!detail) return null;
    return (
      <ConflictFieldRow
        key={row.key}
        row={row}
        detail={detail}
        onlyDifferences={onlyDifferences}
        t={t}
      />
    );
  };
  const diffCount = fieldRows.reduce(
    (n, row) =>
      n +
      (row.changed
        ? detail?.remote_deleted
          ? 1 // remote 整行删除：整行为一处差异
          : // 对象字段统计展开后的差异叶子数；标量行 buildDiffEntries 为 null，按 1 处计
            (buildDiffEntries(row.key, row.local, row.remote, t)?.filter((e) => e.changed)
              .length ?? 1)
        : 0),
    0,
  );

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
            <div className={styles.listHeader}>
              {t('settings:sync_conflict_list_count', {
                defaultValue: '{{count}} conflicts',
                count: conflicts.length,
              })}
            </div>
            <div className={styles.list}>
              {conflicts.map((c) => (
                <button
                  key={c.id}
                  type="button"
                  className={`${styles.item} ${selectedId === c.id ? styles.selected : ''}`}
                  onClick={() => handleSelect(c.id)}
                >                    <div className={styles.itemTable}>{conflictTableLabel(c.table, t)}</div>
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
                  {/* 冲突位置指示 + 上一条/下一条导航：明确告知用户共有 N 条冲突、当前查看第几条 */}
                  <div className={styles.conflictNav}>
                    <Button
                      variant="tertiary"
                      onClick={() => selectedIndex > 0 && handleSelect(conflicts[selectedIndex - 1].id)}
                      disabled={selectedIndex <= 0}
                    >
                      {t('settings:sync_conflict_prev', { defaultValue: '‹ Previous' })}
                    </Button>
                    <span className={styles.conflictNavPosition}>
                      {t('settings:sync_conflict_nav_position', {
                        defaultValue: 'Conflict {{index}} / {{total}}',
                        index: selectedIndex + 1,
                        total: conflicts.length,
                      })}
                    </span>
                    <Button
                      variant="tertiary"
                      onClick={() =>
                        selectedIndex < conflicts.length - 1 &&
                        handleSelect(conflicts[selectedIndex + 1].id)
                      }
                      disabled={selectedIndex >= conflicts.length - 1}
                    >
                      {t('settings:sync_conflict_next', { defaultValue: 'Next ›' })}
                    </Button>
                  </div>
                  <div>
                    <strong>{t('settings:sync_conflict_record', { defaultValue: 'Record' })}:</strong>{' '}
                    {detail.record_id}
                  </div>
                  <div>
                    <strong>{t('settings:sync_conflict_table', { defaultValue: 'Table' })}:</strong>{' '}
                    {conflictTableLabel(detail.table, t)}
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
                  <div className={styles.diffToolbar}>
                    <span className={styles.onlyDiffToggle}>
                      <ToggleSwitch
                        checked={onlyDifferences}
                        onChange={() => setOnlyDifferences(!onlyDifferences)}
                      />
                      <span>
                        {t('settings:sync_conflict_only_diff', {
                          defaultValue: 'Only show differences',
                        })}
                      </span>
                    </span>
                    {onlyDifferences && (
                      <span className={styles.onlyDiffCount}>
                        {t('settings:sync_conflict_only_diff_count', {
                          defaultValue: '{{count}} differences',
                          count: diffCount,
                        })}
                      </span>
                    )}
                  </div>
                  <div className={styles.diffGrid}>
                    <div className={styles.diffTitle}>
                      {t('settings:sync_conflict_item', { defaultValue: 'Item' })}
                    </div>
                    <div className={styles.diffTitle}>
                      {t('settings:sync_conflict_local', { defaultValue: 'Local' })}
                    </div>
                    <div className={styles.diffTitle}>
                      {t('settings:sync_conflict_remote', { defaultValue: 'Remote' })}
                    </div>
                  </div>
                  {fieldRows.length === 0 ? (
                    <div className={styles.fieldEmpty}>
                      {onlyDifferences
                        ? t('settings:sync_conflict_no_differences', {
                            defaultValue: 'No differences.',
                          })
                        : t('settings:sync_conflict_no_fields', {
                            defaultValue: 'No comparable fields.',
                          })}
                    </div>
                  ) : (
                    fieldRows.map(renderFieldRow)
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

/**
 * P027: 字段级冲突行（从 SyncConflictDialog 抽出，降低主组件 JSX 嵌套深度）。
 * 渲染字段名 + 差异徽章 + 本地值/远程值两列（对象/数组字段展开为叶子级 diff 条目，
 * 标量字段沿用整值渲染；敏感度/图标/截断规则与既有实现一致）。
 */
function ConflictFieldRow({
  row,
  detail,
  onlyDifferences,
  t,
}: {
  row: { key: string; local: unknown; remote: unknown; changed: boolean };
  detail: SyncConflictDetail;
  onlyDifferences: boolean;
  t: (key: string, opts?: Record<string, unknown>) => string;
}) {
  // 对象/数组字段展开为叶子级条目，逐行高亮差异；标量字段沿用整值渲染
  const diffEntries = detail.remote_deleted
    ? null
    : buildDiffEntries(row.key, row.local, row.remote, t, onlyDifferences);
  const hasEntries = diffEntries !== null && diffEntries.length > 0;
  const missingLabel = t('settings:sync_conflict_missing', { defaultValue: '—' });
  const localIcon = resolveConflictIcon(row.key, row.local);
  const remoteIcon = resolveConflictIcon(row.key, row.remote);

  return (
    <div className={`${styles.fieldRow} ${row.changed ? styles.fieldChanged : ''}`}>
      <div className={styles.fieldName} title={row.key}>
        {conflictFieldLabel(row.key, t)}
        {row.changed && (
          <span className={styles.changedBadge}>
            {t('settings:sync_conflict_changed', { defaultValue: 'changed' })}
          </span>
        )}
      </div>
      <div className={styles.fieldValue}>
        {row.local === undefined ? (
          <span className={styles.fieldMissing}>{missingLabel}</span>
        ) : hasEntries ? (
          <div className={styles.diffEntryList}>
            {diffEntries.map((e) => (
              <div
                key={e.path}
                className={`${styles.diffEntry} ${e.changed ? styles.diffEntryChanged : ''}`}
              >
                <span className={styles.diffEntryLabel}>
                  {e.label || conflictFieldLabel(row.key, t)}:
                </span>
                {e.localText === null ? (
                  <span className={styles.fieldMissing}>{missingLabel}</span>
                ) : e.localLevel ? (
                  <SensitivityBadge level={e.localLevel} />
                ) : e.localIcon ? (
                  <span className={styles.diffValueWithIcon}>
                    <DiffIcon icon={e.localIcon} />
                    {truncateConflictValue(e.localText)}
                  </span>
                ) : (
                  truncateConflictValue(e.localText)
                )}
              </div>
            ))}
          </div>
        ) : isSensitivityLevel(row.local) ? (
          <SensitivityBadge level={row.local} />
        ) : localIcon ? (
          <span className={styles.diffValueWithIcon}>
            <DiffIcon icon={localIcon} />
            {truncateConflictValue(formatConflictValue(row.key, row.local, t))}
          </span>
        ) : (
          truncateConflictValue(formatConflictValue(row.key, row.local, t))
        )}
      </div>
      <div className={styles.fieldValue}>
        {detail.remote_deleted ? (
          <span className={styles.fieldMissing}>
            {t('settings:sync_conflict_remote_deleted', { defaultValue: 'Remote deleted' })}
          </span>
        ) : row.remote === undefined ? (
          <span className={styles.fieldMissing}>{missingLabel}</span>
        ) : hasEntries ? (
          <div className={styles.diffEntryList}>
            {diffEntries.map((e) => (
              <div
                key={e.path}
                className={`${styles.diffEntry} ${e.changed ? styles.diffEntryChanged : ''}`}
              >
                <span className={styles.diffEntryLabel}>
                  {e.label || conflictFieldLabel(row.key, t)}:
                </span>
                {e.remoteText === null ? (
                  <span className={styles.fieldMissing}>{missingLabel}</span>
                ) : e.remoteLevel ? (
                  <SensitivityBadge level={e.remoteLevel} />
                ) : e.remoteIcon ? (
                  <span className={styles.diffValueWithIcon}>
                    <DiffIcon icon={e.remoteIcon} />
                    {truncateConflictValue(e.remoteText)}
                  </span>
                ) : (
                  truncateConflictValue(e.remoteText)
                )}
              </div>
            ))}
          </div>
        ) : isSensitivityLevel(row.remote) ? (
          <SensitivityBadge level={row.remote} />
        ) : remoteIcon ? (
          <span className={styles.diffValueWithIcon}>
            <DiffIcon icon={remoteIcon} />
            {truncateConflictValue(formatConflictValue(row.key, row.remote, t))}
          </span>
        ) : (
          truncateConflictValue(formatConflictValue(row.key, row.remote, t))
        )}
      </div>
    </div>
  );
}
