import { useTranslation } from 'react-i18next';
import { Card } from '@/components/ui/Card';
import { SyncConflictDialog } from '@/components/sync/SyncConflictDialog';
import type { SyncConflictDetail, SyncConflictStrategy, SyncConflictSummary } from '@/lib/ipc';

interface ConflictPanelProps {
  conflicts: SyncConflictSummary[];
  selectedConflict: SyncConflictDetail | null;
  dialogOpen: boolean;
  isLoading: boolean;
  onOpenDialog: () => void;
  onCloseDialog: () => void;
  onResolve: (conflictId: string, strategy: SyncConflictStrategy) => void;
}

/**
 * 冲突面板：未解决冲突摘要卡片 + 冲突解决对话框。
 * 数据与回调经 SyncPage 从 useSyncPage 透传（P224-② 拆分）。
 */
export function ConflictPanel({
  conflicts,
  selectedConflict,
  dialogOpen,
  isLoading,
  onOpenDialog,
  onCloseDialog,
  onResolve,
}: ConflictPanelProps) {
  const { t } = useTranslation(['settings']);
  return (
    <>
      {/* Conflicts card */}
      {conflicts.length > 0 && (
        <Card>
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
            <div>
              <div style={{ fontSize: 'var(--text-card-title)', fontWeight: 600 }}>
                {t('settings:sync_conflicts_title', { defaultValue: 'Sync Conflicts' })}
              </div>
              <div style={{ fontSize: 'var(--text-caption)', color: 'var(--text-tertiary)' }}>
                {t('settings:sync_conflicts_desc', {
                  defaultValue: `${conflicts.length} unresolved conflict(s) need your attention.`,
                })}
              </div>
            </div>
            <button
              onClick={onOpenDialog}
              className="interactive-danger-soft"
              style={{
                padding: '8px 16px',
                borderRadius: 8,
                borderWidth: 1,
                borderStyle: 'solid',
                fontSize: 'var(--text-body-sm)',
                fontWeight: 500,
                cursor: 'pointer',
                fontFamily: 'inherit',
              }}
            >
              {t('settings:sync_review_conflicts', { defaultValue: 'Review' })}
            </button>
          </div>
        </Card>
      )}

      <SyncConflictDialog
        isOpen={dialogOpen}
        conflicts={conflicts}
        detail={selectedConflict}
        isLoading={isLoading}
        onClose={onCloseDialog}
        onResolve={onResolve}
      />
    </>
  );
}
