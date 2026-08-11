import { formatBytes } from '@/lib/utils';
import type { ReactNode } from 'react';

interface DownloadProgressBarProps {
  downloadedBytes: number;
  totalBytes: number;
  progressPercent: number;
  /** 自定义状态文本（如「正在安装…」）；缺省显示 已下载/总大小 (百分比)。 */
  statusText?: ReactNode;
}

/**
 * 下载进度条（P043：MandatoryUpdateOverlay 与 UpdateInfoCard 的下载进度区一致，
 * 收敛为共享组件——渐变进度条 + 等宽数字状态文本）。
 */
export function DownloadProgressBar({
  downloadedBytes,
  totalBytes,
  progressPercent,
  statusText,
}: DownloadProgressBarProps) {
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
      <div
        style={{
          width: '100%',
          height: 6,
          borderRadius: 3,
          background: 'var(--bg-toolbar)',
          overflow: 'hidden',
        }}
      >
        <div
          style={{
            width: `${progressPercent}%`,
            height: '100%',
            background: 'linear-gradient(90deg, var(--accent-primary), var(--accent-warm))',
            borderRadius: 3,
            transition: 'width 0.2s ease',
          }}
        />
      </div>
      <span
        style={{
          fontSize: 'var(--text-badge)',
          color: 'var(--text-tertiary)',
          textAlign: 'center',
          /* 数字等宽，避免下载字节数变化时文字宽度抖动 */
          fontVariantNumeric: 'tabular-nums',
        }}
      >
        {statusText ??
          `${formatBytes(downloadedBytes)} / ${formatBytes(totalBytes)} (${progressPercent}%)`}
      </span>
    </div>
  );
}
