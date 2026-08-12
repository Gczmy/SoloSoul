import type { TFunction } from 'i18next';
import { Card } from '@/components/ui/Card';
import { formatDiscoveredName } from '@/lib/syncPeer';
import { DeviceCardShell } from './DeviceCard';
import type { DiscoveredDevice } from '@/stores/syncStore';

/**
 * DeviceListPanel 的「已发现设备」卡片（P046 拆分：展示子组件）。
 */
export function DeviceListDiscoveredCard({
  syncEnabled,
  isLoading,
  isDiscoveringDevices,
  discoveredDevices,
  onDiscover,
  onSyncWithDevice,
  onOpenDiscoveredDetail,
  t,
}: {
  syncEnabled: boolean;
  isLoading: boolean;
  isDiscoveringDevices: boolean;
  discoveredDevices: DiscoveredDevice[];
  onDiscover: () => void;
  onSyncWithDevice: (addr: string) => void;
  onOpenDiscoveredDetail: (device: DiscoveredDevice) => void;
  t: TFunction;
}) {
  return (
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
            const deviceName = formatDiscoveredName(device);
            // P012: 共享外壳（交互容器 + 图标 + 名称行），副标题/操作区注入
            return (
              <DeviceCardShell
                key={deviceAddr}
                clientType={device.clientType}
                name={deviceName}
                subtitle={
                  <div
                    style={{ fontSize: 'var(--text-badge)', color: 'var(--text-tertiary)' }}
                  >
                    {deviceAddr}
                  </div>
                }
                actions={
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      onSyncWithDevice(deviceAddr);
                    }}
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
                }
                onOpen={() => onOpenDiscoveredDetail(device)}
              />
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
  );
}
