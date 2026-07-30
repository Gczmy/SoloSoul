import { useEffect, useId, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Html5Qrcode, type CameraDevice } from 'html5-qrcode';
import { Loader2 } from 'lucide-react';

interface RecoveryQrScannerProps {
  onScan: (text: string) => void;
  onError?: (message: string) => void;
  onCancel?: () => void;
}

export function RecoveryQrScanner({ onScan, onError, onCancel }: RecoveryQrScannerProps) {
  const { t } = useTranslation(['common']);
  const scannerId = useId();
  const containerId = `recovery-qr-video-${scannerId}`;
  const scannerRef = useRef<Html5Qrcode | null>(null);
  const [cameras, setCameras] = useState<CameraDevice[]>([]);
  const [selectedCamera, setSelectedCamera] = useState<string>('');
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let isMounted = true;

    Html5Qrcode.getCameras()
      .then((devices) => {
        if (!isMounted) return;
        if (devices.length === 0) {
          setError(t('common:recovery_qr_no_camera'));
          setLoading(false);
          return;
        }
        setCameras(devices);
        setSelectedCamera(devices[0].id);
        setLoading(false);
      })
      .catch((err) => {
        if (!isMounted) return;
        setError(String(err));
        setLoading(false);
        onError?.(String(err));
      });

    return () => {
      isMounted = false;
    };
  }, [onError, t]);

  useEffect(() => {
    if (!selectedCamera) {
      setLoading(false);
      return;
    }

    const scanner = new Html5Qrcode(containerId);
    scannerRef.current = scanner;

    setLoading(true);
    setError(null);

    scanner
      .start(
        selectedCamera,
        { fps: 10, qrbox: { width: 250, height: 250 } },
        (decodedText) => {
          onScan(decodedText);
        },
        () => {
          // 每一帧都可能触发但无二维码，忽略
        },
      )
      .then(() => {
        setLoading(false);
      })
      .catch((err) => {
        setLoading(false);
        setError(String(err));
        onError?.(String(err));
      });

    return () => {
      scanner
        .stop()
        .then(() => scanner.clear())
        .catch(() => {});
    };
  }, [onScan, onError, selectedCamera, containerId]);

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 12, width: '100%' }}>
      {cameras.length > 1 && (
        <select
          value={selectedCamera}
          onChange={(e) => setSelectedCamera(e.target.value)}
          style={{
            width: '100%',
            padding: '10px 14px',
            borderRadius: 8,
            border: '1px solid var(--border-subtle)',
            background: 'var(--bg-elevated)',
            color: 'var(--text-primary)',
            fontSize: 'var(--text-body)',
            fontFamily: 'inherit',
          }}
        >
          {cameras.map((camera) => (
            <option key={camera.id} value={camera.id}>
              {camera.label}
            </option>
          ))}
        </select>
      )}

      <div
        style={{
          position: 'relative',
          width: '100%',
          aspectRatio: '1 / 1',
          borderRadius: 12,
          overflow: 'hidden',
          border: '1px solid var(--border-subtle)',
          background: 'var(--bg-toolbar)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
        }}
      >
        <div id={containerId} style={{ width: '100%', height: '100%' }} />

        {loading && (
          <div
            style={{
              position: 'absolute',
              inset: 0,
              display: 'flex',
              flexDirection: 'column',
              alignItems: 'center',
              justifyContent: 'center',
              gap: 8,
              background: 'var(--bg-overlay)',
            }}
          >
            <Loader2 size={24} style={{ animation: 'spin 1s linear infinite' }} />
            <span style={{ fontSize: 'var(--text-body-sm)', color: 'var(--text-secondary)' }}>
              {t('common:loading')}
            </span>
          </div>
        )}
      </div>

      {error && (
        <div style={{ color: '#e74c3c', fontSize: 'var(--text-body-sm)', textAlign: 'center' }}>
          {error}
        </div>
      )}

      <p
        style={{
          fontSize: 'var(--text-body-sm)',
          color: 'var(--text-secondary)',
          textAlign: 'center',
          margin: 0,
          lineHeight: 1.5,
        }}
      >
        {t('common:recovery_qr_hint')}
      </p>

      {onCancel && (
        <button
          type="button"
          onClick={onCancel}
          style={{
            width: '100%',
            padding: '10px 12px',
            borderRadius: 8,
            border: '1px solid var(--border-subtle)',
            background: 'var(--bg-toolbar)',
            color: 'var(--text-primary)',
            cursor: 'pointer',
            fontFamily: 'inherit',
            fontSize: 'var(--text-body-sm)',
            transition: 'all 0.15s ease',
          }}
          onMouseEnter={(e) => {
            e.currentTarget.style.background =
            'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
            e.currentTarget.style.borderColor = 'var(--accent-primary)';
            e.currentTarget.style.color = 'var(--accent-primary)';
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.background = 'var(--bg-toolbar)';
            e.currentTarget.style.borderColor = 'var(--border-subtle)';
            e.currentTarget.style.color = 'var(--text-primary)';
          }}
        >
          {t('common:cancel')}
        </button>
      )}
    </div>
  );
}
