import { useTranslation } from 'react-i18next';
import { QrScanFallback } from '@/components/recovery/QrScanFallback';
import { RecoveryQrScanner } from '@/components/recovery/RecoveryQrScanner';
import type { CameraCapability } from '@/lib/cameraCapability';

interface RecoveryScanViewProps {
  cameraCapability: CameraCapability;
  scannerError: string | null;
  error: string | null;
  onScan: (text: string) => void;
  onScannerError: (msg: string) => void;
  onSwitchManual: () => void;
}

/** 扫码 tab：新设备摄像头扫描旧设备恢复二维码。 */
export function RecoveryScanView({
  cameraCapability,
  scannerError,
  error,
  onScan,
  onScannerError,
  onSwitchManual,
}: RecoveryScanViewProps) {
  const { t } = useTranslation(['common']);

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
      <p
        style={{
          fontSize: 'var(--text-body-sm)',
          color: 'var(--text-secondary)',
          margin: '0 0 4px',
          lineHeight: 1.5,
        }}
      >
        {t('common:recovery_receive_scan_desc', {
          defaultValue:
            'On your old device, go to Settings → Device Sync → Show Recovery QR, then scan it with this camera.',
        })}
      </p>

      <QrScanFallback
        cameraCapability={cameraCapability}
        scannerError={scannerError}
        unsupportedText={t('common:recovery_scan_unsupported', {
          defaultValue:
            'This device does not support QR scanning. Please use manual input mode.',
        })}
        unsupportedButtonLabel={t('common:recovery_manual_tab', { defaultValue: 'Manual' })}
        scannerErrorButtonLabel={t('common:recovery_use_manual', {
          defaultValue: 'Use manual input mode',
        })}
        onAction={onSwitchManual}
      >
        <RecoveryQrScanner onScan={onScan} onError={onScannerError} />
      </QrScanFallback>

      {error && (
        <div
          style={{
            color: '#e74c3c',
            fontSize: 'var(--text-body-sm)',
            textAlign: 'center',
          }}
        >
          {error}
        </div>
      )}
    </div>
  );
}
