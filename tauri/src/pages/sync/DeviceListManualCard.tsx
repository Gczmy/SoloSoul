import type { TFunction } from 'i18next';
import { Card } from '@/components/ui/Card';
import { Input } from '@/components/ui/Input';
import { resolveBackendErrorMessage } from '@/lib/backendError';
import type { SyncResult } from '@/lib/ipc';

/**
 * DeviceListPanel 的「手动同步」卡片（P046 拆分：展示子组件）。
 */
export function DeviceListManualCard({
  manualAddr,
  lastResult,
  error,
  isLoading,
  onManualAddrChange,
  onSyncWithDevice,
  t,
}: {
  manualAddr: string;
  lastResult: SyncResult | null;
  error: string | null;
  isLoading: boolean;
  onManualAddrChange: (value: string) => void;
  onSyncWithDevice: (addr: string) => void;
  t: TFunction;
}) {
  return (
    <Card>
      <h3 style={{ fontSize: 'var(--text-sm)', fontWeight: 600, marginBottom: 12 }}>
        {t('settings:sync_with_device', { defaultValue: 'Sync with Device' })}
      </h3>
      <p
        style={{
          fontSize: 'var(--text-caption)',
          color: 'var(--text-tertiary)',
          marginBottom: 12,
        }}
      >
        {t('settings:sync_device_input_hint', {
          defaultValue: 'Enter a discovered device ID or a host:port address.',
        })}
      </p>
      <div style={{ display: 'flex', gap: 8 }}>
        <Input
          placeholder="host:port"
          value={manualAddr}
          onChange={(e) => onManualAddrChange(e.target.value)}
          style={{ flex: 1 }}
        />
        <button
          onClick={() => onSyncWithDevice(manualAddr)}
          disabled={!manualAddr.trim() || isLoading}
          className="interactive-toolbar"
          style={{
            padding: '8px 16px',
            borderRadius: 8,
            borderWidth: 1,
            borderStyle: 'solid',
            fontSize: 'var(--text-body-sm)',
            fontWeight: 500,
            cursor: !manualAddr.trim() || isLoading ? 'default' : 'pointer',
            opacity: !manualAddr.trim() || isLoading ? 0.5 : 1,
            fontFamily: 'inherit',
            whiteSpace: 'nowrap',
          }}
        >
          {isLoading
            ? t('common:loading', { defaultValue: 'Loading...' })
            : t('settings:sync_manual_sync', { defaultValue: 'Sync' })}
        </button>
      </div>
      {lastResult && (
        <p
          style={{
            fontSize: 'var(--text-caption)',
            color: 'var(--text-secondary)',
            marginTop: 8,
          }}
        >
          {t('settings:sync_result', { defaultValue: 'Result' })}:{' '}
          {t('settings:sync_result_stats', {
            examined: lastResult.examined,
            applied: lastResult.applied,
            skipped: lastResult.skipped,
            conflicts: lastResult.conflictCount ?? lastResult.conflicts.length,
          })}
          {/* B：入站结果携带发回对端条数（完整交换量），发起方结果无此字段不显示 */}
          {lastResult.outboundRecords != null &&
            ` · ${t('settings:sync_result_outbound', {
              outbound: lastResult.outboundRecords,
              defaultValue: 'sent {{outbound}} back',
            })}`}
        </p>
      )}
      {error && (
        <p style={{ fontSize: 'var(--text-caption)', color: '#e74c3c', marginTop: 8 }}>
          {resolveBackendErrorMessage(error)}
        </p>
      )}
    </Card>
  );
}
