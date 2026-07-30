import { useState, useEffect, useCallback, useMemo } from 'react';
import type { ReactNode } from 'react';
import { useNavigate, useLocation } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { AppShell } from '@/components/layout/AppShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { PairingDialog } from '@/components/sync/PairingDialog';
import {
  Smartphone,
  Wifi,
  WifiOff,
  RefreshCw,
  ShieldOff,
  ChevronDown,
  ChevronUp,
  Info,
  QrCode,
  ScanLine,
} from 'lucide-react';
import { useSyncStore } from '@/stores/syncStore';
import { DeleteButton } from '@/components/ui/DeleteButton';
import { PageGuideButton } from '@/components/guide/PageGuideButton';
import { SyncConflictDialog } from '@/components/sync/SyncConflictDialog';
import { SyncShowQrDialog } from '@/components/sync/SyncShowQrDialog';
import { SyncScanQrDialog } from '@/components/sync/SyncScanQrDialog';
import type { SyncConflict } from '@/lib/ipc';
import { ICON_SIZE } from '@/lib/constants';

function formatNodeId(bytes: number[]): string {
  return bytes.map((b) => b.toString(16).padStart(2, '0')).join('');
}

interface SyncIconButtonProps {
  label: string;
  icon: ReactNode;
  onClick: () => void;
  disabled: boolean;
}

function SyncIconButton({ label, icon, onClick, disabled }: SyncIconButtonProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={disabled}
      title={label}
      aria-label={label}
      style={{
        width: 40,
        height: 40,
        borderRadius: 8,
        border: '1px solid var(--border-subtle)',
        background: 'var(--bg-toolbar)',
        color: 'var(--text-primary)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        flexShrink: 0,
        cursor: disabled ? 'default' : 'pointer',
        opacity: disabled ? 0.6 : 1,
        transition: 'all 0.15s ease',
        fontFamily: 'inherit',
      }}
      onMouseEnter={(e) => {
        if (disabled) return;
        e.currentTarget.style.background =
          'color-mix(in srgb, var(--accent-primary) 12%, transparent)';
        e.currentTarget.style.borderColor = 'var(--accent-primary)';
        e.currentTarget.style.color = 'var(--accent-primary)';
      }}
      onMouseLeave={(e) => {
        if (disabled) return;
        e.currentTarget.style.background = 'var(--bg-toolbar)';
        e.currentTarget.style.borderColor = 'var(--border-subtle)';
        e.currentTarget.style.color = 'var(--text-primary)';
      }}
    >
      {icon}
    </button>
  );
}

function formatHlc(hlc: SyncConflict['local_hlc']): string {
  return `${hlc.wall_time_ms}-${hlc.counter}-${formatNodeId(hlc.node_id)}`;
}

