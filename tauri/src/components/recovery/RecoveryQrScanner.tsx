import { useCallback, useEffect, useId, useRef, useState } from 'react';
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
  // '' 表示未指定具体设备：以 facingMode:'environment' 启动，移动端默认优先后置摄像头
  const [selectedCamera, setSelectedCamera] = useState<string>('');
  // 是否直接以默认摄像头（facingMode: environment）启动：
  // - 枚举失败/为空时置 true（无设备信息可用，直接尝试默认摄像头）
  // - 枚举成功但用户未选择具体设备时也置 true（移动端默认优先后置）
  const [useDefaultCamera, setUseDefaultCamera] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // 回调引用：父组件每次渲染都会产生新的 onScan/onError 引用，
  // 若直接作为 effect 依赖会导致扫描器反复卸载/重启（并触发 stop() 崩溃路径）。
  const onScanRef = useRef(onScan);
  onScanRef.current = onScan;
  const onErrorRef = useRef(onError);
  onErrorRef.current = onError;

  // 将底层错误映射为友好的提示，区分「权限被拒」与「无摄像头设备」两种场景。
  const friendlyError = useCallback((raw: string): string => {
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
  }, [t]);

  useEffect(() => {
    let isMounted = true;

    Html5Qrcode.getCameras()
      .then((devices) => {
        if (!isMounted) return;
        if (devices.length === 0) {
          // 枚举为空：尝试默认摄像头启动
          setUseDefaultCamera(true);
          setLoading(false);
          return;
        }
        setCameras(devices);
        // 不自动选中具体设备：以 facingMode:'environment' 启动，移动端浏览器
        // 会自动优先使用后置摄像头。注意 html5-qrcode 的 getCameras() 返回的
        // 设备对象只有 id/label（无 facing 字段），无法从枚举结果判断前后置，
        // 因此交给浏览器按 facingMode 约束选择最合适的摄像头。
        setSelectedCamera('');
        setUseDefaultCamera(true);
        setLoading(false);
      })
      .catch(() => {
        if (!isMounted) return;
        // 枚举失败：先尝试默认摄像头启动，不直接阻断
        setUseDefaultCamera(true);
        setLoading(false);
      });

    return () => {
      isMounted = false;
    };
  }, [t]);

  useEffect(() => {
    if (!selectedCamera && !useDefaultCamera) {
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

    try {
      scanner
        .start(
          cameraIdOrConfig,
          { fps: 10, qrbox: { width: 250, height: 250 } },
          (decodedText) => {
            onScanRef.current(decodedText);
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
          onErrorRef.current?.(msg);
        });
    } catch (err) {
      // start() 也可能同步 throw（例如容器不可用），同样需要保护
      setLoading(false);
      const msg = String(err);
      setError(friendlyError(msg));
      onErrorRef.current?.(msg);
    }

    return () => {
      // html5-qrcode 的 stop() 在扫描器从未成功启动时是同步 throw
      // （throw "Cannot stop, scanner is not running or paused."，而非 promise
      // rejection），.catch 接不住同步异常。若不 try/catch 包裹，异常会逃逸出
      // React 的 effect cleanup，导致整棵组件树崩溃（页面消失）。
      try {
        scanner
          .stop()
          .then(() => scanner.clear())
          .catch(() => {});
      } catch {
        // 扫描器未启动（如权限被拒后直接切换），stop() 同步抛错，忽略即可
      }
    };
  }, [selectedCamera, useDefaultCamera, containerId, friendlyError, t]);

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
          <option value="">{t('common:recovery_qr_default_camera')}</option>
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
