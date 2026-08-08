import type { ReactNode } from 'react';
import { ClientTypeIcon } from '@/components/sync/ClientTypeIcon';
import { ICON_SIZE } from '@/lib/constants';

interface DeviceCardShellProps {
  clientType?: string;
  /** 设备名（已裁剪）。 */
  name: string;
  /** 副标题区域（地址 / 信任徽章 + 在线状态…）。 */
  subtitle?: ReactNode;
  /** 右侧操作区（同步按钮 / 信任-忘记按钮组…）。 */
  actions?: ReactNode;
  onOpen: () => void;
}

/**
 * P012: 设备卡片共享外壳——已发现设备与已知设备两张卡片此前重复约 40 行
 * （交互容器 interactive-card-lift + 键盘可访问性 + 客户端类型图标 + 名称行）。
 * 副标题与操作区由调用方注入，保持两张卡片各自的语义差异。
 */
export function DeviceCardShell({
  clientType,
  name,
  subtitle,
  actions,
  onOpen,
}: DeviceCardShellProps) {
  return (
    <div
      role="button"
      tabIndex={0}
      onClick={onOpen}
      onKeyDown={(e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          onOpen();
        }
      }}
      className="interactive-card-lift"
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 10,
        padding: '10px 12px',
        borderRadius: 8,
        background: 'var(--bg-toolbar)',
        // 细线边框：与设备卡片同款，区分卡片与页面背景
        border: '1px solid var(--border-subtle)',
        cursor: 'pointer',
      }}
    >
      {/* 客户端类型图标（macos 笔记本 / android 手机…），与两张卡片同源 */}
      <ClientTypeIcon clientType={clientType} size={ICON_SIZE.lg} />
      <div style={{ flex: 1, minWidth: 0 }}>
        {/* 设备名：ellipsis 兜底防安卓端溢出 */}
        <div
          style={{
            fontSize: 'var(--text-body-sm)',
            fontWeight: 500,
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
          }}
        >
          {name}
        </div>
        {subtitle}
      </div>
      {actions && <div style={{ display: 'flex', gap: 6, flexShrink: 0 }}>{actions}</div>}
    </div>
  );
}
