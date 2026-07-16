import { useCallback, useEffect, useState } from 'react';
import { Routes, Route, Navigate, useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { listen } from '@tauri-apps/api/event';
import { useAuthStore } from '@/stores/authStore';
import { useObjectStore } from '@/stores/objectStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { useProfileStore } from '@/stores/profileStore';
import { useApplyThemeFromSettings } from '@/hooks/useApplyThemeFromSettings';
import { useAutoLock } from '@/hooks/useAutoLock';
import { applyTheme, getSystemTheme, listenForSystemTheme } from '@/lib/theme';
import { isMobilePlatformSync } from '@/lib/platform';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { confirm } from '@tauri-apps/plugin-dialog';
import { UpdateBanner, type UpdateBannerState } from '@/components/ui/UpdateBanner';
import { OcrInstallBanner, type OcrInstallPhase } from '@/components/ui/OcrInstallBanner';
import { relaunch } from '@tauri-apps/plugin-process';
import type { Update } from '@tauri-apps/plugin-updater';
import { checkForUpdate } from '@/lib/updater';
import {
  useOcrInstallStore,
  isOcrFirstInstallDone,
  markOcrFirstInstallDone,
} from '@/stores/ocrInstallStore';
import { invoke } from '@tauri-apps/api/core';
import { ST_SKIPPED_VERSION } from '@/lib/constants';
import type { OcrModelStatus } from '@/lib/ipc';
import { protectedRoutes, AuthGuard } from './routes';
import { BootstrapPage } from '@/pages/auth/BootstrapPage';
import { LoginPage } from '@/pages/auth/LoginPage';

export function AppRoutes() {
  const navigate = useNavigate();
  const { t } = useTranslation('ocr');
  const isMobile = isMobilePlatformSync();
  const { checkHasAccount, hasAccount, isAuthenticated } = useAuthStore();
  const [updateState, setUpdateState] = useState<
    | { kind: 'hidden' }
    | {
        kind: 'available' | 'downloading' | 'downloaded' | 'error';
        update: Update;
        version: string;
        downloadedBytes: number;
        totalBytes: number;
        error?: string;
      }
  >({ kind: 'hidden' });
  const [showOcrBanner, setShowOcrBanner] = useState(false);
  const { isInstalling, progress, error, startListening } = useOcrInstallStore();

  // Derive OCR banner phase from store state for the new banner component.
  const ocrPhase: OcrInstallPhase = error ? 'error' : isInstalling ? 'installing' : 'completed';

  // 启动时检查更新并显示非侵入式横幅（桌面端）
  useEffect(() => {
    if (isMobile) return;
    checkForUpdate().then((result) => {
      if (result.kind !== 'available') return;
      const skipped = localStorage.getItem(ST_SKIPPED_VERSION);
      if (skipped === result.info.version) return;
      setUpdateState({
        kind: 'available',
        update: result.update,
        version: result.info.version,
        downloadedBytes: 0,
        totalBytes: 0,
      });
    });
  }, [isMobile]);

  const startDownload = useCallback(async () => {
    if (updateState.kind !== 'available' && updateState.kind !== 'error') return;
    const { update, version } = updateState;
    setUpdateState({ kind: 'downloading', update, version, downloadedBytes: 0, totalBytes: 0 });
    try {
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
      setUpdateState({ kind: 'downloaded', update, version, downloadedBytes: 0, totalBytes: 0 });
    } catch (err) {
      setUpdateState({
        kind: 'error',
        update,
        version,
        downloadedBytes: 0,
        totalBytes: 0,
        error: err instanceof Error ? err.message : String(err),
      });
    }
  }, [updateState]);

  const installUpdate = useCallback(async () => {
    if (updateState.kind !== 'downloaded') return;
    try {
      await updateState.update.install();
      await relaunch();
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
  }, [updateState]);

  // 首次启动时静默安装 bundled small OCR 模型（桌面端）
  const triggerOcrFirstInstall = useCallback(async () => {
    if (isMobile) {
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
  }, [startListening, isMobile]);

  useEffect(() => {
    triggerOcrFirstInstall();
  }, [triggerOcrFirstInstall]);

  // Banner auto-dismiss is now handled inside OcrInstallBanner component.

  // OCR 模型安装期间拦截窗口关闭，提示用户避免退出导致安装不完整（桌面端）
  useEffect(() => {
    if (isMobile || !isInstalling) return;

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
      .catch((err) => console.warn('[AppRoutes] CloseRequested listener failed:', err));

    return () => {
      unlisten?.();
    };
  }, [isInstalling, t, isMobile]);

  useEffect(() => {
    checkHasAccount();
  }, [checkHasAccount]);

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

  const retryOcrInstall = useCallback(() => {
    useOcrInstallStore.getState().reset();
    triggerOcrFirstInstall();
  }, [triggerOcrFirstInstall]);

  return (
    <>
      {(updateState.kind !== 'hidden' || showOcrBanner) && (
        <div
          style={{
            position: 'fixed',
            top: 0,
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
              error={updateState.error}
              onUpdate={startDownload}
              onInstall={installUpdate}
              onSkip={() => {
                localStorage.setItem(ST_SKIPPED_VERSION, updateState.version);
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
        </div>
      )}
      <Routes>
        <Route
          path="/bootstrap"
          element={
            hasAccount === true ? (
              <Navigate to="/login" replace />
            ) : hasAccount === false ? (
              <BootstrapPage />
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
