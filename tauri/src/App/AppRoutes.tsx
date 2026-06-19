import { useCallback, useEffect, useState } from 'react';
import { Routes, Route, Navigate, useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { listen } from '@tauri-apps/api/event';
import { useAuthStore } from '@/stores/authStore';
import { useObjectStore } from '@/stores/objectStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { useProfileStore } from '@/stores/profileStore';
import { applyTheme, listenForSystemTheme, stopListeningForSystemTheme } from '@/lib/theme';
import { useWindowSize } from '@/hooks/useWindowSize';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { confirm } from '@tauri-apps/plugin-dialog';
import { UpdateBanner, type UpdateBannerState } from '@/components/ui/UpdateBanner';
import { OcrInstallBanner } from '@/components/ui/OcrInstallBanner';
import { relaunch } from '@tauri-apps/plugin-process';
import type { Update } from '@tauri-apps/plugin-updater';
import { checkForUpdate } from '@/lib/updater';
import {
  useOcrInstallStore,
  isOcrFirstInstallDone,
  markOcrFirstInstallDone,
} from '@/stores/ocrInstallStore';
import { commands } from '@/lib/ipc';
import { protectedRoutes, AuthGuard } from './routes';
import { BootstrapPage }  from '@/pages/auth/BootstrapPage';
import { LoginPage }  from '@/pages/auth/LoginPage';

export function AppRoutes() {
  const navigate = useNavigate();
  const { t } = useTranslation('ocr');
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

  // 启动时检查更新并显示非侵入式横幅
  useEffect(() => {
    checkForUpdate().then((result) => {
      if (result.kind !== 'available') return;
      const skipped = localStorage.getItem('solosoul_skipped_version');
      if (skipped === result.info.version) return;
      setUpdateState({
        kind: 'available',
        update: result.update,
        version: result.info.version,
        downloadedBytes: 0,
        totalBytes: 0,
      });
    });
  }, []);

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

  // 首次启动时静默安装 bundled small OCR 模型
  const triggerOcrFirstInstall = useCallback(async () => {
    if (isOcrFirstInstallDone()) return;
    try {
      const status = await commands.ocrGetModelStatus('small');
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
      await commands.ocrInstallBundledModelWithProgress('small');
    } catch {
      // 错误会通过 ocr-install-progress 事件进入 store；这里兜底确保 banner 不消失。
      setShowOcrBanner(true);
    }
  }, [startListening]);

  useEffect(() => {
    triggerOcrFirstInstall();
  }, [triggerOcrFirstInstall]);

  // 安装完成或出错后，若已不在安装中且已完成，隐藏 banner。
  useEffect(() => {
    if (!isInstalling && isOcrFirstInstallDone()) {
      setShowOcrBanner(false);
    }
  }, [isInstalling]);

  // OCR 模型安装期间拦截窗口关闭，提示用户避免退出导致安装不完整。
  useEffect(() => {
    if (!isInstalling) return;

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
      .catch(() => {});

    return () => {
      unlisten?.();
    };
  }, [isInstalling, t]);

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
        .then(() => {
          // Re-apply theme with loaded settings (otherwise stays at defaults)
          const s = useSettingsStore.getState().settings;
          applyTheme({
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

  // Listen to window resize and save size to UI preferences (available before login)
  useWindowSize();

  // Apply theme on mount (4.3.5 — instant, no refresh needed)
  useEffect(() => {
    const settings = useSettingsStore.getState().settings;
    applyTheme({
      preset:
        settings.theme === 'dark'
          ? 'warm-stone-dark'
          : settings.theme === 'light'
            ? 'warm-stone-light'
            : 'system',
      accentColor: settings.accentColor,
      backgroundType: settings.backgroundType,
      backgroundValue: settings.backgroundValue,
      defaultLightTheme: settings.defaultLightTheme,
      defaultDarkTheme: settings.defaultDarkTheme,
    });

    // Language is already set by initI18n() via IPC (authoritative).
    // User-saved preferences are applied in the isAuthenticated effect (line 69-72).
    // Skip here to avoid overriding correct detection with DEFAULT_SETTINGS on first launch.

    // Listen for system theme changes
    const config = {
      preset: (settings.theme === 'dark'
        ? 'warm-stone-dark'
        : settings.theme === 'light'
          ? 'warm-stone-light'
          : 'system') as import('@/types').ThemePreset,
      accentColor: settings.accentColor as
        | 'ocean'
        | 'amber'
        | 'forest'
        | 'rose'
        | 'purple'
        | 'custom',
      backgroundType: settings.backgroundType as 'solid' | 'gradient' | 'image',
      backgroundValue: settings.backgroundValue,
      defaultLightTheme: settings.defaultLightTheme,
      defaultDarkTheme: settings.defaultDarkTheme,
    };
    listenForSystemTheme(config, () => {
      const s = useSettingsStore.getState().settings;
      applyTheme({
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
      });
    });

    return () => {
      stopListeningForSystemTheme();
    };
  }, []);

  // Listen for vault-locked event — clear sensitive state and redirect
  useEffect(() => {
    const unlisten = listen('vault-locked', () => {
      useObjectStore.getState().clearOnVaultLock();
      useSettingsStore.getState().clearOnVaultLock();
      useProfileStore.getState().clear();
      useAuthStore.getState().logout();
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
                localStorage.setItem('solosoul_skipped_version', updateState.version);
                setUpdateState({ kind: 'hidden' });
              }}
              onClose={() => setUpdateState({ kind: 'hidden' })}
            />
          )}
          {showOcrBanner && (
            <OcrInstallBanner progress={progress} error={error} onRetry={retryOcrInstall} />
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
                  fontSize: 14,
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
                  fontSize: 14,
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

