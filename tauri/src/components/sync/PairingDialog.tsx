import { useTranslation } from 'react-i18next';
import { ShieldAlert, Smartphone } from 'lucide-react';
import { Dialog } from '@/components/ui/Dialog';
import { Button } from '@/components/ui/Button';
import type { SyncPeer } from '@/stores/syncStore';

interface PairingDialogProps {
  isOpen: boolean;
  peer: SyncPeer | null;
  onTrust: () => void;
  onIgnore: () => void;
}

export function PairingDialog({ isOpen, peer, onTrust, onIgnore }: PairingDialogProps) {
  const { t } = useTranslation(['settings', 'common']);
  if (!peer) return null;

  return (
    <Dialog
      isOpen={isOpen}
      onClose={onIgnore}
      title={t('settings:sync_pairing_title', { defaultValue: 'Pair New Device' })}
    >
      <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 12,
            padding: 12,
            borderRadius: 10,
            background: 'var(--danger-subtle)',
            color: 'var(--danger)',
          }}
        >
          <ShieldAlert size={22} />
          <span style={{ fontSize: 'var(--text-body-sm)', lineHeight: 1.5 }}>
            {t('settings:sync_pairing_warning', {
              defaultValue:
                'Only trust devices you physically own or control. An attacker could impersonate your data.',
            })}
          </span>
        </div>

        <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
          <div
            style={{
              width: 40,
              height: 40,
              borderRadius: 10,
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              background: 'var(--bg-toolbar)',
            }}
          >
            <Smartphone size={20} style={{ color: 'var(--accent-primary)' }} />
          </div>
          <div>
            <div style={{ fontSize: 'var(--text-card-title)', fontWeight: 600 }}>{peer.name || peer.id}</div>
            <div style={{ fontSize: 'var(--text-caption)', color: 'var(--text-tertiary)' }}>
              {peer.addr || 'offline'}
            </div>
          </div>
        </div>

        <div>
          <div style={{ fontSize: 'var(--text-caption)', color: 'var(--text-secondary)', marginBottom: 6 }}>
            {t('settings:sync_pairing_verify_prompt', {
              defaultValue:
                'Verify the fingerprint below matches the one shown on the other device:',
            })}
          </div>
          <div
            style={{
              padding: 12,
              borderRadius: 8,
              background: 'var(--bg-toolbar)',
              fontFamily: 'monospace',
              fontSize: 'var(--text-caption)',
              wordBreak: 'break-all',
              color: 'var(--text-primary)',
            }}
          >
            {peer.fingerprint ||
              t('settings:sync_pairing_no_fingerprint', {
                defaultValue: 'No fingerprint available',
              })}
          </div>
        </div>

        <div style={{ display: 'flex', gap: 10, justifyContent: 'flex-end', marginTop: 8 }}>
          <Button variant="secondary" onClick={onIgnore}>
            {t('settings:sync_pairing_ignore', { defaultValue: 'Ignore' })}
          </Button>
          <Button onClick={onTrust}>
            {t('settings:sync_pairing_trust', { defaultValue: 'Trust & Pair' })}
          </Button>
        </div>
      </div>
    </Dialog>
  );
}
