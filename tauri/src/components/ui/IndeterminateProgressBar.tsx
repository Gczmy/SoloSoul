/**
 * 不确定进度条（加载中动画）。
 * 提取自外部存储选择（SAF 同步）的加载动画，供各加载场景复用。
 * 关键帧 `sync-progress-bar` 定义于全局 styles/animations.css，此处不重复注入。
 */
interface IndeterminateProgressBarProps {
  /** 轨道高度（px），默认 6 */
  height?: number;
  style?: React.CSSProperties;
}

export function IndeterminateProgressBar({ height = 6, style }: IndeterminateProgressBarProps) {
  return (
    <div
      style={{
        width: '100%',
        height,
        borderRadius: height / 2,
        background: 'var(--border-subtle)',
        overflow: 'hidden',
        ...style,
      }}
    >
      <div
        style={{
          width: '30%',
          height: '100%',
          borderRadius: height / 2,
          background: 'linear-gradient(90deg, var(--accent-primary), var(--accent-warm))',
          animation: 'sync-progress-bar 1.5s ease-in-out infinite',
          willChange: 'transform',
        }}
      />
    </div>
  );
}
