import { useTranslation } from 'react-i18next';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { DeleteButton } from '@/components/ui/DeleteButton';
import { ConfirmDialog } from '@/components/ui/ConfirmDialog';
import { RefreshCw, ShieldCheck, ShieldOff, Smartphone } from 'lucide-react';
import { resolveBackendErrorMessage } from '@/lib/backendError';
import { formatPeerName } from '@/lib/syncPeer';
import { ICON_SIZE } from '@/lib/constants';
import type { SyncResult } from '@/lib/ipc';
import type { DiscoveredDevice, SyncPeer } from '@/stores/syncStore';

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
  onManualAddrChange: (value: string) => void;
  onDiscover: () => void;
  onSyncWithDevice: (addr: string) => void;
  onTrustPeer: (peerId: string) => void;
  onOpenPairTarget: (peer: SyncPeer) => void;
  onForgetRequest: (peer: SyncPeer) => void;
  onForgetConfirm: () => void;
  onForgetCancel: () => void;
  onRefresh: () => void;
}

/**
 * 设备列表面板：手动同步 + 已发现设备 + 已知设备 + 忘记设备二次确认对话框。
 * 数据与回调经 SyncPage 从 useSyncPage 透传（P224-② 拆分）。
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
  onManualAddrChange,
  onDiscover,
  onSyncWithDevice,
  onTrustPeer,
  onOpenPairTarget,
  onForgetRequest,
  onForgetConfirm,
  onForgetCancel,
  onRefresh,
}: DeviceListPanelProps) {
  const { t } = useTranslation(['settings', 'common']);
  return (
    <>
      {/* Discovered devices */}
      <Card>
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
          <h3 style={{ fontSize: 'var(--text-sm)', fontWeight: 600 }}>
            {t('settings:sync_discovered_devices', { defaultValue: 'Discovered Devices' })}
          </h3>
          <button
            onClick={onDiscover}
            disabled={!syncEnabled || isDiscoveringDevices}
            style={{
              padding: '6px 12px',
              borderRadius: 8,
              border: '1px solid var(--border-subtle)',
              background: 'var(--bg-toolbar)',
              color: 'var(--text-primary)',
              fontSize: 'var(--text-body-sm)',
              fontWeight: 500,
              cursor: !syncEnabled || isDiscoveringDevices ? 'default' : 'pointer',
              opacity: !syncEnabled || isDiscoveringDevices ? 0.5 : 1,
              transition: 'all 0.15s ease',
              fontFamily: 'inherit',
            }}
          >
            {isDiscoveringDevices
              ? t('common:loading', { defaultValue: 'Loading...' })
              : t('settings:sync_discover', { defaultValue: 'Discover' })}
          </button>
        </div>
        <p
          style={{
            fontSize: 'var(--text-caption)',
            color: 'var(--text-tertiary)',
            marginTop: 8,
            marginBottom: 12,
          }}
        >
          {t('settings:sync_discovered_hint', {
            defaultValue: 'Nearby devices advertising SoloSoul sync will appear here.',
          })}
        </p>
        {discoveredDevices.length > 0 ? (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
            {discoveredDevices.map((device) => {
              const deviceAddr = device.addresses[0] || `${device.host}:${device.port}`;
              return (
                <div
                  key={deviceAddr}
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: 10,
                    padding: '10px 12px',
                    borderRadius: 8,
                    background: 'var(--bg-toolbar)',
                  }}
                >
                  <Smartphone size={ICON_SIZE.lg} style={{ color: 'var(--accent-primary)' }} />
                  <div style={{ flex: 1, minWidth: 0 }}>
                    <div style={{ fontSize: 'var(--text-body-sm)', fontWeight: 500 }}>
                      {device.name ||
                        t('settings:sync_unknown_device', { defaultValue: 'Unknown device' })}
                    </div>
                    <div style={{ fontSize: 'var(--text-badge)', color: 'var(--text-tertiary)' }}>
                      {deviceAddr}
                    </div>
                  </div>
                  <button
                    onClick={() => onSyncWithDevice(deviceAddr)}
                    disabled={isLoading}
                    style={{
                      padding: '6px 12px',
                      borderRadius: 8,
                      border: '1px solid var(--border-subtle)',
                      background: 'var(--bg-toolbar)',
                      color: 'var(--text-primary)',
                      fontSize: 'var(--text-body-sm)',
                      fontWeight: 500,
                      cursor: isLoading ? 'default' : 'pointer',
                      opacity: isLoading ? 0.5 : 1,
                      transition: 'all 0.15s ease',
                      fontFamily: 'inherit',
                      whiteSpace: 'nowrap',
                    }}
                  >
                    {t('settings:sync_manual_sync', { defaultValue: 'Sync' })}
                  </button>
                </div>
              );
            })}
          </div>
        ) : (
          <>
            <p style={{ fontSize: 'var(--text-caption)', color: 'var(--text-tertiary)' }}>
              {t('settings:sync_no_devices_found', {
                defaultValue: 'No devices found. Click Discover to scan.',
              })}
            </p>
            {syncEnabled && (
              <p
                style={{
                  fontSize: 'var(--text-caption)',
                  color: 'var(--text-tertiary)',
                  marginTop: 8,
                }}
              >
                {t('settings:sync_manual_fallback_hint', {
                  defaultValue:
                    "If automatic discovery fails, ensure both devices are on the same Wi-Fi and enter the other device's IP address with port below.",
                })}
              </p>
            )}
          </>
        )}
      </Card>

      {/* Manual sync */}
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
            {t('settings:sync_result', { defaultValue: 'Result' })}: {lastResult.summary}
          </p>
        )}
        {error && (
          <p style={{ fontSize: 'var(--text-caption)', color: '#e74c3c', marginTop: 8 }}>
            {resolveBackendErrorMessage(error)}
          </p>
        )}
      </Card>

      {/* Known peers */}
      <Card>
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
          <h3 style={{ fontSize: 'var(--text-sm)', fontWeight: 600 }}>
            {t('settings:sync_known_devices_title', { defaultValue: 'Known Devices' })}
          </h3>
          <Button size="sm" variant="tertiary" onClick={onRefresh} loading={isLoading}>
            <RefreshCw size={ICON_SIZE.sm} />
          </Button>
        </div>
        <p
          style={{
            fontSize: 'var(--text-caption)',
            color: 'var(--text-tertiary)',
            marginTop: 8,
            marginBottom: 12,
          }}
        >
          {t('settings:sync_known_devices_hint', {
            defaultValue:
              'Devices you have discovered or connected to before; only trusted devices can sync.',
          })}
        </p>

        {connectedPeers.length > 0 ? (
          <div style={{ marginTop: 12, display: 'flex', flexDirection: 'column', gap: 8 }}>
            {connectedPeers.map((peer) => {
              const displayName = formatPeerName(peer);
              return (
                <div
                  key={peer.id}
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: 10,
                    padding: '10px 12px',
                    borderRadius: 8,
                    background: 'var(--bg-toolbar)',
                  }}
                >
                  <Smartphone size={ICON_SIZE.lg} style={{ color: 'var(--accent-primary)' }} />
                  <div style={{ flex: 1, minWidth: 0 }}>
                    <div
                      style={{
                        fontSize: 'var(--text-body-sm)',
                        fontWeight: 500,
                        display: 'flex',
                        alignItems: 'center',
                        gap: 8,
                      }}
                    >
                      <span
                        style={{
                          overflow: 'hidden',
                          textOverflow: 'ellipsis',
                          whiteSpace: 'nowrap',
                        }}
                      >
                        {displayName}
                      </span>
                      <span
                        style={{
                          fontSize: 'var(--text-badge)',
                          padding: '2px 8px',
                          borderRadius: 999,
                          flexShrink: 0,
                          background: peer.trusted
                            ? 'rgba(39,174,96,0.12)'
                            : 'rgba(128,128,128,0.1)',
                          color: peer.trusted ? '#27ae60' : 'var(--text-tertiary)',
                          whiteSpace: 'nowrap',
                        }}
                      >
                        {peer.trusted
                          ? t('settings:sync_trusted_badge', { defaultValue: 'Trusted' })
                          : t('settings:sync_untrusted_badge', { defaultValue: 'Not trusted' })}
                      </span>
                    </div>
                    <div style={{ fontSize: 'var(--text-badge)', color: 'var(--text-tertiary)' }}>
                      {peer.addr || 'offline'} · {peer.lastSeen || 'never'}
                    </div>
                    <div
                      style={{
                        fontSize: 'var(--text-badge)',
                        color: 'var(--text-tertiary)',
                        fontFamily: 'monospace',
                        wordBreak: 'break-all',
                      }}
                    >
                      {peer.fingerprint}
                    </div>
                  </div>
                  <div style={{ display: 'flex', gap: 6 }}>
                    {peer.trusted ? (
                      <Button
                        size="sm"
                        variant="secondary"
                        onClick={() => onTrustPeer(peer.id)}
                        title={t('settings:sync_revoke_tooltip', {
                          defaultValue: 'Revoke trust: keep the record, reject its syncs',
                        })}
                      >
                        <ShieldOff size={ICON_SIZE.sm} />
                      </Button>
                    ) : (
                      <Button
                        size="sm"
                        variant="secondary"
                        onClick={() => onOpenPairTarget(peer)}
                        title={t('settings:sync_pair_tooltip', {
                          defaultValue: 'Pair this device',
                        })}
                      >
                        <ShieldCheck size={ICON_SIZE.sm} />
                      </Button>
                    )}
                    <DeleteButton
                      onClick={() => onForgetRequest(peer)}
                      title={t('settings:sync_forget_tooltip', {
                        defaultValue: 'Forget: delete the record, you will need to re-pair',
                      })}
                      iconOnly
                    />
                  </div>
                </div>
              );
            })}
          </div>
        ) : (
          <p
            style={{
              fontSize: 'var(--text-caption)',
              color: 'var(--text-tertiary)',
              marginTop: 8,
            }}
          >
            {t('settings:sync_no_devices', {
              defaultValue:
                'No devices known yet. Enable sync and sync with another device to add it.',
            })}
          </p>
        )}
      </Card>
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
    </>
  );
}
