import { useState, useEffect, useCallback, useMemo } from 'react';
import { useNavigate, useLocation } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
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
  Trash2,
  ChevronDown,
  ChevronUp,
} from 'lucide-react';
import { useSyncStore } from '@/stores/syncStore';
import type { SyncConflict } from '@/lib/ipc';
import { ICON_SIZE } from '@/lib/iconSizes';


function formatNodeId(bytes: number[]): string {
  return bytes.map((b) => b.toString(16).padStart(2, '0')).join('');
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

  const pendingPeer = useMemo(() => {
    return (
      store.connectedPeers.find((p) => !p.trusted && !ignoredPeerIds.has(p.id) && p.fingerprint) ||
      null
    );
  }, [store.connectedPeers, ignoredPeerIds]);

  const loadStatus = useCallback(async () => {
    await store.loadStatus();
  }, [store]);

  useEffect(() => {
    loadStatus();
  }, [loadStatus]);

  const handleToggleSync = async () => {
    await store.enable(!store.syncEnabled);
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

  return (
    <AppShell
      title={t('settings:sync', { defaultValue: 'Device Sync' })}
      onBack={() => navigate(backTo || '/home', { replace: true })}
    >
      <PageContainer variant="xs" gap="default">
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
                  e.currentTarget.style.background = 'color-mix(in srgb, var(--accent-primary) 12%, transparent)';
                  e.currentTarget.style.borderColor = 'var(--accent-primary)';
                  e.currentTarget.style.color = 'var(--accent-primary)';
                }
              }}
              onMouseLeave={(e) => {
                if (!store.isLoading) {
                  e.currentTarget.style.background = 'var(--bg-toolbar)';
                  e.currentTarget.style.borderColor = 'var(--border-subtle)';
                  e.currentTarget.style.color = store.syncEnabled ? 'var(--accent-primary)' : 'var(--text-primary)';
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
        </Card>

        {/* Manual sync */}
        <Card>
          <h3 style={{ fontSize: 'var(--text-sm)', fontWeight: 600, marginBottom: 12 }}>
            {t('settings:sync_with_device', { defaultValue: 'Sync with Device' })}
          </h3>
          <p style={{ fontSize: 'var(--text-caption)', color: 'var(--text-tertiary)', marginBottom: 12 }}>
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
                  e.currentTarget.style.background = 'color-mix(in srgb, var(--accent-primary) 12%, transparent)';
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
            <p style={{ fontSize: 'var(--text-caption)', color: 'var(--text-secondary)', marginTop: 8 }}>
              {t('settings:sync_result', { defaultValue: 'Result' })}: {store.lastResult.summary}
            </p>
          )}
          {store.error && (
            <p style={{ fontSize: 'var(--text-caption)', color: '#e74c3c', marginTop: 8 }}>{store.error}</p>
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
                    <div style={{ fontSize: 'var(--text-body-sm)', fontWeight: 500 }}>{peer.name || peer.id}</div>
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
                    <Button
                      size="sm"
                      variant="secondary"
                      onClick={() => store.forgetPeer(peer.id)}
                      title={t('settings:sync_forget', { defaultValue: 'Forget' })}
                    >
                      <Trash2 size={ICON_SIZE.sm} />
                    </Button>
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <p style={{ fontSize: 'var(--text-caption)', color: 'var(--text-tertiary)', marginTop: 8 }}>
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
    </AppShell>
  );
}
