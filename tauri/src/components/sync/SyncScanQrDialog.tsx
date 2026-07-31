import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { X, CheckCircle2 } from 'lucide-react';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { RecoveryQrScanner } from '@/components/recovery/RecoveryQrScanner';

type QrType = 'sync' | 'unknown';

interface ParsedQr {
  type: QrType;
  addr: string;
  fingerprint?: string;
  deviceName?: string;
  raw: string;
}

interface SyncScanQrDialogProps {
  isOpen: boolean;
  onClose: () => void;
  onSync?: (addr: string, fingerprint: string) => void | Promise<void>;
}

export function SyncScanQrDialog({ isOpen, onClose, onSync }: SyncScanQrDialogProps) {
  const { t } = useTranslation(['common']);
  const [error, setError] = useState<string | null>(null);
  const [scanned, setScanned] = useState<ParsedQr | null>(null);
  const [processing, setProcessing] = useState(false);
  const [success, setSuccess] = useState<string | null>(null);

  if (!isOpen) return null;

  const handleScan = (text: string) => {
    try {
      const parsed = JSON.parse(text);
      const type = parsed.t === 'sync' ? 'sync' : 'unknown';
      if (type === 'unknown') {
        setError(t('common:sync_qr_unrecognized'));
        setSuccess(null);
        return;
      }
      setError(null);
      setSuccess(null);
      setScanned({
        type,
        addr: parsed.a || '',
        fingerprint: parsed.f,
        deviceName: parsed.n,
        raw: text,
      });
    } catch {
      setError(t('common:sync_qr_invalid_payload'));
    }
  };

  const handleClose = () => {
    setError(null);
    setScanned(null);
    setProcessing(false);
    setSuccess(null);
    onClose();
  };

  const handleConfirmSync = async () => {
    if (!scanned || scanned.type !== 'sync' || !onSync) return;
    setProcessing(true);
    setError(null);
    try {
      await onSync(scanned.addr, scanned.fingerprint || '');
      setSuccess(t('common:sync_qr_success_sync') ?? 'Sync completed');
    } catch (err) {
      setError(String(err));
    } finally {
      setProcessing(false);
    }
  };

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        zIndex: 'var(--z-modal)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        background: 'var(--bg-overlay)',
        backdropFilter: 'blur(4px)',
        padding: 16,
      }}
      onClick={(e) => {
        if (e.target === e.currentTarget) handleClose();
      }}
    >
      <Card
        style={{
          maxWidth: 420,
          width: '100%',
          padding: 24,
          position: 'relative',
        }}
      >
        <button
          type="button"
          onClick={handleClose}
          style={{
            position: 'absolute',
            top: 12,
            right: 12,
            background: 'none',
            border: 'none',
            cursor: 'pointer',
            color: 'var(--text-tertiary)',
          }}
          aria-label={t('common:close')}
        >
          <X size={20} />
        </button>

        <h2
          style={{
            fontSize: 'var(--text-card-title)',
            fontWeight: 700,
            margin: '0 0 8px',
            color: 'var(--text-primary)',
          }}
        >
          {t('common:sync_qr_scan_title')}
        </h2>

        {!scanned ? (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
            <p
              style={{
                fontSize: 'var(--text-body-sm)',
                color: 'var(--text-secondary)',
                margin: '0 0 12px',
                lineHeight: 1.5,
              }}
            >
              {t('common:sync_qr_scan_desc')}
            </p>
            <RecoveryQrScanner onScan={handleScan} onCancel={handleClose} />
            {error && (
              <div style={{ color: '#e74c3c', fontSize: 'var(--text-body-sm)', textAlign: 'center' }}>
                {error}
              </div>
            )}
          </div>
        ) : success ? (
          <div style={{ textAlign: 'center', padding: '12px 0' }}>
            <div
              style={{
                width: 56,
                height: 56,
                borderRadius: '50%',
                background: 'rgba(39,174,96,0.12)',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                margin: '0 auto 16px',
              }}
            >
              <CheckCircle2 size={32} color="#27ae60" />
            </div>
            <h3
              style={{
                fontSize: 'var(--text-body)',
                fontWeight: 600,
                margin: '0 0 8px',
                color: 'var(--text-primary)',
              }}
            >
              {success}
            </h3>
            <Button onClick={handleClose} style={{ width: '100%', marginTop: 12 }}>
              {t('common:close')}
            </Button>
          </div>
        ) : (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
            <p
              style={{
                fontSize: 'var(--text-body-sm)',
                color: 'var(--text-secondary)',
                margin: 0,
                lineHeight: 1.5,
              }}
            >
              {t('common:sync_qr_confirm_sync', { deviceName: scanned.deviceName || scanned.addr })}
            </p>
            <div
              style={{
                padding: '10px 12px',
                borderRadius: 8,
                background: 'var(--bg-toolbar)',
                fontSize: 'var(--text-body-sm)',
                color: 'var(--text-secondary)',
              }}
            >
              <div>
                <strong>{t('common:sync_qr_addr')}:</strong>{' '}
                <span style={{ fontFamily: 'monospace', color: 'var(--text-primary)' }}>
                  {scanned.addr}
                </span>
              </div>
              {scanned.fingerprint && (
                <div style={{ marginTop: 4 }}>
                  <strong>{t('common:sync_qr_fingerprint')}:</strong>{' '}
                  <span style={{ fontFamily: 'monospace', color: 'var(--text-primary)' }}>
                    {scanned.fingerprint}
                  </span>
                </div>
              )}
            </div>
            <Button onClick={handleConfirmSync} disabled={processing} style={{ width: '100%' }}>
              {processing ? t('common:loading') : t('common:sync_qr_confirm_sync_button')}
            </Button>
            {error && (
              <div style={{ color: '#e74c3c', fontSize: 'var(--text-body-sm)', textAlign: 'center' }}>
                {error}
              </div>
            )}
            <Button variant="secondary" onClick={() => setScanned(null)} disabled={processing} style={{ width: '100%' }}>
              {t('common:cancel')}
            </Button>
          </div>
        )}
      </Card>
    </div>
  );
}
