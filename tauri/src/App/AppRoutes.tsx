import { Suspense } from 'react';
import { Routes, Route, Navigate, useNavigate, useSearchParams } from 'react-router-dom';
import { useShallow } from 'zustand/react/shallow';
import { useAuthStore } from '@/stores/authStore';
import { ErrorBoundary } from '@/components/ui/ErrorBoundary';
import { RouteLoadingSkeleton } from '@/components/ui/RouteLoadingSkeleton';
import { ShellLayout } from '@/components/layout/ShellLayout';
import { AuthLayout } from '@/components/layout/AuthLayout';
import { BootstrapPage } from '@/pages/auth/BootstrapPage';
import { LoginPage } from '@/pages/auth/LoginPage';
import { protectedRoutes, AuthGuard } from './routes';
import { AppNotifications } from './AppNotifications';
import { useSessionLifecycle } from './useSessionLifecycle';
import { useNativeAppEvents } from './useNativeAppEvents';

export function AppRoutes() {
  const navigate = useNavigate();
  const { checkHasAccount, hasAccount, isAuthenticated, backendError, accountId } = useAuthStore(
    useShallow((s) => ({
      checkHasAccount: s.checkHasAccount,
      hasAccount: s.hasAccount,
      isAuthenticated: s.isAuthenticated,
      backendError: s.backendError,
      accountId: s.currentAccount?.id,
    })),
  );

  useNativeAppEvents({ navigate, isAuthenticated });
  useSessionLifecycle({
    navigate,
    checkHasAccount,
    hasAccount,
    backendError,
    isAuthenticated,
    accountId,
  });
  // 支持 /bootstrap?mode=create 在已有账户时仍能创建新账户
  const [searchParams] = useSearchParams();
  const bootstrapMode = searchParams.get('mode');

  return (
    <AppNotifications isAuthenticated={isAuthenticated}>
      {/* 方案 A 扩展：全部页面静态导入后无 lazy 组件，Suspense 边界保留（零触发）作为
          未来若重新引入懒加载时的结构位；B1 壳常驻布局保持不变。 */}
      <Suspense fallback={<RouteLoadingSkeleton />}>
        <Routes>
          <Route element={<AuthLayout />}>
            <Route
              path="/bootstrap"
              element={
                hasAccount === false || bootstrapMode === 'create' ? (
                  <BootstrapPage />
                ) : hasAccount === true ? (
                  <Navigate to="/login" replace />
                ) : (
                  <div
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      height: '100%',
                      color: 'var(--text-secondary)',
                      fontSize: 'var(--text-body)',
                    }}
                  >
                    Connecting to backend...
                  </div>
                )
              }
            />
            <Route
              path="/login"
              element={
                hasAccount === null ? (
                  <div
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      height: '100%',
                      color: 'var(--text-secondary)',
                      fontSize: 'var(--text-body)',
                    }}
                  >
                    Connecting...
                  </div>
                ) : (
                  <LoginPage />
                )
              }
            />
          </Route>
          {/* B1: 受保护路由统一挂在常驻壳布局下（AuthGuard 提升到布局层），
              切页仅内容区（Outlet）等待新页面 chunk，壳不卸载。 */}
          <Route
            element={
              <AuthGuard>
                <ShellLayout key={accountId} />
              </AuthGuard>
            }
          >
            {protectedRoutes.map((r) => (
              <Route
                key={r.path}
                path={r.path}
                element={<ErrorBoundary label={`route:${r.path}`}>{r.element}</ErrorBoundary>}
              />
            ))}
          </Route>
          <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>
      </Suspense>
    </AppNotifications>
  );
}
