import type { ReactNode } from 'react';
import { CameraOff } from 'lucide-react';
import type { CameraCapability } from '@/lib/cameraCapability';

interface QrScanFallbackProps {
  cameraCapability: CameraCapability;
  scannerError: string | null;
  /** 无摄像头占位文案（dashed 框内） */
  unsupportedText: ReactNode;
  /** 无摄像头占位按钮文案 */
  unsupportedButtonLabel: ReactNode;
  /** 扫码启动失败（权限被拒等）时兜底按钮文案 */
  scannerErrorButtonLabel: ReactNode;
  /** 兜底动作：切换手动输入 / 关闭对话框等 */
  onAction: () => void;
  /** 摄像头可用时渲染的扫码器 */
  children: ReactNode;
}

/**
 * P226: 扫码位统一兜底——无摄像头占位 + 扫码启动失败兜底按钮。
 *
 * 收敛自 RecoveryScanView 与 SyncScanQrDialog 两处逐字节相同的两块：
 * ① 设备无摄像头（cameraCapability === 'unsupported'）时的虚线占位框（CameraOff 图标 +
 *    提示文案 + 手动输入按钮）；② 扫码器启动失败（权限被拒/无设备）时的兜底按钮。
 * 两者样式完全一致，仅文案与动作 handler 不同，以 props 参数化。
 */
export function QrScanFallback({
  cameraCapability,
  scannerError,
  unsupportedText,
  unsupportedButtonLabel,
  scannerErrorButtonLabel,
  onAction,
  children,
}: QrScanFallbackProps) {
  return (
    <>
      {cameraCapability === 'unsupported' ? (
        /* 设备无摄像头：扫码位置显示提示，引导使用手动输入/设备发现 */
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
            {unsupportedText}
          </span>
          <button
            type="button"
            onClick={onAction}
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
            {unsupportedButtonLabel}
          </button>
        </div>
      ) : (
        children
      )}

      {/* 扫码启动失败（权限被拒/无摄像头）时，提供兜底入口 */}
      {scannerError && cameraCapability !== 'unsupported' && (
        <button
          type="button"
          onClick={onAction}
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
          {scannerErrorButtonLabel}
        </button>
      )}
    </>
  );
}
