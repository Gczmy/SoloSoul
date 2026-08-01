import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { X, CheckCircle2, CameraOff } from 'lucide-react';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { useCameraCapability } from '@/hooks/useCameraCapability';
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
  // 设备摄像头能力（启动时预加载，模块级缓存）：无摄像头时扫码位直接提示
  const cameraCapability = useCameraCapability();
  const [error, setError] = useState<string | null>(null);
  const [scanned, setScanned] = useState<ParsedQr | null>(null);
  const [processing, setProcessing] = useState(false);
  const [success, setSuccess] = useState<string | null>(null);
  // 扫码器启动失败（如权限被拒）时置位，展示「使用设备发现/手动输入」兜底
  const [scannerError, setScannerError] = useState<string | null>(null);

  if (!isOpen) return null;

  const handleScan = (text: string) => {
    try {
      const parsed = JSON.parse(text);
      const type = parsed.t === 'sync' ? 'sync' : 'unknown';
      if (type === 'unknown') {
        // 恢复二维码（t:"rec"）的消费端是登录页「从其他设备恢复」流程，
        // 这里给出明确指引，替代笼统的「无法识别」提示。
        if (parsed.t === 'rec') {
          setError(
            t('common:sync_qr_is_recovery', {
              defaultValue:
                'This is a recovery QR code. Please use it from the "Restore from another device" flow on the login page of a new device.',
            }),
          );
        } else {
          setError(t('common:sync_qr_unrecognized'));
        }
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
    setScannerError(null);
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
            {cameraCapability === 'unsupported' ? (
              /* 设备无摄像头：扫码位置显示提示，引导使用设备发现/手动输入 */
              <div
                style={{
                  display: 'flex',
                  flexDirection: 'column',
                  alignItems: 'center',
                  justifyContent: 'center',
                  gap: 10,
                  padding: '28px 16px',
                  borderRadius: 12,
                  border: '1px dashed var(--border-subtle)',
                  background: 'var(--bg-toolbar)',
                  textAlign: 'center',
                }}
              >
                <CameraOff size={28} color="var(--text-tertiary)" />
                <span
                  style={{
                    fontSize: 'var(--text-body-sm)',
                    color: 'var(--text-secondary)',
                    lineHeight: 1.5,
                  }}
                >
                  {t('common:sync_scan_unsupported', {
                    defaultValue:
                      'This device does not support QR scanning. Use device discovery or manual input instead.',
                  })}
                </span>
                <button
                  type="button"
                  onClick={handleClose}
                  className="interactive-outline"
                  style={{
                    marginTop: 4,
                    padding: '8px 16px',
                    borderRadius: 8,
                    borderWidth: 1,
                    borderStyle: 'solid',
                    background: 'var(--bg-elevated)',
                    color: 'var(--accent-primary)',
                    cursor: 'pointer',
                    fontFamily: 'inherit',
                    fontSize: 'var(--text-body-sm)',
                    fontWeight: 500,
                  }}
                >
                  {t('common:sync_use_manual', {
                    defaultValue: 'Use discovery or manual input',
                  })}
                </button>
              </div>
            ) : (
              <RecoveryQrScanner
                onScan={handleScan}
                onError={setScannerError}
                onCancel={handleClose}
              />
            )}

            {/* 扫码启动失败（权限被拒）时，提供关闭对话框、回页面使用发现/手动输入的兜底 */}
            {scannerError && cameraCapability !== 'unsupported' && (
              <button
                type="button"
                onClick={handleClose}
                className="interactive-outline"
                style={{
                  padding: '10px 12px',
                  borderRadius: 8,
                  borderWidth: 1,
                  borderStyle: 'solid',
                  background: 'var(--bg-toolbar)',
                  color: 'var(--accent-primary)',
                  cursor: 'pointer',
                  fontFamily: 'inherit',
                  fontSize: 'var(--text-body-sm)',
                  fontWeight: 500,
                }}
              >
                {t('common:sync_use_manual', {
                  defaultValue: 'Use discovery or manual input',
                })}
              </button>
            )}

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
