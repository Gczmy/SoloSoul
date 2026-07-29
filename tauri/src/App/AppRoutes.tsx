import { useCallback, useEffect, useState } from 'react';
import { Routes, Route, Navigate, useNavigate, useSearchParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { listen } from '@tauri-apps/api/event';
import { useAuthStore } from '@/stores/authStore';
import { useObjectStore } from '@/stores/objectStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { useProfileStore } from '@/stores/profileStore';
import { useApplyThemeFromSettings } from '@/hooks/useApplyThemeFromSettings';
import { useAutoLock } from '@/hooks/useAutoLock';
import { initLlmNotificationListener } from '@/lib/notification';
import { applyTheme, getSystemTheme, listenForSystemTheme } from '@/lib/theme';
import { isMobilePlatformSync } from '@/lib/platform';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { confirm } from '@tauri-apps/plugin-dialog';
import { UpdateBanner, type UpdateBannerState } from '@/components/ui/UpdateBanner';
import { OcrInstallBanner, type OcrInstallPhase } from '@/components/ui/OcrInstallBanner';
import { relaunch } from '@tauri-apps/plugin-process';
import type { Update } from '@tauri-apps/plugin-updater';
import {
  checkForUpdate,
  androidCheckForUpdate,
  androidDownloadApk,
  androidInstallApk,
  androidIsApkDownloaded,
  type AndroidUpdateInfo,
} from '@/lib/updater';
import type { UnlistenFn } from '@tauri-apps/api/event';
import {
  useOcrInstallStore,
  isOcrFirstInstallDone,
  markOcrFirstInstallDone,
} from '@/stores/ocrInstallStore';
import { invoke } from '@tauri-apps/api/core';
import { ST_SKIPPED_VERSION, SAFE_AREA_TOP } from '@/lib/constants';
import { logger } from '@/lib/logger';
import { setGlobalNavigate } from '@/lib/navigation';
import { useSafSyncStore } from '@/stores/safSyncStore';
import { SafSyncIndicator } from '@/components/sync/SafSyncIndicator';
import { PostLoginSetupGuide } from '@/components/guide/PostLoginSetupGuide';
import type { OcrModelStatus } from '@/lib/ipc';
import { protectedRoutes, AuthGuard } from './routes';
import { BootstrapPage } from '@/pages/auth/BootstrapPage';
import { LoginPage } from '@/pages/auth/LoginPage';

export function AppRoutes() {
  const navigate = useNavigate();
  useEffect(() => {
    setGlobalNavigate(navigate);
    return () => {
      setGlobalNavigate(null);
    };
  }, [navigate]);
  const { t } = useTranslation(['ocr', 'settings']);
  const isMobilePlatform = isMobilePlatformSync();
  const { checkHasAccount, hasAccount, isAuthenticated } = useAuthStore();
  // 统一更新状态：桌面端持有 Tauri Update 对象，Android 端持有 GitHub Release 信息。
  // `platform` 区分两条下载/安装路径，`mandatory` 透传给 UpdateBanner 隐藏跳过/关闭按钮。
  const [updateState, setUpdateState] = useState<
    | { kind: 'hidden' }
    | {
        kind: 'available' | 'downloading' | 'downloaded' | 'error';
        update: Update | null;
        androidInfo: AndroidUpdateInfo | null;
        version: string;
        downloadedBytes: number;
        totalBytes: number;
        progressPercent: number;
        mandatory: boolean;
        error?: string;
      }
  >({ kind: 'hidden' });
  const [showOcrBanner, setShowOcrBanner] = useState(false);
  const { isInstalling, progress, error, startListening } = useOcrInstallStore();

  // Derive OCR banner phase from store state for the new banner component.
  const ocrPhase: OcrInstallPhase = error ? 'error' : isInstalling ? 'installing' : 'completed';

  // 启动时检查更新并显示非侵入式横幅（桌面端 + Android）
  useEffect(() => {
    if (isMobilePlatform) {
      androidCheckForUpdate().then((result) => {
        if (result.kind !== 'available') return;
        const info = result.info;
        const skipped = localStorage.getItem(ST_SKIPPED_VERSION);
        if (!info.mandatory && skipped === info.latestVersion) return;
        setUpdateState({
          kind: 'available',
          update: null,
          androidInfo: info,
          version: info.latestVersion,
          downloadedBytes: 0,
          totalBytes: 0,
          progressPercent: 0,
          mandatory: info.mandatory,
        });
      });
    } else {
      checkForUpdate().then((result) => {
        if (result.kind !== 'available') return;
        const skipped = localStorage.getItem(ST_SKIPPED_VERSION);
        if (skipped === result.info.version) return;
        setUpdateState({
          kind: 'available',
          update: result.update,
          androidInfo: null,
          version: result.info.version,
          downloadedBytes: 0,
          totalBytes: 0,
          progressPercent: 0,
          mandatory: false,
        });
      });
    }
  }, [isMobilePlatform]);

  const startDownload = useCallback(async () => {
    if (updateState.kind !== 'available' && updateState.kind !== 'error') return;
    setUpdateState((prev) =>
      prev.kind === 'available' || prev.kind === 'error'
        ? {
            ...prev,
            kind: 'downloading' as const,
            downloadedBytes: 0,
            totalBytes: 0,
            progressPercent: 0,
          }
        : prev,
    );
    try {
      if (isMobilePlatform) {
        // Android：通过 GitHub Release 下载 APK，事件驱动进度
        const info = updateState.androidInfo;
        if (!info || !info.downloadUrl) {
          throw new Error('No download URL available');
        }
        // 如果 APK 已下载过，直接进入安装阶段
        const alreadyDownloaded = await androidIsApkDownloaded(updateState.version);
        if (alreadyDownloaded) {
          setUpdateState((prev) =>
            prev.kind === 'downloading'
              ? { ...prev, kind: 'downloaded' as const, progressPercent: 100 }
              : prev,
          );
          return;
        }
        // 下载 APK，完成后 resolve；unlistenFn 用于在完成后移除事件监听器防止泄漏
        let unlistenFn: UnlistenFn | undefined;
        let settled = false;
        const downloadUrl = info.downloadUrl;
        try {
          await new Promise<void>((resolve, reject) => {
            androidDownloadApk(updateState.version, downloadUrl, info.checksum, (progress) => {
              setUpdateState((prev) => {
                if (prev.kind !== 'downloading') return prev;
                return {
                  ...prev,
                  downloadedBytes: progress.downloaded,
                  totalBytes: progress.total,
                  progressPercent: progress.progress,
                };
              });
              if (progress.done && !settled) {
                settled = true;
                if (progress.error) {
                  reject(new Error(progress.error));
                } else {
                  resolve();
                }
              }
            })
              .then((fn) => {
                unlistenFn = fn;
              })
              .catch((err) => {
                if (!settled) {
                  settled = true;
                  reject(err);
                }
              });
          });
        } finally {
          // 无论成功或失败，都移除 Tauri 事件监听器，防止累积泄漏
          unlistenFn?.();
        }
        setUpdateState((prev) =>
          prev.kind === 'downloading'
            ? { ...prev, kind: 'downloaded' as const, progressPercent: 100 }
            : prev,
        );
      } else {
        // 桌面端：使用 Tauri plugin-updater 下载
        const update = updateState.update;
        if (!update) throw new Error('No update available');
        await update.download((event) => {
          setUpdateState((prev) => {
            if (prev.kind !== 'downloading') return prev;
            if (event.event === 'Started') {
              return { ...prev, totalBytes: event.data.contentLength ?? 0 };
            }
            if (event.event === 'Progress') {
              return { ...prev, downloadedBytes: prev.downloadedBytes + event.data.chunkLength };
            }
            if (event.event === 'Finished') {
              return prev;
            }
            return prev;
          });
        });
        setUpdateState((prev) =>
          prev.kind === 'downloading' ? { ...prev, kind: 'downloaded' as const } : prev,
        );
      }
    } catch (err) {
      setUpdateState((prev) => {
        if (prev.kind !== 'downloading') return prev;
        return {
          ...prev,
          kind: 'error' as const,
          error: err instanceof Error ? err.message : String(err),
        };
      });
    }
  }, [updateState, isMobilePlatform]);

  const installUpdate = useCallback(async () => {
    if (updateState.kind !== 'downloaded') return;
    try {
      if (isMobilePlatform) {
        // Android：调用系统包安装器
        await androidInstallApk(updateState.version);
      } else {
        // 桌面端：安装并重启
        if (!updateState.update) throw new Error('No update available');
        await updateState.update.install();
        await relaunch();
      }
    } catch (err) {
      setUpdateState((prev) =>
        prev.kind === 'downloaded'
          ? {
              ...prev,
              kind: 'error' as const,
              error: err instanceof Error ? err.message : String(err),
            }
          : prev,
      );
    }
  }, [updateState, isMobilePlatform]);

  // 首次启动时静默安装 bundled small OCR 模型（桌面端）
  const triggerOcrFirstInstall = useCallback(async () => {
    if (isMobilePlatform) {
      markOcrFirstInstallDone();
      return;
    }
    if (isOcrFirstInstallDone()) return;
    try {
      const status = await invoke<OcrModelStatus>('ocr_get_model_status', { tier: 'small' });
      if (status.installed) {
        markOcrFirstInstallDone();
        return;
      }
      if (!status.bundled) {
        // 安装包未附带 small 模型，跳过自动安装。
        markOcrFirstInstallDone();
        return;
      }
      setShowOcrBanner(true);
      startListening();
      await invoke<void>('ocr_install_bundled_model_with_progress', { tier: 'small' });
    } catch {
      // 错误会通过 ocr-install-progress 事件进入 store；这里兜底确保 banner 不消失。
      setShowOcrBanner(true);
    }
  }, [startListening, isMobilePlatform]);

  useEffect(() => {
    triggerOcrFirstInstall();
  }, [triggerOcrFirstInstall]);

  // Banner auto-dismiss is now handled inside OcrInstallBanner component.

  // OCR 模型安装期间拦截窗口关闭，提示用户避免退出导致安装不完整（桌面端）
  useEffect(() => {
    if (isMobilePlatform || !isInstalling) return;

    const appWindow = getCurrentWindow();
    let unlisten: (() => void) | undefined;

    appWindow
      .onCloseRequested(async (event) => {
        event.preventDefault();
        const confirmed = await confirm(t('quit_while_installing_message'), {
          title: t('quit_while_installing_title'),
          kind: 'warning',
        });
        if (confirmed) {
          await appWindow.close();
        }
      })
      .then((fn) => {
        unlisten = fn;
      })
      .catch((err) => logger.warn('[AppRoutes] CloseRequested listener failed:', err));

    return () => {
      unlisten?.();
    };
  }, [isInstalling, t, isMobilePlatform]);

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
        if (!valid) {
          // SAF 目录失效（用户手动删除了外部目录），弹确认对话框引导用户
          logger.warn('[AppRoutes] SAF vault directory access revoked');
          await confirm(
            t(
              'settings:vault_directory_invalid_message',
              '您之前使用的外部存储目录已被删除或无法访问。\n\nSoloSoul 已将您的数据保留在本地应用存储中，您可以继续正常使用。\n\n如需重新选择外部目录，请前往「设置 > 保险库目录」。',
            ),
            {
              title: t('settings:vault_directory_invalid_title', '存储目录不可用'),
              kind: 'warning',
            },
          );
        }
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
          useSettingsStore.getState().loadCustomPages(account.id);
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
      // SAF 授权已失效，提示用户前往设置重新选择
      logger.warn('[AppRoutes] SAF auth revoked event received');
      import('@/stores/uiStore').then(({ useUiStore }) => {
        useUiStore.getState().showToast({
          type: 'warning',
          message: t(
            'settings:vault_directory_invalid_toast',
            'SAF directory access revoked. Go to Settings > Vault Directory to re-select.',
          ),
          duration: 10000,
        });
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

  const retryOcrInstall = useCallback(() => {
    useOcrInstallStore.getState().reset();
    triggerOcrFirstInstall();
  }, [triggerOcrFirstInstall]);

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
            zIndex: 1000,
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
              onUpdate={startDownload}
              onInstall={installUpdate}
              onSkip={() => {
                if (!updateState.mandatory) {
                  localStorage.setItem(ST_SKIPPED_VERSION, updateState.version);
                }
                setUpdateState({ kind: 'hidden' });
              }}
              onClose={() => setUpdateState({ kind: 'hidden' })}
            />
          )}
          {showOcrBanner && (
            <OcrInstallBanner
              phase={ocrPhase}
              progress={progress}
              error={error}
              onRetry={retryOcrInstall}
              onClose={() => {
                setShowOcrBanner(false);
                markOcrFirstInstallDone();
              }}
            />
          )}
          <SafSyncIndicator />
        </div>
      )}
      {isAuthenticated && <PostLoginSetupGuide />}
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
    </>
  );
}
