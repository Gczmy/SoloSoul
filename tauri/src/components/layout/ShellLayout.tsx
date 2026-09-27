import { Suspense } from 'react';
import { Outlet, useLocation } from 'react-router-dom';
import { useShallow } from 'zustand/react/shallow';
import { AppShell } from './AppShell';
import { EMPTY_SHELL_CONFIG, shellRouteIdentity, useShellConfigStore } from './shellConfigStore';
import { ContentLoadingSkeleton } from '@/components/ui/ContentLoadingSkeleton';

/**
 * 受保护路由的常驻壳布局：AppShell 位于内容区 Suspense 之外，正常导航不卸载。
 * PageShell 注册当前路由的配置；新页面尚未注册或渲染失败时顶栏采用明确空态。
 */
export function ShellLayout() {
  const route = shellRouteIdentity(useLocation());
  const { title, actions, primaryActions, onBack } = useShellConfigStore(
    useShallow((s) =>
      s.registration?.route === route && s.registration.isCurrent()
        ? {
            title: s.title,
            actions: s.actions,
            primaryActions: s.primaryActions,
            onBack: s.onBack,
          }
        : EMPTY_SHELL_CONFIG,
    ),
  );
  return (
    <AppShell title={title} primaryActions={primaryActions} actions={actions} onBack={onBack}>
      <Suspense fallback={<ContentLoadingSkeleton />}>
        <Outlet />
      </Suspense>
    </AppShell>
  );
}
