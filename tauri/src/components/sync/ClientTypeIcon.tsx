import { Monitor, Terminal, Smartphone, Tablet, Cpu, Laptop } from 'lucide-react';

/**
 * 客户端类型 → 图标组件（设备同步卡片统一使用）。
 *
 * - macOS → 笔记本（Laptop）：用户明确要求不用 Apple 商标图标，用笔记本图标表达
 *   macOS 设备（MacBook）；Windows 桌面 → Monitor；Linux → Terminal；Android 手机 →
 *   Smartphone；iOS → Tablet；未知 → Cpu。
 *
 * 已知设备卡片、详情弹窗、配对对话框共用，保证设备图标与客户端类型一一对应。
 */
export function ClientTypeIcon({ clientType, size }: { clientType?: string; size: number }) {
  const color = 'var(--accent-primary)';
  switch (clientType) {
    case 'macos':
      return <Laptop size={size} color={color} />;
    case 'windows':
      return <Monitor size={size} color={color} />;
    case 'linux':
      return <Terminal size={size} color={color} />;
    case 'android':
      return <Smartphone size={size} color={color} />;
    case 'ios':
      return <Tablet size={size} color={color} />;
    default:
      return <Cpu size={size} color={color} />;
  }
}
