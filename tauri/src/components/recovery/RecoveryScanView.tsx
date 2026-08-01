import { useTranslation } from 'react-i18next';
import { CameraOff } from 'lucide-react';
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

      {cameraCapability === 'unsupported' ? (
        /* 设备无摄像头：扫码位置显示提示，引导使用手动输入 */
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
            {t('common:recovery_scan_unsupported', {
              defaultValue: 'This device does not support QR scanning. Please use manual input mode.',
            })}
          </span>
          <button
            type="button"
            onClick={onSwitchManual}
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
            {t('common:recovery_manual_tab', { defaultValue: 'Manual' })}
          </button>
        </div>
      ) : (
        <RecoveryQrScanner onScan={onScan} onError={onScannerError} />
      )}

      {/* 扫码启动失败（权限被拒/无摄像头）时，提供手动输入的兜底入口 */}
      {scannerError && cameraCapability !== 'unsupported' && (
        <button
          type="button"
          onClick={onSwitchManual}
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
          {t('common:recovery_use_manual', {
            defaultValue: 'Use manual input mode',
          })}
        </button>
      )}

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
