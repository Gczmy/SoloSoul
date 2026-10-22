import { Suspense } from 'react';
import { matchPath, Outlet, useLocation, useMatch } from 'react-router-dom';
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
  const inspectorTargetId = useMatch('/editor/:objectId')?.params.objectId;
  const route = shellRouteIdentity(location);
  const {
    title,
    actions,
    primaryActions,
    windowsContentToolbar,
    onBack,
    objectInspector,
    continuity,
    handoff,
  } = useShellConfigStore(
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
            windowsContentToolbar: s.windowsContentToolbar,
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
      windowsContentToolbar={windowsContentToolbar}
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
        // 导航 key 更新不等于对象变化；同一对象不能因此卸载历史和附件会话。
        objectInspector && handoff && objectInspector.objectId !== inspectorTargetId
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
