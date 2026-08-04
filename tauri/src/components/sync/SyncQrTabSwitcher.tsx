import { QrCode, LifeBuoy } from 'lucide-react';
import type { useTranslation } from 'react-i18next';

/** 二维码类型：同步配对 / 数据恢复。 */
export type QrMode = 'sync' | 'recovery';

type T = ReturnType<typeof useTranslation>['t'];

/** 同步 / 恢复二维码 Tab 切换器。 */
export function SyncQrTabSwitcher({
  t,
  isRecovery,
  onSelect,
}: {
  t: T;
  isRecovery: boolean;
  onSelect: (mode: QrMode) => void;
}) {
  return (
    <div
      style={{
        display: 'flex',
        gap: 4,
        padding: 4,
        borderRadius: 10,
        background: 'var(--bg-toolbar)',
        marginBottom: 16,
      }}
    >
      <button
        type="button"
        onClick={() => onSelect('sync')}
        style={{
          flex: 1,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          gap: 6,
          padding: '8px 0',
          borderRadius: 8,
          border: 'none',
          background: !isRecovery ? 'var(--bg-elevated)' : 'transparent',
          color: !isRecovery ? 'var(--accent-primary)' : 'var(--text-secondary)',
          fontSize: 'var(--text-body-sm)',
          fontWeight: 500,
          cursor: 'pointer',
          fontFamily: 'inherit',
          transition: 'all 0.15s ease',
        }}
      >
        <QrCode size={16} />
        {t('settings:sync_qr_tab_sync', { defaultValue: 'Sync QR' })}
      </button>
      <button
        type="button"
        onClick={() => onSelect('recovery')}
        style={{
          flex: 1,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          gap: 6,
          padding: '8px 0',
          borderRadius: 8,
          border: 'none',
          background: isRecovery ? 'var(--bg-elevated)' : 'transparent',
          color: isRecovery ? 'var(--accent-primary)' : 'var(--text-secondary)',
          fontSize: 'var(--text-body-sm)',
          fontWeight: 500,
          cursor: 'pointer',
          fontFamily: 'inherit',
          transition: 'all 0.15s ease',
        }}
      >
        <LifeBuoy size={16} />
        {t('settings:sync_qr_tab_recovery', { defaultValue: 'Recovery QR' })}
      </button>
    </div>
  );
}
