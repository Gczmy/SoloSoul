import { Suspense } from 'react';
import { Outlet } from 'react-router-dom';
import { useShallow } from 'zustand/react/shallow';
import { AppShell } from './AppShell';
import { useShellConfigStore } from './shellConfigStore';
import { ContentLoadingSkeleton } from '@/components/ui/ContentLoadingSkeleton';

/**
 * 受保护路由的常驻壳布局（B1）：AppShell（侧边栏/顶栏/底部导航）挂在路由
 * Suspense 之外，切页不卸载；只有内容区（Outlet）随页面 chunk 懒加载，
 * 拉取期显示内容区骨架，不再整窗空白。
 *
 * 页面通过 <PageShell> 注册 title/actions/onBack（见 shellConfigStore）。
 */
export function ShellLayout() {
  const { title, actions, onBack } = useShellConfigStore(
    useShallow((s) => ({ title: s.title, actions: s.actions, onBack: s.onBack })),
  );
  return (
    <AppShell title={title} actions={actions} onBack={onBack}>
      <Suspense fallback={<ContentLoadingSkeleton />}>
        <Outlet />
      </Suspense>
    </AppShell>
  );
}
