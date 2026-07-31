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
  // 部分 WebView（如 macOS WKWebView 未授予权限或枚举受限）中 getCameras()
  // 会失败或返回空列表。此时回退到浏览器默认摄像头（facingMode: environment）
  // 直接启动，避免用户看到 "Unable to query supported devices" 而无法使用。
  const [fallbackStart, setFallbackStart] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // 将底层错误映射为友好的提示，区分「权限被拒」与「无摄像头设备」两种场景。
  const friendlyError = (raw: string): string => {
    const lower = raw.toLowerCase();
    if (
      lower.includes('notallowed') ||
      lower.includes('permission') ||
      lower.includes('denied') ||
      lower.includes('notreadable') ||
      lower.includes('inuse') ||
      lower.includes('unable to query')
    ) {
      // 权限被阻止/摄像头不可用（NotAllowedError / NotReadableError 等）
      return t('common:recovery_qr_permission_denied');
    }
    if (
      lower.includes('notfound') ||
      lower.includes('no camera') ||
      lower.includes('no device') ||
      lower.includes('no video') ||
      lower.includes('no supported devices')
    ) {
      // 未检测到摄像头设备（NotFoundError / 枚举为空等）
      return t('common:recovery_qr_no_camera');
    }
    return raw;
  };

  useEffect(() => {
    let isMounted = true;

    Html5Qrcode.getCameras()
      .then((devices) => {
        if (!isMounted) return;
        if (devices.length === 0) {
          // 枚举为空：尝试默认摄像头启动
          setFallbackStart(true);
          setLoading(false);
          return;
        }
        setCameras(devices);
        setSelectedCamera(devices[0].id);
        setLoading(false);
      })
      .catch((err) => {
        if (!isMounted) return;
        // 枚举失败：先尝试默认摄像头启动，不直接阻断
        setFallbackStart(true);
        setLoading(false);
      });

    return () => {
      isMounted = false;
    };
  }, [t]);

  useEffect(() => {
    if (!selectedCamera && !fallbackStart) {
      setLoading(false);
      return;
    }

    const scanner = new Html5Qrcode(containerId);
    scannerRef.current = scanner;

    setLoading(true);
    setError(null);

    // 有具体设备 ID 时用该设备；否则用浏览器默认摄像头（environment 优先）
    const cameraIdOrConfig: string | MediaTrackConstraints =
      selectedCamera || { facingMode: 'environment' };

    scanner
      .start(
        cameraIdOrConfig,
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
        const msg = String(err);
        setError(friendlyError(msg));
        onError?.(msg);
      });

    return () => {
      scanner
        .stop()
        .then(() => scanner.clear())
        .catch(() => {});
    };
  }, [onScan, onError, selectedCamera, fallbackStart, containerId, t]);

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

      {!error && (
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
      )}

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
