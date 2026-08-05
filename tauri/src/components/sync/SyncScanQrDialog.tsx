import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { motion } from 'framer-motion';
import { X } from 'lucide-react';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { useCameraCapability } from '@/hooks/useCameraCapability';
import { QrScanFallback } from '@/components/recovery/QrScanFallback';
import { RecoveryQrScanner } from '@/components/recovery/RecoveryQrScanner';
import { logger } from '@/lib/logger';

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
        return;
      }
      setError(null);
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
    setScannerError(null);
    onClose();
  };

  const handleConfirmSync = () => {
    if (!scanned || scanned.type !== 'sync' || !onSync) return;
    const { addr, fingerprint } = scanned;
    // 后台执行：立即关闭扫码对话框，不再阻塞等待完整同步（期间用户可自由操作）。
    // 结果反馈由页面侧负责：同步成功 → toast「同步完成」；配对中 → PairingDialog
    // 双向确认；失败 → 页面错误横幅（resolveBackendErrorMessage）。
    void Promise.resolve(onSync(addr, fingerprint || '')).catch((err) => {
      logger.warn('[SyncScanQrDialog] background sync failed:', err);
    });
    handleClose();
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
      {/* 卡片进场淡入：消除手写模态的硬弹出闪烁（与 SyncShowQrDialog / 共享 Dialog 模式对齐） */}
      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ duration: 0.2 }}
        style={{ width: '100%', maxWidth: 420 }}
      >
        <Card
          style={{
            // 宽度由外层 motion.div（width:100% + maxWidth:420）约束，这里只填满，避免双重 maxWidth
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
              <QrScanFallback
                cameraCapability={cameraCapability}
                scannerError={scannerError}
                unsupportedText={t('common:sync_scan_unsupported', {
                  defaultValue:
                    'This device does not support QR scanning. Use device discovery or manual input instead.',
                })}
                unsupportedButtonLabel={t('common:sync_use_manual', {
                  defaultValue: 'Use discovery or manual input',
                })}
                scannerErrorButtonLabel={t('common:sync_use_manual', {
                  defaultValue: 'Use discovery or manual input',
                })}
                onAction={handleClose}
              >
                <RecoveryQrScanner
                  onScan={handleScan}
                  onError={setScannerError}
                  onCancel={handleClose}
                />
              </QrScanFallback>

              {error && (
                <div
                  style={{ color: '#e74c3c', fontSize: 'var(--text-body-sm)', textAlign: 'center' }}
                >
                  {error}
                </div>
              )}
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
                {t('common:sync_qr_confirm_sync', {
                  deviceName: scanned.deviceName || scanned.addr,
                })}
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
              <Button onClick={handleConfirmSync} style={{ width: '100%' }}>
                {t('common:sync_qr_confirm_sync_button')}
              </Button>
              {error && (
                <div
                  style={{ color: '#e74c3c', fontSize: 'var(--text-body-sm)', textAlign: 'center' }}
                >
                  {error}
                </div>
              )}
              <Button variant="secondary" onClick={() => setScanned(null)} style={{ width: '100%' }}>
                {t('common:cancel')}
              </Button>
            </div>
          )}
        </Card>
      </motion.div>
    </div>
  );
}
