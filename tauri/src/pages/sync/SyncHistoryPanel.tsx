import { useTranslation } from 'react-i18next';
import { Card } from '@/components/ui/Card';
import { ChevronDown, ChevronUp } from 'lucide-react';
import { ICON_SIZE } from '@/lib/constants';
import type { SyncConflict, SyncResult } from '@/lib/ipc';

function formatHlc(hlc: SyncConflict['local_hlc']): string {
  // P001：node_id 后端统一 hex 编码为字符串（桌面/移动一致），直接截取前 8 位。
  return `${hlc.wall_time_ms}-${hlc.counter}-${hlc.node_id.slice(0, 8)}`;
}

interface SyncHistoryPanelProps {
  activityOpen: boolean;
  recentResults: SyncResult[];
  onToggleActivity: () => void;
}

/**
 * 同步历史面板：折叠式同步活动卡片（近期同步结果，含分表明细与冲突 HLC）。
 * 数据与回调经 SyncPage 从 useSyncPage 透传（P224-② 拆分）。
 */
export function SyncHistoryPanel({
  activityOpen,
  recentResults,
  onToggleActivity,
}: SyncHistoryPanelProps) {
  const { t } = useTranslation(['settings']);
  if (recentResults.length === 0) {
    return null;
  }
  return (
    <>
      {/* Sync activity */}
      <Card>
        <button
          type="button"
          onClick={onToggleActivity}
          className="interactive-color-accent"
          style={{
            width: '100%',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            background: 'none',
            border: 'none',
            padding: '4px 0',
            cursor: 'pointer',
          }}
        >
          <h3 style={{ fontSize: 'var(--text-sm)', fontWeight: 600 }}>
            {t('settings:sync_activity_title', { defaultValue: 'Sync Activity' })}
          </h3>
          {activityOpen ? <ChevronUp size={ICON_SIZE.lg} /> : <ChevronDown size={ICON_SIZE.lg} />}
        </button>

        {activityOpen && (
          <div
            style={{
              marginTop: 12,
              display: 'flex',
              flexDirection: 'column',
              gap: 12,
            }}
          >
            {recentResults.map((result, idx) => (
              <div
                key={idx}
                style={{
                  padding: 12,
                  borderRadius: 8,
                  background: 'var(--bg-toolbar)',
                  fontSize: 'var(--text-caption)',
                }}
              >
                <div style={{ fontWeight: 500, marginBottom: 6 }}>
                  {t('settings:sync_result_stats', {
                    examined: result.examined,
                    applied: result.applied,
                    skipped: result.skipped,
                    conflicts: result.conflictCount ?? result.conflicts.length,
                  })}
                  {/* B：入站结果携带发回对端条数（完整交换量） */}
                  {result.outboundRecords != null &&
                    ` · ${t('settings:sync_result_outbound', {
                      outbound: result.outboundRecords,
                      defaultValue: 'sent {{outbound}} back',
                    })}`}
                </div>
                {result.per_table.length > 0 && (
                  <div
                    style={{
                      display: 'flex',
                      flexWrap: 'wrap',
                      gap: 6,
                      marginBottom: result.conflicts.length > 0 ? 8 : 0,
                    }}
                  >
                    {result.per_table.map((tbl) => (
                      <span
                        key={tbl.table}
                        style={{
                          padding: '2px 8px',
                          borderRadius: 4,
                          background: 'var(--bg-elevated)',
                          color: 'var(--text-secondary)',
                        }}
                      >
                        {tbl.table}: {tbl.applied}+{tbl.skipped}/{tbl.examined}
                      </span>
                    ))}
                  </div>
                )}
                {result.conflicts.length > 0 && (
                  <div style={{ marginTop: 6 }}>
                    <div style={{ color: '#c0392b', marginBottom: 4 }}>
                      {t('settings:sync_conflicts', { defaultValue: 'Conflicts' })}:{' '}
                      {result.conflicts.length}
                    </div>
                    <ul
                      style={{
                        margin: 0,
                        paddingLeft: 16,
                        color: 'var(--text-secondary)',
                      }}
                    >
                      {result.conflicts.map((c, cidx) => (
                        <li key={cidx}>
                          {c.table}/{c.id} — {t('settings:sync_winner', { defaultValue: 'winner' })}
                          : {c.winner}
                          <div
                            style={{
                              fontFamily: 'monospace',
                              fontSize: 'var(--text-badge)',
                              color: 'var(--text-tertiary)',
                            }}
                          >
                            local: {formatHlc(c.local_hlc)} / remote: {formatHlc(c.remote_hlc)}
                          </div>
                        </li>
                      ))}
                    </ul>
                  </div>
                )}
              </div>
            ))}
          </div>
        )}
      </Card>
    </>
  );
}
