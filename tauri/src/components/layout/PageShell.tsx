import { useLayoutEffect, type ReactNode } from 'react';
import { useShellConfigStore } from './shellConfigStore';

export interface PageShellProps {
  children: ReactNode;
  title: string;
  actions?: ReactNode;
  onBack?: () => void;
}

/**
 * 页面壳配置桥（B1）：替代页面内 `<AppShell>` 包装。
 * title/actions/onBack 经 store 注册给常驻 <ShellLayout>（壳在 Suspense 之外），
 * children 渲染进壳的内容区。切页时壳不卸载，仅内容区等待新页面 chunk。
 *
 * 与旧 <AppShell> 的 props 形状完全一致，页面迁移仅需替换包装组件名。
 * useLayoutEffect 保证标题/操作在浏览器绘制前生效，不闪旧标题。
 */
export function PageShell({ children, title, actions, onBack }: PageShellProps) {
  const setConfig = useShellConfigStore((s) => s.setConfig);
  useLayoutEffect(() => {
    setConfig({ title, actions, onBack });
  }, [setConfig, title, actions, onBack]);
  return <>{children}</>;
}
