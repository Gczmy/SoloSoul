import { useLayoutEffect, useMemo, useState, type ReactNode } from 'react';
import { useLocation } from 'react-router-dom';
import { createSessionRequests } from '@/lib/sessionRequests';
import { shellRouteIdentity, useShellConfigStore } from './shellConfigStore';

export interface PageShellProps {
  children: ReactNode;
  title: string;
  actions?: ReactNode;
  primaryActions?: ReactNode;
  onBack?: () => void;
}

/**
 * 页面壳配置桥：常驻 ShellLayout 在 Suspense 外，页面只注册顶栏并渲染内容。
 * 每次路由注册绑定页面 owner 与首次挂载的会话，旧清理不能删除新页面配置。
 * useLayoutEffect 保证标题/操作在浏览器绘制前生效，不闪旧标题。
 */
export function PageShell({ children, title, actions, primaryActions, onBack }: PageShellProps) {
  const route = shellRouteIdentity(useLocation());
  const [owner] = useState(() => Symbol('PageShell'));
  const [session] = useState(() => createSessionRequests().begin());
  const registration = useMemo(
    () => ({ owner, route, isCurrent: session.isCurrent }),
    [owner, route, session],
  );
  const register = useShellConfigStore((s) => s.register);
  const unregister = useShellConfigStore((s) => s.unregister);

  // 配置更新不触发注销；StrictMode 的清理也不能使整页原会话票据永久失效。
  useLayoutEffect(() => () => unregister(registration), [unregister, registration]);
  useLayoutEffect(() => {
    register(registration, { title, actions, primaryActions, onBack });
  }, [register, registration, title, actions, primaryActions, onBack]);
  return <>{children}</>;
}
