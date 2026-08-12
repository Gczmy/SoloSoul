import { useTranslation } from 'react-i18next';
import { ConfirmDialog } from '@/components/ui/ConfirmDialog';
import { formatPeerName } from '@/lib/syncPeer';
import type { SyncResult } from '@/lib/ipc';
import type { DiscoveredDevice, SyncPeer } from '@/stores/syncStore';

import { DeviceListDiscoveredCard } from './DeviceListDiscoveredCard';
import { DeviceListManualCard } from './DeviceListManualCard';
import { DeviceListKnownCard } from './DeviceListKnownCard';
import { DeviceDetailDialog } from './DeviceDetailDialog';

interface DeviceListPanelProps {
  syncEnabled: boolean;
  isLoading: boolean;
  isDiscoveringDevices: boolean;
  discoveredDevices: DiscoveredDevice[];
  connectedPeers: SyncPeer[];
  manualAddr: string;
  lastResult: SyncResult | null;
  error: string | null;
  forgetTarget: SyncPeer | null;
  /** 详情弹窗当前展示的设备（null = 关闭）。 */
  detailPeer: SyncPeer | null;
  /** 详情弹窗当前展示的已发现设备（未匹配已知设备时非 null，与 detailPeer 互斥）。 */
  detailDiscovered?: DiscoveredDevice | null;
  onManualAddrChange: (value: string) => void;
  onDiscover: () => void;
  onSyncWithDevice: (addr: string) => void;
  onTrustPeer: (peerId: string) => void;
  onOpenPairTarget: (peer: SyncPeer) => void;
  onForgetRequest: (peer: SyncPeer) => void;
  onForgetConfirm: () => void;
  onForgetCancel: () => void;
  onRefresh: () => void;
  /** 点击已发现设备卡片打开详情弹窗（若匹配已知设备则复用已知设备详情）。 */
  onOpenDiscoveredDetail: (device: DiscoveredDevice) => void;
  /** 点击卡片主体打开已知设备详情弹窗。 */
  onOpenDetail: (peer: SyncPeer) => void;
  onCloseDetail: () => void;
  /** 对未匹配已知设备的发现设备发起立即同步。 */
  onSyncDiscovered?: (addr: string) => void;
}

/**
 * 设备列表面板 — P046 拆分后为纯组合层：
 * 已发现设备（DeviceListDiscoveredCard）、手动同步（DeviceListManualCard）、
 * 已知设备（DeviceListKnownCard）为独立展示子组件；
 * 本组件保留忘记设备确认对话框与详情弹窗装配。
 */
export function DeviceListPanel({
  syncEnabled,
  isLoading,
  isDiscoveringDevices,
  discoveredDevices,
  connectedPeers,
  manualAddr,
  lastResult,
  error,
  forgetTarget,
  detailPeer,
  detailDiscovered = null,
  onManualAddrChange,
  onDiscover,
  onSyncWithDevice,
  onTrustPeer,
  onOpenPairTarget,
  onForgetRequest,
  onForgetConfirm,
  onForgetCancel,
  onRefresh,
  onOpenDiscoveredDetail,
  onOpenDetail,
  onCloseDetail,
  onSyncDiscovered,
}: DeviceListPanelProps) {
  const { t } = useTranslation(['settings', 'common']);
  return (
    <>
      {/* Discovered devices（P046 拆分：DeviceListDiscoveredCard） */}
      <DeviceListDiscoveredCard
        syncEnabled={syncEnabled}
        isLoading={isLoading}
        isDiscoveringDevices={isDiscoveringDevices}
        discoveredDevices={discoveredDevices}
        onDiscover={onDiscover}
        onSyncWithDevice={onSyncWithDevice}
        onOpenDiscoveredDetail={onOpenDiscoveredDetail}
        t={t}
      />

      {/* Manual sync（P046 拆分：DeviceListManualCard） */}
      <DeviceListManualCard
        manualAddr={manualAddr}
        lastResult={lastResult}
        error={error}
        isLoading={isLoading}
        onManualAddrChange={onManualAddrChange}
        onSyncWithDevice={onSyncWithDevice}
        t={t}
      />

      {/* Known peers（P046 拆分：DeviceListKnownCard） */}
      <DeviceListKnownCard
        connectedPeers={connectedPeers}
        isLoading={isLoading}
        onRefresh={onRefresh}
        onTrustPeer={onTrustPeer}
        onOpenPairTarget={onOpenPairTarget}
        onForgetRequest={onForgetRequest}
        onOpenDetail={onOpenDetail}
        t={t}
      />
      <ConfirmDialog
        isOpen={!!forgetTarget}
        title={t('settings:sync_forget_confirm_title', { defaultValue: 'Forget device?' })}
        message={t('settings:sync_forget_confirm_desc', {
          defaultValue:
            'The connection record will be deleted. You will need to pair again to sync.',
        })}
        confirmLabel={t('settings:sync_forget_confirm_ok', { defaultValue: 'Forget' })}
        onConfirm={onForgetConfirm}
        onCancel={onForgetCancel}
      >
        {forgetTarget && (
          <div
            style={{
              marginBottom: 12,
              padding: 12,
              borderRadius: 8,
              background: 'var(--bg-toolbar)',
              display: 'flex',
              flexDirection: 'column',
              gap: 6,
              fontSize: 'var(--text-caption)',
            }}
          >
            <div>
              <strong>
                {t('settings:sync_forget_confirm_device', { defaultValue: 'Device' })}:
              </strong>{' '}
              {formatPeerName(forgetTarget)}
            </div>
            <div>
              <strong>
                {t('settings:sync_forget_confirm_addr', { defaultValue: 'Address' })}:
              </strong>{' '}
              {forgetTarget.addr || 'offline'}
            </div>
            <div>
              <strong>
                {t('settings:sync_forget_confirm_fp', { defaultValue: 'Fingerprint' })}:
              </strong>{' '}
              <span style={{ fontFamily: 'monospace', wordBreak: 'break-all' }}>
                {forgetTarget.fingerprint || '-'}
              </span>
            </div>
            <div>
              <strong>
                {t('settings:sync_forget_confirm_trust', { defaultValue: 'Trust status' })}:
              </strong>{' '}
              {forgetTarget.trusted
                ? t('settings:sync_trusted_badge', { defaultValue: 'Trusted' })
                : t('settings:sync_untrusted_badge', { defaultValue: 'Not trusted' })}
            </div>
          </div>
        )}
      </ConfirmDialog>
      <DeviceDetailDialog
        peer={detailPeer}
        discovered={detailDiscovered}
        onClose={onCloseDetail}
        isLoading={isLoading}
        onToggleTrust={(peer) => {
          if (peer.trusted) {
            onTrustPeer(peer.id);
          } else {
            onOpenPairTarget(peer);
          }
        }}
        onForgetRequest={(peer) => {
          onForgetRequest(peer);
        }}
        onSyncDiscovered={onSyncDiscovered}
      />
    </>
  );
}
