import { Suspense, lazy, useEffect } from 'react';
import { Routes, Route, Navigate, useNavigate, useSearchParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { useShallow } from 'zustand/react/shallow';
import { listen } from '@tauri-apps/api/event';
import { useAuthStore } from '@/stores/authStore';
import { useObjectStore } from '@/stores/objectStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { useProfileStore } from '@/stores/profileStore';
import { useTrashStore } from '@/stores/trashStore';
import { useOcrScanStore } from '@/stores/ocrScanStore';
import { useLlmStore } from '@/stores/llmStore';
import { useApplyThemeFromSettings } from '@/hooks/useApplyThemeFromSettings';
import { useAutoLock } from '@/hooks/useAutoLock';
import { useAppUpdate } from '@/hooks/useAppUpdate';
import { useOcrFirstInstall } from '@/hooks/useOcrFirstInstall';
import { initLlmNotificationListener } from '@/lib/notification';
import { searchCache } from '@/lib/searchCache';
import { applyTheme, getSystemTheme, listenForSystemTheme } from '@/lib/theme';
import { confirmWithPause } from '@/lib/dialog';
import { UpdateBanner, type UpdateBannerState } from '@/components/ui/UpdateBanner';
import { OcrInstallBanner } from '@/components/ui/OcrInstallBanner';
import { ST_SKIPPED_VERSION, SAFE_AREA_TOP } from '@/lib/constants';
import { logger } from '@/lib/logger';
import { setGlobalNavigate } from '@/lib/navigation';
import { useSafSyncStore } from '@/stores/safSyncStore';
import { useUiStore } from '@/stores/uiStore';
import { SafSyncIndicator } from '@/components/sync/SafSyncIndicator';
import { PostLoginSetupGuide } from '@/components/guide/PostLoginSetupGuide';
import { protectedRoutes, AuthGuard, routeLoaders } from './routes';
import { RouteLoadingSkeleton } from '@/components/ui/RouteLoadingSkeleton';
// P015: 认证页同样懒加载（首屏只加载当前需要的 chunk）
const loadBootstrapPage = () => import('@/pages/auth/BootstrapPage');
const BootstrapPage = lazy(() => loadBootstrapPage().then((m) => ({ default: m.BootstrapPage })));
const loadLoginPage = () => import('@/pages/auth/LoginPage');
const LoginPage = lazy(() => loadLoginPage().then((m) => ({ default: m.LoginPage })));

export function AppRoutes() {
  const navigate = useNavigate();
  useEffect(() => {
    setGlobalNavigate(navigate);
    return () => {
      setGlobalNavigate(null);
    };
  }, [navigate]);
  const { t } = useTranslation(['settings']);
  // P022: useShallow 字段级选择——避免 store 任意字段（error/backendError 等）翻转时整页重渲染
  const { checkHasAccount, hasAccount, isAuthenticated } = useAuthStore(
    useShallow((s) => ({
      checkHasAccount: s.checkHasAccount,
      hasAccount: s.hasAccount,
      isAuthenticated: s.isAuthenticated,
    })),
  );
  // P041: 统一更新状态机（桌面 plugin-updater + Android GitHub Release）与 OCR 首装逻辑
  // 已各自拆入 useAppUpdate / useOcrFirstInstall。
  const { updateState, startDownload, installUpdate, dismissUpdate } = useAppUpdate();
  const { showOcrBanner, ocrPhase, progress, error, retryOcrInstall, closeOcrBanner } =
    useOcrFirstInstall();

  useEffect(() => {
    checkHasAccount();
  }, [checkHasAccount]);

  // Check SAF vault directory validity after login
  useEffect(() => {
    if (!isAuthenticated) return;
    const checkVaultDir = async () => {
      try {
        const { checkVaultDirectory } = await import('@/lib/vaultDirectory');
        const valid = await checkVaultDirectory();
        if (valid) {
          // 目录已恢复（用户重新选择了 SAF 目录并迁移成功）→
          // 清除授权失效常驻横幅状态与 toast 去重标志，避免横幅一直悬挂。
          useUiStore.getState().setSafAuthRevoked(false);
          useUiStore.getState().setSafAuthToastShown(false);
          useUiStore.getState().setSafSyncError(null);
          useUiStore.getState().setSafSyncState('idle');
          return;
        }
        // SAF 目录失效（用户手动删除了外部目录）：
        // 1. 立即置位授权失效常驻横幅（每次启动登录后都会重新检测，因此每次都会提醒）；
        // 2. 按会话去重弹出提示 toast（与 saf-auth-revoked 事件处理一致，避免多次弹）；
        // 3. 弹确认对话框引导用户前往数据管理重新选择目录。
        logger.warn('[AppRoutes] SAF vault directory access revoked');
        const ui = useUiStore.getState();
        ui.setSafAuthRevoked(true);
        if (!ui.safAuthToastShown) {
          ui.setSafAuthToastShown(true);
          ui.showToast({
            type: 'warning',
            message: t(
              'settings:vault_directory_invalid_toast',
              'SAF directory access revoked. Go to Settings > Data Management to re-select.',
            ),
            duration: 10000,
          });
        }
        await confirmWithPause(
          t(
            'settings:vault_directory_invalid_message',
            '您之前使用的外部存储目录已被删除或无法访问。\n\nSoloSoul 已将您的数据保留在本地应用存储中，您可以继续正常使用。\n\n如需重新选择外部目录，请前往「设置 > 数据管理」。',
          ),
          {
            title: t('settings:vault_directory_invalid_title', '存储目录不可用'),
            kind: 'warning',
          },
        );
      } catch {
        // Silently ignore if not on Android or dialog not supported
      }
    };
    checkVaultDir();
  }, [isAuthenticated, t]);

  // Load settings and profile after authentication
  useEffect(() => {
    const account = useAuthStore.getState().currentAccount;
    if (isAuthenticated && account) {
      useProfileStore.getState().loadProfile(account.id);
      useSettingsStore
        .getState()
        .loadSettings(account.id)
        .then(async () => {
          // Re-apply theme with loaded settings (otherwise stays at defaults)
          const s = useSettingsStore.getState().settings;
          const resolvedSystemTheme = s.theme === 'system' ? await getSystemTheme() : undefined;
          await applyTheme({
            preset:
              s.theme === 'dark'
                ? 'warm-stone-dark'
                : s.theme === 'light'
                  ? 'warm-stone-light'
                  : 'system',
            accentColor: s.accentColor,
            backgroundType: s.backgroundType,
            backgroundValue: s.backgroundValue,
            defaultLightTheme: s.defaultLightTheme,
            defaultDarkTheme: s.defaultDarkTheme,
            resolvedSystemTheme:
              typeof resolvedSystemTheme === 'string' ? resolvedSystemTheme : undefined,
          });
          // Language is correctly set by initI18n() via Rust IPC.
          // User changes via settings are handled in settingsStore.
          // Skip here — vault-stored locale may be stale (navigator.language fallback).
          // P0-1: Load custom pages from objects table (separate from profile preferences)
          // Must run AFTER loadSettings finishes to avoid race condition where
          // loadSettings overwrites customPages with DEFAULT_SETTINGS.
          // P127: await + catch——失败不再产生 unhandled rejection 或自定义页面静默缺失。
          try {
            await useSettingsStore.getState().loadCustomPages(account.id);
          } catch (err) {
            logger.warn('[AppRoutes] Failed to load custom pages:', err);
          }
        });
    }
  }, [isAuthenticated]);

  useApplyThemeFromSettings();
  useAutoLock();

  // 延迟初始化通知监听器，直到用户解锁 Vault 后再注册，避免启动时
  // 触发权限申请或占用资源（MOB-P3-03）。
  useEffect(() => {
    if (!isAuthenticated) return;
    initLlmNotificationListener().catch((err) =>
      logger.warn('[AppRoutes] LLM notification listener failed:', err),
    );
  }, [isAuthenticated]);

  // 初始化 SAF 同步事件监听（仅在 Android 上有效）
  useEffect(() => {
    if (!isAuthenticated) return;
    useSafSyncStore.getState().startListening();
    return () => {
      useSafSyncStore.getState().stopListening();
    };
  }, [isAuthenticated]);

  // 监听 saf-auth-revoked 事件：SAF 授权被系统撤销时通知用户
  useEffect(() => {
    if (!isAuthenticated) return;
    const unlisten = listen('saf-auth-revoked', () => {
      // SAF 授权已失效，提示用户前往设置重新选择。
      // auto-sync 在目录失效期间每 30s 周期性重试都会发射该事件——
      // 用专用 safAuthToastShown 标志去重（同一会话只弹一次 toast），
      // 不能复用 safAuthRevoked：GlobalSyncIndicator 先注册监听并置位它，
      // 共用会吞掉首次 toast（整场零提示）。
      logger.warn('[AppRoutes] SAF auth revoked event received');
      const ui = useUiStore.getState();
      if (ui.safAuthToastShown) return;
      ui.setSafAuthToastShown(true);
      ui.setSafAuthRevoked(true);
      ui.showToast({
        type: 'warning',
        message: t(
          'settings:vault_directory_invalid_toast',
          'SAF directory access revoked. Go to Settings > Data Management to re-select.',
        ),
        duration: 10000,
      });
    });
    return () => {
      unlisten.then((f) => f());
    };
  }, [isAuthenticated, t]);

  // Listen for system theme changes (via Tauri Event from Rust backend)
  useEffect(() => {
    let unlistenSystemTheme: (() => void) | undefined;
    (async () => {
      unlistenSystemTheme = await listenForSystemTheme((mode) => {
        const s = useSettingsStore.getState().settings;
        if (s.theme !== 'system') return;
        void applyTheme({
          preset: 'system',
          accentColor: s.accentColor,
          backgroundType: s.backgroundType,
          backgroundValue: s.backgroundValue,
          defaultLightTheme: s.defaultLightTheme,
          defaultDarkTheme: s.defaultDarkTheme,
          resolvedSystemTheme: mode,
        });
      });
    })();

    return () => {
      unlistenSystemTheme?.();
    };
  }, []);

  // Listen for vault-locked event — clear sensitive state and redirect
  useEffect(() => {
    const unlisten = listen('vault-locked', async () => {
      useObjectStore.getState().clearOnVaultLock();
      useSettingsStore.getState().clearOnVaultLock();
      useProfileStore.getState().clear();
      // P004/P005: 锁定后立即清理回收站解密摘要与搜索明文缓存，
      // 避免解密数据残留在内存直至 TTL 自然过期。
      useTrashStore.getState().clearOnVaultLock();
      // P230: 锁定后清空 OCR 扫描结果内存态（含 MRZ 证件号）。
      useOcrScanStore.getState().clearOnVaultLock();
      // N-3: 锁定后清空 LLM 流式缓冲明文（streamBuffer/streamError），
      // 并取消进行中的 llm-stream-chunk 事件订阅，避免对话内容残留内存。
      useLlmStore.getState().reset();
      searchCache.clear();
      useAuthStore.getState().logout();
      // Re-check account state so hasAccount resolves from null → true/false
      // (otherwise /login route stays on "Connecting...")
      await useAuthStore.getState().checkHasAccount();
      navigate('/login');
    });
    return () => {
      unlisten.then((f) => f());
    };
  }, [navigate]);

  // Android 快捷方式「新建对象」：监听 Kotlin 端注入的 DOM 事件
  useEffect(() => {
    const handleShortcut = () => {
      if (isAuthenticated) {
        navigate('/editor?new=1');
      } else {
        sessionStorage.setItem('solosoul_pending_shortcut', 'new_object');
      }
    };

    // 消费可能来自冷启动的 pending action
    // 注意：必须先判断登录态再移除 pending，否则未登录时 pending 会被吞掉。
    const pending = sessionStorage.getItem('solosoul_pending_shortcut');
    if (pending === 'new_object' && isAuthenticated) {
      sessionStorage.removeItem('solosoul_pending_shortcut');
      navigate('/editor?new=1');
    }

    // 暴露全局回调供 Kotlin evaluateJavascript 调用
    (
      window as typeof window & { __SOLOSOUL_HANDLE_SHORTCUT__?: (action: string) => void }
    ).__SOLOSOUL_HANDLE_SHORTCUT__ = (action: string) => {
      if (action === 'new_object') {
        handleShortcut();
      }
    };

    return () => {
      delete (window as typeof window & { __SOLOSOUL_HANDLE_SHORTCUT__?: (action: string) => void })
        .__SOLOSOUL_HANDLE_SHORTCUT__;
    };
  }, [navigate, isAuthenticated]);

  // P015-R: 路由 chunk 后台预取——懒加载后首次进入未访问页面需拉取页面 chunk 及其
  // 共享依赖 chunk（如 PageContainer 561K / RecoveryQrScanner 375K），期间整窗显示
  // Suspense 占位，桌面端感知为半秒空白。登录解锁后分批预取全部路由 chunk（含认证页），
  // 之后切换页面全部命中缓存，空白消失；移动端首屏仍保持瘦加载，预取仅登录后触发。
  useEffect(() => {
    if (!isAuthenticated) return;
    let cancelled = false;
    const loaders = [loadBootstrapPage, loadLoginPage, ...routeLoaders];
    let index = 0;
    const BATCH = 3;
    const TICK_MS = 80;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const tick = () => {
      if (cancelled) return;
      for (let n = 0; n < BATCH && index < loaders.length; n += 1, index += 1) {
        void loaders[index]().catch(() => {
          // 预取失败静默忽略：目标页面仍会按需加载
        });
      }
      if (index < loaders.length) {
        timer = setTimeout(tick, TICK_MS);
      }
    };
    // 等一帧再开始，避免与登录后的首帧渲染竞争主线程
    const raf = requestAnimationFrame(() => tick());
    return () => {
      cancelled = true;
      cancelAnimationFrame(raf);
      if (timer) clearTimeout(timer);
    };
  }, [isAuthenticated]);

  // 支持 /bootstrap?mode=create 在已有账户时仍能创建新账户
  const [searchParams] = useSearchParams();
  const bootstrapMode = searchParams.get('mode');

  return (
    <>
      {(updateState.kind !== 'hidden' || showOcrBanner) && (
        <div
          style={{
            position: 'fixed',
            top: SAFE_AREA_TOP,
            left: 0,
            right: 0,
            // 高于 AppBar（1000）：登录解锁后横幅不被顶部栏遮挡；
            // 低于弹窗（--z-auth-modal: 8000）与 toast（--z-toast: 9000）
            zIndex: 'var(--z-modal)',
            display: 'flex',
            flexDirection: 'column',
          }}
        >
          {updateState.kind !== 'hidden' && (
            <UpdateBanner
              version={updateState.version}
              state={updateState.kind as UpdateBannerState}
              downloadedBytes={updateState.downloadedBytes}
              totalBytes={updateState.totalBytes}
              progressPercent={updateState.progressPercent}
              mandatory={updateState.mandatory}
              error={updateState.error}
              releaseNotes={updateState.releaseNotes}
              checksumWarning={updateState.checksumWarning}
              onUpdate={startDownload}
              onInstall={installUpdate}
              onSkip={() => {
                if (!updateState.mandatory) {
                  localStorage.setItem(ST_SKIPPED_VERSION, updateState.version);
                }
                dismissUpdate();
              }}
              onClose={dismissUpdate}
            />
          )}
          {showOcrBanner && (
            <OcrInstallBanner
              phase={ocrPhase}
              progress={progress}
              error={error}
              onRetry={retryOcrInstall}
              onClose={closeOcrBanner}
            />
          )}
          <SafSyncIndicator />
        </div>
      )}
      {isAuthenticated && <PostLoginSetupGuide />}
      {/* P015: Suspense 承接路由级懒加载 chunk 拉取期。P015-R: 纯色占位升级为骨架屏，
          残余拉取期保持侧边栏/导航与内容区结构，避免「整窗空白」感知。 */}
      <Suspense fallback={<RouteLoadingSkeleton />}>
        <Routes>
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
                    height: '100vh',
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
                    height: '100vh',
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
          {protectedRoutes.map((r) => (
            <Route key={r.path} path={r.path} element={<AuthGuard>{r.element}</AuthGuard>} />
          ))}
          <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>
      </Suspense>
    </>
  );
}
