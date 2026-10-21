import { useLayoutEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import { useLocation } from 'react-router-dom';
import { createSessionRequests } from '@/lib/sessionRequests';
import {
  shellRouteIdentity,
  useShellConfigStore,
  type ObjectInspectorContext,
} from './shellConfigStore';

export interface PageShellProps {
  children: ReactNode;
  title: string;
  actions?: ReactNode;
  primaryActions?: ReactNode;
  onBack?: () => void;
  continuityScope?: string;
  objectInspector?: ObjectInspectorContext;
}

/**
 * 页面壳配置桥：常驻 ShellLayout 在 Suspense 外，页面只注册顶栏并渲染内容。
 * 每次路由注册绑定页面 owner 与首次挂载的会话，旧清理不能删除新页面配置。
 * useLayoutEffect 保证标题/操作在浏览器绘制前生效，不闪旧标题。
 */
export function PageShell({
  children,
  title,
  actions,
  primaryActions,
  onBack,
  continuityScope,
  objectInspector,
}: PageShellProps) {
  const route = shellRouteIdentity(useLocation());
  const [owner] = useState(() => Symbol('PageShell'));
  const [session] = useState(() => createSessionRequests().begin());
  const registration = useMemo(
    () => ({
      owner,
      route,
      isCurrent: session.isCurrent,
      ...(continuityScope ? { continuityScope } : {}),
    }),
    [owner, route, session, continuityScope],
  );
  const register = useShellConfigStore((s) => s.register);
  const unregister = useShellConfigStore((s) => s.unregister);

  // 配置更新不触发注销；StrictMode 的清理也不能使整页原会话票据永久失效。
  // 路由参数更新是同一 owner 的交接，不先注销成空态；真正卸载仍注销当前注册。
  const latestRegistration = useRef(registration);
  useLayoutEffect(() => {
    latestRegistration.current = registration;
  }, [registration]);
  useLayoutEffect(() => () => unregister(latestRegistration.current), [unregister]);
  useLayoutEffect(() => {
    register(registration, { title, actions, primaryActions, onBack, objectInspector });
  }, [register, registration, title, actions, primaryActions, onBack, objectInspector]);
  return <>{children}</>;
}
