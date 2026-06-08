import { useState, useEffect, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { Smartphone, Wifi, WifiOff, RefreshCw } from 'lucide-react';

interface DiscoveredDevice {
  name: string;
  host: string;
  port: number;
  addresses: string[];
}

interface SyncStatus {
  isDiscovering: boolean;
  connectedPeers: Array<{ id: string; name: string; addr: string; lastSeen: string }>;
  syncEnabled: boolean;
}

export function SyncPage() {
  const navigate = useNavigate();
  const { t } = useTranslation(['settings', 'common']);
  const [status, setStatus] = useState<SyncStatus>({
    isDiscovering: false,
    connectedPeers: [],
    syncEnabled: false,
  });
  const [devices, setDevices] = useState<DiscoveredDevice[]>([]);
  const [isScanning, setIsScanning] = useState(false);
  const [isEnabling, setIsEnabling] = useState(false);

  const loadStatus = useCallback(async () => {
    try {
      const s = await invoke<SyncStatus>('sync_get_status');
      setStatus(s);
    } catch { /* backend may not be ready */ }
  }, []);

  useEffect(() => { loadStatus(); }, [loadStatus]);

  const handleDiscover = async () => {
    setIsScanning(true);
    try {
      const result = await invoke<DiscoveredDevice[]>('mdns_discover', { timeoutMs: 3000 });
      setDevices(result);
    } catch (e) {
      console.error('Discovery failed:', e);
    } finally {
      setIsScanning(false);
    }
  };

  const handleToggleSync = async () => {
    setIsEnabling(true);
    try {
      await invoke('sync_enable', { enable: !status.syncEnabled });
      await loadStatus();
    } catch { /* stub */ }
    finally { setIsEnabling(false); }
  };

  return (
    <AppShell title={t('settings:items.sync', { defaultValue: 'Device Sync' })} onBack={() => navigate('/home')}>
      <div style={{ maxWidth: 560, margin: '0 auto', display: 'flex', flexDirection: 'column', gap: 16 }}>
        {/* Sync status card */}
        <Card>
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
              <div style={{
                width: 40, height: 40, borderRadius: 10, display: 'flex', alignItems: 'center', justifyContent: 'center',
                background: status.syncEnabled ? 'rgba(39,174,96,0.12)' : 'rgba(128,128,128,0.1)',
              }}>
                {status.syncEnabled ? <Wifi size={20} color="#27ae60" /> : <WifiOff size={20} color="#888" />}
              </div>
              <div>
                <div style={{ fontSize: 15, fontWeight: 600 }}>
                  {status.syncEnabled ? 'Sync Enabled' : 'Sync Disabled'}
                </div>
                <div style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>
                  {status.connectedPeers.length} device(s) connected
                </div>
              </div>
            </div>
            <Button
              variant={status.syncEnabled ? 'secondary' : 'primary'}
              onClick={handleToggleSync}
              loading={isEnabling}
            >
              {status.syncEnabled ? 'Disable' : 'Enable'}
            </Button>
          </div>
        </Card>

        {/* Device discovery */}
        <Card>
          <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 12 }}>Discover Devices</h3>
          <p style={{ fontSize: 12, color: 'var(--text-tertiary)', marginBottom: 12 }}>
            Scan your local network for SoloSoul devices via mDNS.
          </p>
          <Button variant="secondary" onClick={handleDiscover} loading={isScanning}>
            <RefreshCw size={14} style={{ marginRight: 6 }} />
            Scan Network
          </Button>

          {devices.length > 0 && (
            <div style={{ marginTop: 12, display: 'flex', flexDirection: 'column', gap: 8 }}>
              {devices.map((d, i) => (
                <div
                  key={i}
                  style={{
                    display: 'flex', alignItems: 'center', gap: 10, padding: '10px 12px',
                    borderRadius: 8, background: 'var(--bg-toolbar)',
                  }}
                >
                  <Smartphone size={18} style={{ color: 'var(--accent-primary)' }} />
                  <div style={{ flex: 1 }}>
                    <div style={{ fontSize: 13, fontWeight: 500 }}>{d.name}</div>
                    <div style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>
                      {d.host}:{d.port}
                    </div>
                  </div>
                  <Button size="sm" onClick={() => {
                    invoke('sync_with_device', { deviceId: d.name }).catch(() => {});
                  }}>
                    Connect
                  </Button>
                </div>
              ))}
            </div>
          )}

          {!isScanning && devices.length === 0 && (
            <p style={{ fontSize: 12, color: 'var(--text-tertiary)', marginTop: 8 }}>
              No devices found on the local network.
            </p>
          )}
        </Card>

        {/* Connected peers */}
        {status.connectedPeers.length > 0 && (
          <Card>
            <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 12 }}>Connected Devices</h3>
            {status.connectedPeers.map((peer) => (
              <div key={peer.id} style={{ display: 'flex', alignItems: 'center', gap: 10, padding: '8px 0' }}>
                <Smartphone size={16} style={{ color: 'var(--accent-primary)' }} />
                <div>
                  <div style={{ fontSize: 13, fontWeight: 500 }}>{peer.name}</div>
                  <div style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>
                    {peer.addr} · Last seen: {peer.lastSeen}
                  </div>
                </div>
              </div>
            ))}
          </Card>
        )}
      </div>
    </AppShell>
  );
}
