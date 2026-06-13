import { useState, useEffect, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { Smartphone, Wifi, WifiOff, RefreshCw, Shield, Trash2 } from 'lucide-react';
import { useSyncStore } from '@/stores/syncStore';

export function SyncPage() {
  const navigate = useNavigate();
  const { t } = useTranslation(['settings', 'common']);
  const store = useSyncStore();
  const [manualAddr, setManualAddr] = useState('');

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

  return (
    <AppShell
      title={t('settings:sync', { defaultValue: 'Device Sync' })}
      onBack={() => navigate('/home')}
    >
      <div
        style={{
          maxWidth: 560,
          margin: '0 auto',
          display: 'flex',
          flexDirection: 'column',
          gap: 16,
        }}
      >
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
                  <Wifi size={20} color="#27ae60" />
                ) : (
                  <WifiOff size={20} color="#888" />
                )}
              </div>
              <div>
                <div style={{ fontSize: 15, fontWeight: 600 }}>
                  {store.syncEnabled
                    ? t('settings:sync_enabled', { defaultValue: 'Sync Enabled' })
                    : t('settings:sync_disabled', { defaultValue: 'Sync Disabled' })}
                </div>
                <div style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>
                  {t('settings:sync_known_devices', {
                    count: store.connectedPeers.length,
                    defaultValue: `${store.connectedPeers.length} device(s) known`,
                  })}
                </div>
              </div>
            </div>
            <Button
              variant={store.syncEnabled ? 'secondary' : 'primary'}
              onClick={handleToggleSync}
              loading={store.isLoading}
            >
              {store.syncEnabled
                ? t('settings:sync_disable', { defaultValue: 'Disable' })
                : t('settings:sync_enable', { defaultValue: 'Enable' })}
            </Button>
          </div>

          {store.localFingerprint && (
            <div
              style={{
                marginTop: 12,
                padding: 10,
                borderRadius: 8,
                background: 'var(--bg-toolbar)',
                fontSize: 12,
                fontFamily: 'monospace',
                wordBreak: 'break-all',
              }}
            >
              <strong>{t('settings:sync_your_fingerprint', { defaultValue: 'Your fingerprint' })}:</strong>{' '}
              {store.localFingerprint}
            </div>
          )}
        </Card>

        {/* Manual sync */}
        <Card>
          <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 12 }}>
            {t('settings:sync_with_device', { defaultValue: 'Sync with Device' })}
          </h3>
          <p style={{ fontSize: 12, color: 'var(--text-tertiary)', marginBottom: 12 }}>
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
            <Button
              onClick={() => handleSyncWithDevice(manualAddr)}
              loading={store.isLoading}
              disabled={!manualAddr.trim()}
            >
              {t('settings:sync_manual_sync', { defaultValue: 'Sync' })}
            </Button>
          </div>
          {store.lastResult && (
            <p style={{ fontSize: 12, color: 'var(--text-secondary)', marginTop: 8 }}>
              {t('settings:sync_result', { defaultValue: 'Result' })}: {store.lastResult}
            </p>
          )}
          {store.error && (
            <p style={{ fontSize: 12, color: '#e74c3c', marginTop: 8 }}>{store.error}</p>
          )}
        </Card>

        {/* Known peers */}
        <Card>
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
            <h3 style={{ fontSize: 14, fontWeight: 600 }}>
              {t('settings:sync_known_devices_title', { defaultValue: 'Known Devices' })}
            </h3>
            <Button size="sm" variant="tertiary" onClick={loadStatus} loading={store.isLoading}>
              <RefreshCw size={14} />
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
                  <Smartphone size={18} style={{ color: 'var(--accent-primary)' }} />
                  <div style={{ flex: 1, minWidth: 0 }}>
                    <div style={{ fontSize: 13, fontWeight: 500 }}>{peer.name || peer.id}</div>
                    <div style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>
                      {peer.addr || 'offline'} · {peer.lastSeen || 'never'}
                    </div>
                    <div
                      style={{
                        fontSize: 11,
                        color: 'var(--text-tertiary)',
                        fontFamily: 'monospace',
                        wordBreak: 'break-all',
                      }}
                    >
                      {peer.fingerprint}
                    </div>
                  </div>
                  <div style={{ display: 'flex', gap: 6 }}>
                    <Button
                      size="sm"
                      variant={peer.trusted ? 'secondary' : 'primary'}
                      onClick={() => store.trustPeer(peer.id, !peer.trusted)}
                      title={
                        peer.trusted
                          ? t('settings:sync_revoke', { defaultValue: 'Revoke' })
                          : t('settings:sync_trust', { defaultValue: 'Trust' })
                      }
                    >
                      <Shield size={14} />
                    </Button>
                    <Button
                      size="sm"
                      variant="secondary"
                      onClick={() => store.forgetPeer(peer.id)}
                      title={t('settings:sync_forget', { defaultValue: 'Forget' })}
                    >
                      <Trash2 size={14} />
                    </Button>
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <p style={{ fontSize: 12, color: 'var(--text-tertiary)', marginTop: 8 }}>
              {t('settings:sync_no_devices', {
                defaultValue:
                  'No devices known yet. Enable sync and sync with another device to add it.',
              })}
            </p>
          )}
        </Card>
      </div>
    </AppShell>
  );
}
