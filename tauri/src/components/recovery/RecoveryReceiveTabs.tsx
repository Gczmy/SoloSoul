import { useTranslation } from 'react-i18next';
import { QrCode, Link2 } from 'lucide-react';
import type { LucideIcon } from 'lucide-react';
import type { TabMode } from '@/components/recovery/recoveryReceiveTypes';

interface RecoveryReceiveTabsProps {
  tab: TabMode;
  loading: boolean;
  onSwitch: (tab: TabMode) => void;
}

/** Tab 切换（仅收集连接信息阶段显示）。 */
export function RecoveryReceiveTabs({ tab, loading, onSwitch }: RecoveryReceiveTabsProps) {
  const { t } = useTranslation(['common']);
  const tabButton = (mode: TabMode, label: string, Icon: LucideIcon) => (
    <button
      type="button"
      onClick={() => onSwitch(mode)}
      style={{
        flex: 1,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        gap: 6,
        padding: '8px 12px',
        borderRadius: 8,
        border: 'none',
        background: tab === mode ? 'var(--bg-elevated)' : 'transparent',
        color: tab === mode ? 'var(--accent-primary)' : 'var(--text-tertiary)',
        cursor: loading ? 'not-allowed' : 'pointer',
        fontFamily: 'inherit',
        fontSize: 'var(--text-body-sm)',
        fontWeight: 500,
        transition: 'all 0.15s ease',
        opacity: loading ? 0.5 : 1,
      }}
    >
      <Icon size={16} />
      {label}
    </button>
  );

  return (
    <div
      style={{
        display: 'flex',
        gap: 4,
        marginBottom: 16,
        background: 'var(--bg-toolbar)',
        borderRadius: 10,
        padding: 3,
      }}
    >
      {tabButton('scan', t('common:recovery_scan_tab', { defaultValue: 'Scan QR' }), QrCode)}
      {tabButton('manual', t('common:recovery_manual_tab', { defaultValue: 'Manual' }), Link2)}
    </div>
  );
}