export function SyncPage() {
  const navigate = useNavigate();
  const location = useLocation();
  const backTo = (location.state as { from?: string } | null)?.from;
  const { t } = useTranslation(['settings', 'common']);
  const store = useSyncStore();
  const [manualAddr, setManualAddr] = useState('');
  const [ignoredPeerIds, setIgnoredPeerIds] = useState<Set<string>>(new Set());
  const [activityOpen, setActivityOpen] = useState(false);
  const [conflictDialogOpen, setConflictDialogOpen] = useState(false);
  const [showQrDialogOpen, setShowQrDialogOpen] = useState(false);
  const [scanQrDialogOpen, setScanQrDialogOpen] = useState(false);

  const syncGuidePages = useMemo(
    () => [
      {
        icon: Info,
        title: t('common:guide_sync_title') ?? 'Device Sync Guide',
        steps: [
          {
            icon: Wifi,
            title: t('common:guide_sync_step1_title') ?? 'Enable Sync',
            description:
              t('common:guide_sync_step1_desc') ??
              'Turn on sync to make your device discoverable and start listening on a local port. Both devices must be on the same Wi-Fi network.',
          },
          {
            icon: RefreshCw,
            title: t('common:guide_sync_step2_title') ?? 'Discover & Pair',
            description:
              t('common:guide_sync_step2_desc') ??
              'Tap Discover to scan for nearby devices. Tap Sync on a discovered device to pair, then verify the fingerprint to trust it.',
          },
          {
            icon: Smartphone,
            title: t('common:guide_sync_step3_title') ?? 'Automatic Sync',
            description:
              t('common:guide_sync_step3_desc') ??
              'Enable Automatic Sync to keep data in sync when the app is in the foreground, on data changes, and periodically.',
          },
        ],
        helpLinks: [
          {
            title: t('common:guide_help_device_sync') ?? 'Device Sync',
            description:
              t('common:guide_help_device_sync_desc') ??
              'Pair devices over LAN and keep data in sync',
            href: '/help?id=device-sync',
          },
        ],
      },
    ],
    [t],
  );

  const pendingPeer = useMemo(() => {
    return (
      store.connectedPeers.find((p) => !p.trusted && !ignoredPeerIds.has(p.id) && p.fingerprint) ||
      null
    );
  }, [store.connectedPeers, ignoredPeerIds]);

  // ⚡ 使用 getState() 避免闭包捕获整个 store 导致无限重触发
  const loadStatus = useCallback(async () => {
    const s = useSyncStore.getState();
    await Promise.all([
      s.loadStatus(),
      s.loadListenPort(),
      s.loadAutoSyncStatus(),
      s.loadConflicts(),
    ]);
  }, []);

  useEffect(() => {
    loadStatus();
  }, [loadStatus]);

  // 使用 selector 只监听 syncEnabled 变化，避免 store 全局变化时误触发
  const syncEnabled = useSyncStore((s) => s.syncEnabled);
  useEffect(() => {
    if (syncEnabled) {
      useSyncStore.getState().discoverDevices(5000);
    }
  }, [syncEnabled]);

  const handleToggleSync = async () => {
    await store.enable(!store.syncEnabled);
  };

  const handleToggleAutoSync = async () => {
    await store.setAutoSyncEnabled(!store.autoSyncEnabled);
  };

  const handleDiscover = async () => {
    await store.discoverDevices(5000);
  };

  const handleSyncWithDevice = async (deviceId: string) => {
    await store.syncWithDevice(deviceId);
  };

  const handleTrustPending = async () => {
    if (!pendingPeer) return;
    await store.trustPeer(pendingPeer.id, true);
  };

  const handleIgnorePending = () => {
    if (!pendingPeer) return;
    setIgnoredPeerIds((prev) => new Set(prev).add(pendingPeer.id));
  };

  const handleOpenConflictDialog = () => {
    setConflictDialogOpen(true);
  };

  const handleScanSync = async (addr: string) => {
    await store.syncWithDevice(addr);
  };

  const handleScanRecovery = async (payload: {
    hostAddr: string;
    pin: string;
    fingerprint?: string;
    nonce?: string;
  }) => {
    await invoke('recovery_host_push', {
      hostAddr: payload.hostAddr,
      pin: payload.pin,
      fingerprint: payload.fingerprint,
      nonce: payload.nonce,
    });
  };

  return (
    <AppShell
      title={t('settings:sync', { defaultValue: 'Device Sync' })}
      onBack={() => navigate(backTo || '/home', { replace: true })}
      actions={<PageGuideButton pages={syncGuidePages} />}
    >
      <PageContainer variant="xs" gap="default">

        {/* Conflicts card */}
        {store.conflicts.length > 0 && (
          <Card>
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
              <div>
                <div style={{ fontSize: 'var(--text-card-title)', fontWeight: 600 }}>
                  {t('settings:sync_conflicts_title', { defaultValue: 'Sync Conflicts' })}
                </div>
                <div style={{ fontSize: 'var(--text-caption)', color: 'var(--text-tertiary)' }}>
                  {t('settings:sync_conflicts_desc', {
                    defaultValue: `${store.conflicts.length} unresolved conflict(s) need your attention.`,
                  })}
                </div>
              </div>
              <button
                onClick={handleOpenConflictDialog}
                style={{
                  padding: '8px 16px',
                  borderRadius: 8,
                  border: '1px solid #c0392b',
                  background: 'rgba(192,57,43,0.08)',
                  color: '#c0392b',
                  fontSize: 'var(--text-body-sm)',
                  fontWeight: 500,
                  cursor: 'pointer',
                  transition: 'all 0.15s ease',
                  fontFamily: 'inherit',
                }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.background = 'rgba(192,57,43,0.12)';
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.background = 'rgba(192,57,43,0.08)';
                }}
              >
                {t('settings:sync_review_conflicts', { defaultValue: 'Review' })}
              </button>
            </div>
          </Card>
        )}

        {/* Status card */}
        <Card>
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
              <div
                style={{
                  width: 40,
                  height: 40,
                  borderRadius: 10,
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  background: store.syncEnabled ? 'rgba(39,174,96,0.12)' : 'rgba(128,128,128,0.1)',
                }}
              >
                {store.syncEnabled ? (
                  <Wifi size={ICON_SIZE.xl} color="#27ae60" />
                ) : (
                  <WifiOff size={ICON_SIZE.xl} color="#888" />
                )}
              </div>
              <div>
                <div style={{ fontSize: 'var(--text-card-title)', fontWeight: 600 }}>
                  {store.syncEnabled
                    ? t('settings:sync_enabled', { defaultValue: 'Sync Enabled' })
                    : t('settings:sync_disabled', { defaultValue: 'Sync Disabled' })}
                </div>
                <div style={{ fontSize: 'var(--text-caption)', color: 'var(--text-tertiary)' }}>
                  {t('settings:sync_known_devices', {
                    count: store.connectedPeers.length,
                    defaultValue: `${store.connectedPeers.length} device(s) known`,
                  })}
                </div>
              </div>
            </div>
            <button
              onClick={handleToggleSync}
              disabled={store.isLoading}
              onMouseEnter={(e) => {
                if (!store.isLoading) {
                  e.currentTarget.style.background =
                    'color-mix(in srgb, var(--accent-primary) 12%, transparent)';
                  e.currentTarget.style.borderColor = 'var(--accent-primary)';
                  e.currentTarget.style.color = 'var(--accent-primary)';
                }
              }}
              onMouseLeave={(e) => {
                if (!store.isLoading) {
                  e.currentTarget.style.background = 'var(--bg-toolbar)';
                  e.currentTarget.style.borderColor = 'var(--border-subtle)';
                  e.currentTarget.style.color = store.syncEnabled
                    ? 'var(--accent-primary)'
                    : 'var(--text-primary)';
                }
              }}
              style={{
                padding: '8px 16px',
                borderRadius: 8,
                border: store.syncEnabled
                  ? '1px solid var(--accent-primary)'
                  : '1px solid var(--border-subtle)',
                background: 'var(--bg-toolbar)',
                color: store.syncEnabled ? 'var(--accent-primary)' : 'var(--text-primary)',
                fontSize: 'var(--text-body-sm)',
                fontWeight: 500,
                cursor: store.isLoading ? 'default' : 'pointer',
                opacity: store.isLoading ? 0.6 : 1,
                transition: 'all 0.15s ease',
                fontFamily: 'inherit',
              }}
            >
              {store.syncEnabled
                ? t('settings:sync_disable', { defaultValue: 'Disable' })
                : t('settings:sync_enable', { defaultValue: 'Enable' })}
            </button>
          </div>

          {/* Auto-sync toggle */}
          <div
            style={{
              marginTop: 12,
              padding: 10,
              borderRadius: 8,
              background: 'var(--bg-toolbar)',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between',
            }}
          >
            <div>
              <div style={{ fontSize: 'var(--text-body-sm)', fontWeight: 500 }}>
                {t('settings:sync_auto', { defaultValue: 'Automatic Sync' })}
              </div>
              <div style={{ fontSize: 'var(--text-caption)', color: 'var(--text-tertiary)' }}>
                {t('settings:sync_auto_desc', {
                  defaultValue: 'Sync automatically on foreground, data changes, and periodically.',
                })}
              </div>
            </div>
            <button
              onClick={handleToggleAutoSync}
              disabled={!store.syncEnabled || store.isLoading}
              style={{
                padding: '8px 16px',
                borderRadius: 8,
                border: store.autoSyncEnabled
                  ? '1px solid var(--accent-primary)'
                  : '1px solid var(--border-subtle)',
                background: 'var(--bg-elevated)',
                color: store.autoSyncEnabled ? 'var(--accent-primary)' : 'var(--text-primary)',
                fontSize: 'var(--text-body-sm)',
                fontWeight: 500,
                cursor: !store.syncEnabled || store.isLoading ? 'default' : 'pointer',
                opacity: !store.syncEnabled || store.isLoading ? 0.6 : 1,
                transition: 'all 0.15s ease',
                fontFamily: 'inherit',
              }}
            >
              {store.autoSyncEnabled
                ? t('settings:sync_auto_on', { defaultValue: 'On' })
                : t('settings:sync_auto_off', { defaultValue: 'Off' })}
            </button>
          </div>

          {store.localFingerprint && (
            <div
              style={{
                marginTop: 12,
                padding: 10,
                borderRadius: 8,
                background: 'var(--bg-toolbar)',
                fontSize: 'var(--text-caption)',
                fontFamily: 'monospace',
                wordBreak: 'break-all',
              }}
            >
              <strong>
                {t('settings:sync_your_fingerprint', { defaultValue: 'Your fingerprint' })}:
              </strong>{' '}
              {store.localFingerprint}
            </div>
          )}
          {store.syncEnabled && store.listenPort !== 0 && (
            <div
              style={{
                marginTop: 8,
                padding: 10,
                borderRadius: 8,
                background: 'var(--bg-toolbar)',
                fontSize: 'var(--text-caption)',
                color: 'var(--text-secondary)',
              }}
            >
              <strong>
                {t('settings:sync_your_port', { defaultValue: 'Your listen port' })}:
              </strong>{' '}
              {store.listenPort}
            </div>
          )}
        </Card>

        {/* QR pairing */}
        <Card>
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
            <div>
              <div style={{ fontSize: 'var(--text-card-title)', fontWeight: 600 }}>
                {t('settings:sync_qr_pairing', { defaultValue: 'QR Pairing' })}
              </div>
              <div style={{ fontSize: 'var(--text-caption)', color: 'var(--text-tertiary)' }}>
                {t('settings:sync_qr_pairing_desc', {
                  defaultValue: 'Show a QR code for another device to scan, or scan another device.',
                })}
              </div>
            </div>
            <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
              <SyncIconButton
                label={t('settings:sync_qr_show', { defaultValue: 'Show QR' })}
                icon={<QrCode size={ICON_SIZE.lg} />}
                onClick={() => setShowQrDialogOpen(true)}
                disabled={!store.syncEnabled || store.isLoading}
              />
              <SyncIconButton
                label={t('settings:sync_qr_scan', { defaultValue: 'Scan QR' })}
                icon={<ScanLine size={ICON_SIZE.lg} />}
                onClick={() => setScanQrDialogOpen(true)}
                disabled={!store.syncEnabled || store.isLoading}
              />
            </div>
          </div>
        </Card>

        {/* Discovered devices */}
        <Card>
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
            <h3 style={{ fontSize: 'var(--text-sm)', fontWeight: 600 }}>
              {t('settings:sync_discovered_devices', { defaultValue: 'Discovered Devices' })}
            </h3>
            <button
              onClick={handleDiscover}
              disabled={!store.syncEnabled || store.isDiscoveringDevices}
              style={{
                padding: '6px 12px',
                borderRadius: 8,
                border: '1px solid var(--border-subtle)',
                background: 'var(--bg-toolbar)',
                color: 'var(--text-primary)',
                fontSize: 'var(--text-body-sm)',
                fontWeight: 500,
                cursor: !store.syncEnabled || store.isDiscoveringDevices ? 'default' : 'pointer',
                opacity: !store.syncEnabled || store.isDiscoveringDevices ? 0.5 : 1,
                transition: 'all 0.15s ease',
                fontFamily: 'inherit',
              }}
            >
              {store.isDiscoveringDevices
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
          {store.discoveredDevices.length > 0 ? (
            <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
              {store.discoveredDevices.map((device) => {
                const deviceAddr =
                    device.addresses[0] || `${device.host}:${device.port}`;
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
                      {device.name || t('settings:sync_unknown_device', { defaultValue: 'Unknown device' })}
                    </div>
                    <div style={{ fontSize: 'var(--text-badge)', color: 'var(--text-tertiary)' }}>
                      {deviceAddr}
                    </div>
                  </div>
                  <button
                    onClick={() => handleSyncWithDevice(deviceAddr)}
                    disabled={store.isLoading}
                    style={{
                      padding: '6px 12px',
                      borderRadius: 8,
                      border: '1px solid var(--border-subtle)',
                      background: 'var(--bg-toolbar)',
                      color: 'var(--text-primary)',
                      fontSize: 'var(--text-body-sm)',
                      fontWeight: 500,
                      cursor: store.isLoading ? 'default' : 'pointer',
                      opacity: store.isLoading ? 0.5 : 1,
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
                {t('settings:sync_no_devices_found', { defaultValue: 'No devices found. Click Discover to scan.' })}
              </p>
              {store.syncEnabled && (
                <p
                  style={{
                    fontSize: 'var(--text-caption)',
                    color: 'var(--text-tertiary)',
                    marginTop: 8,
                  }}
                >
                  {t('settings:sync_manual_fallback_hint', {
                    defaultValue:
                      'If automatic discovery fails, ensure both devices are on the same Wi-Fi and enter the other device\'s IP address with port below.',
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
              onChange={(e) => setManualAddr(e.target.value)}
              style={{ flex: 1 }}
            />
            <button
              onClick={() => handleSyncWithDevice(manualAddr)}
              disabled={!manualAddr.trim() || store.isLoading}
              onMouseEnter={(e) => {
                if (manualAddr.trim() && !store.isLoading) {
                  e.currentTarget.style.background =
                    'color-mix(in srgb, var(--accent-primary) 12%, transparent)';
                  e.currentTarget.style.borderColor = 'var(--accent-primary)';
                  e.currentTarget.style.color = 'var(--accent-primary)';
                }
              }}
              onMouseLeave={(e) => {
                if (manualAddr.trim() && !store.isLoading) {
                  e.currentTarget.style.background = 'var(--bg-toolbar)';
                  e.currentTarget.style.borderColor = 'var(--border-subtle)';
                  e.currentTarget.style.color = 'var(--text-primary)';
                }
              }}
              style={{
                padding: '8px 16px',
                borderRadius: 8,
                border: '1px solid var(--border-subtle)',
                background: 'var(--bg-toolbar)',
                color: 'var(--text-primary)',
                fontSize: 'var(--text-body-sm)',
                fontWeight: 500,
                cursor: !manualAddr.trim() || store.isLoading ? 'default' : 'pointer',
                opacity: !manualAddr.trim() || store.isLoading ? 0.5 : 1,
                transition: 'all 0.15s ease',
                fontFamily: 'inherit',
                whiteSpace: 'nowrap',
              }}
            >
              {store.isLoading
                ? t('common:loading', { defaultValue: 'Loading...' })
                : t('settings:sync_manual_sync', { defaultValue: 'Sync' })}
            </button>
          </div>
          {store.lastResult && (
            <p
              style={{
                fontSize: 'var(--text-caption)',
                color: 'var(--text-secondary)',
                marginTop: 8,
              }}
            >
              {t('settings:sync_result', { defaultValue: 'Result' })}: {store.lastResult.summary}
            </p>
          )}
          {store.error && (
            <p style={{ fontSize: 'var(--text-caption)', color: '#e74c3c', marginTop: 8 }}>
              {store.error}
            </p>
          )}
        </Card>

        {/* Sync activity */}
        {store.recentResults.length > 0 && (
          <Card>
            <button
              type="button"
              onClick={() => setActivityOpen((v) => !v)}
              onMouseEnter={(e) => {
                e.currentTarget.style.color = 'var(--accent-primary)';
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.color = 'inherit';
              }}
              style={{
                width: '100%',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'space-between',
                background: 'none',
                border: 'none',
                padding: '4px 0',
                cursor: 'pointer',
                color: 'inherit',
                transition: 'color 0.15s ease',
              }}
            >
              <h3 style={{ fontSize: 'var(--text-sm)', fontWeight: 600 }}>
                {t('settings:sync_activity_title', { defaultValue: 'Sync Activity' })}
              </h3>
              {activityOpen ? (
                <ChevronUp size={ICON_SIZE.lg} />
              ) : (
                <ChevronDown size={ICON_SIZE.lg} />
              )}
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
                {store.recentResults.map((result, idx) => (
                  <div
                    key={idx}
                    style={{
                      padding: 12,
                      borderRadius: 8,
                      background: 'var(--bg-toolbar)',
                      fontSize: 'var(--text-caption)',
                    }}
                  >
                    <div style={{ fontWeight: 500, marginBottom: 6 }}>{result.summary}</div>
                    {result.per_table.length > 0 && (
                      <div
                        style={{
                          display: 'flex',
                          flexWrap: 'wrap',
                          gap: 6,
                          marginBottom: result.conflicts.length > 0 ? 8 : 0,
                        }}
                      >
                        {result.per_table.map((t) => (
                          <span
                            key={t.table}
                            style={{
                              padding: '2px 8px',
                              borderRadius: 4,
                              background: 'var(--bg-elevated)',
                              color: 'var(--text-secondary)',
                            }}
                          >
                            {t.table}: {t.applied}+{t.skipped}/{t.examined}
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
                              {c.table}/{c.id} —{' '}
                              {t('settings:sync_winner', { defaultValue: 'winner' })}: {c.winner}
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
        )}

        {/* Known peers */}
        <Card>
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
            <h3 style={{ fontSize: 'var(--text-sm)', fontWeight: 600 }}>
              {t('settings:sync_known_devices_title', { defaultValue: 'Known Devices' })}
            </h3>
            <Button size="sm" variant="tertiary" onClick={loadStatus} loading={store.isLoading}>
              <RefreshCw size={ICON_SIZE.sm} />
            </Button>
          </div>

          {store.connectedPeers.length > 0 ? (
            <div style={{ marginTop: 12, display: 'flex', flexDirection: 'column', gap: 8 }}>
              {store.connectedPeers.map((peer) => (
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
                    <div style={{ fontSize: 'var(--text-body-sm)', fontWeight: 500 }}>
                      {peer.name || peer.id}
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
                    {peer.trusted && (
                      <Button
                        size="sm"
                        variant="secondary"
                        onClick={() => store.trustPeer(peer.id, false)}
                        title={t('settings:sync_revoke', { defaultValue: 'Revoke' })}
                      >
                        <ShieldOff size={ICON_SIZE.sm} />
                      </Button>
                    )}
                    <DeleteButton
                      onClick={() => store.forgetPeer(peer.id)}
                      title={t('settings:sync_forget', { defaultValue: 'Forget' })}
                      iconOnly
                    />
                  </div>
                </div>
              ))}
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
      </PageContainer>

      <PairingDialog
        isOpen={!!pendingPeer}
        peer={pendingPeer}
        onTrust={handleTrustPending}
        onIgnore={handleIgnorePending}
      />
      <SyncConflictDialog
        isOpen={conflictDialogOpen}
        conflicts={store.conflicts}
        detail={store.selectedConflict}
        isLoading={store.isLoading}
        onClose={() => setConflictDialogOpen(false)}
        onResolve={(id, strategy) => store.resolveConflict(id, strategy)}
      />
      <SyncShowQrDialog isOpen={showQrDialogOpen} onClose={() => setShowQrDialogOpen(false)} />
      <SyncScanQrDialog
        isOpen={scanQrDialogOpen}
        onClose={() => setScanQrDialogOpen(false)}
        onSync={handleScanSync}
        onRecoveryPush={handleScanRecovery}
      />
    </AppShell>
  );
}
