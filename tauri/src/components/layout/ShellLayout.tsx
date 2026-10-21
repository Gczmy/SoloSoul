import { Suspense } from 'react';
import { matchPath, Outlet, useLocation } from 'react-router-dom';
import { useShallow } from 'zustand/react/shallow';
import { AppShell } from './AppShell';
import { EMPTY_SHELL_CONFIG, shellRouteIdentity, useShellConfigStore } from './shellConfigStore';
import { ContentLoadingSkeleton } from '@/components/ui/ContentLoadingSkeleton';

/**
 * 受保护路由的常驻壳布局：AppShell 位于内容区 Suspense 之外，正常导航不卸载。
 * PageShell 注册当前路由的配置；新页面尚未注册或渲染失败时顶栏采用明确空态。
 */
export function ShellLayout() {
  const location = useLocation();
  const route = shellRouteIdentity(location);
  const { title, actions, primaryActions, onBack, objectInspector, continuity, handoff } =
    useShellConfigStore(
      useShallow((s) => {
        const continuity = Boolean(s.registration?.continuityScope);
        const exact = s.registration?.route === route;
        const handoff =
          !exact &&
          continuity &&
          Boolean(matchPath(s.registration!.continuityScope!, location.pathname));
        return s.registration?.isCurrent() && (exact || handoff)
          ? {
              title: s.title,
              actions: s.actions,
              primaryActions: s.primaryActions,
              onBack: s.onBack,
              objectInspector: s.objectInspector,
              continuity,
              handoff,
            }
          : { ...EMPTY_SHELL_CONFIG, continuity: false, handoff: false };
      }),
    );
  return (
    <AppShell
      title={title}
      primaryActions={
        continuity && primaryActions ? (
          <div style={{ display: 'contents' }} inert={handoff}>
            {primaryActions}
          </div>
        ) : (
          primaryActions
        )
      }
      actions={
        continuity && actions ? (
          <div style={{ display: 'contents' }} inert={handoff}>
            {actions}
          </div>
        ) : (
          actions
        )
      }
      onBack={onBack}
      objectInspector={
        objectInspector && handoff
          ? { ...objectInspector, ready: false, failed: false, objectName: '' }
          : objectInspector
      }
    >
      <Suspense fallback={<ContentLoadingSkeleton />}>
        <Outlet />
      </Suspense>
    </AppShell>
  );
}
