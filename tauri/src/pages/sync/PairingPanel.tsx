import { useTranslation } from 'react-i18next';
import { Card } from '@/components/ui/Card';
import { QrCode, ScanLine } from 'lucide-react';
import { PairingDialog } from '@/components/sync/PairingDialog';
import { SyncShowQrDialog } from '@/components/sync/SyncShowQrDialog';
import { SyncScanQrDialog } from '@/components/sync/SyncScanQrDialog';
import { ICON_SIZE } from '@/lib/constants';
import type { SyncPeer } from '@/stores/syncStore';

interface PairingPanelProps {
  syncEnabled: boolean;
  isLoading: boolean;
  pairPeer: SyncPeer | null;
  isWaitingFlow: boolean;
  pairWaitState: 'idle' | 'waiting' | 'failed';
  confirmLabel?: string;
  showQrOpen: boolean;
  scanQrOpen: boolean;
  onShowQrOpen: (open: boolean) => void;
  onScanQrOpen: (open: boolean) => void;
  onScanSync: (addr: string, fingerprint: string) => void | Promise<void>;
  onTrust: () => void;
  onIgnore: () => void;
  onCancelWaiting: () => void;
}

/**
 * 配对面板：QR 配对卡片 + 配对等待/展示/扫描三个对话框。
 * 数据与回调经 SyncPage 从 useSyncPage 透传（P224-② 拆分）。
 */
export function PairingPanel({
  syncEnabled,
  isLoading,
  pairPeer,
  isWaitingFlow,
  pairWaitState,
  confirmLabel,
  showQrOpen,
  scanQrOpen,
  onShowQrOpen,
  onScanQrOpen,
  onScanSync,
  onTrust,
  onIgnore,
  onCancelWaiting,
}: PairingPanelProps) {
  const { t } = useTranslation(['settings']);
  return (
    <>
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
            {/* 对话框内含恢复二维码 tab，恢复会话不依赖同步启用，故不限制 syncEnabled */}
            <button
              type="button"
              onClick={() => onShowQrOpen(true)}
              disabled={isLoading}
              title={t('settings:sync_qr_show', { defaultValue: 'Show QR' })}
              aria-label={t('settings:sync_qr_show', { defaultValue: 'Show QR' })}
              className="interactive-toolbar"
              style={{
                width: 40,
                height: 40,
                borderRadius: 8,
                borderWidth: 1,
                borderStyle: 'solid',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                flexShrink: 0,
                cursor: isLoading ? 'default' : 'pointer',
                opacity: isLoading ? 0.6 : 1,
                fontFamily: 'inherit',
              }}
            >
              <QrCode size={ICON_SIZE.lg} />
            </button>
            <button
              type="button"
              onClick={() => onScanQrOpen(true)}
              disabled={!syncEnabled || isLoading}
              title={t('settings:sync_qr_scan', { defaultValue: 'Scan QR' })}
              aria-label={t('settings:sync_qr_scan', { defaultValue: 'Scan QR' })}
              className="interactive-toolbar"
              style={{
                width: 40,
                height: 40,
                borderRadius: 8,
                borderWidth: 1,
                borderStyle: 'solid',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                flexShrink: 0,
                cursor: !syncEnabled || isLoading ? 'default' : 'pointer',
                opacity: !syncEnabled || isLoading ? 0.6 : 1,
                fontFamily: 'inherit',
              }}
            >
              <ScanLine size={ICON_SIZE.lg} />
            </button>
          </div>
        </div>
      </Card>

      <PairingDialog
        isOpen={!!pairPeer}
        peer={pairPeer}
        waiting={isWaitingFlow && pairWaitState === 'waiting'}
        waitFailed={isWaitingFlow && pairWaitState === 'failed'}
        onTrust={onTrust}
        onIgnore={onIgnore}
        onCancelWaiting={onCancelWaiting}
        confirmLabel={confirmLabel}
      />
      <SyncShowQrDialog isOpen={showQrOpen} onClose={() => onShowQrOpen(false)} />
      <SyncScanQrDialog
        isOpen={scanQrOpen}
        onClose={() => onScanQrOpen(false)}
        onSync={onScanSync}
      />
    </>
  );
}
