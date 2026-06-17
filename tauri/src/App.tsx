import { useCallback, useEffect, useState } from 'react';
import { BrowserRouter, Routes, Route, Navigate, useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import type { AccountInfo } from '@/lib/ipc';
import { useAuthStore } from '@/stores/authStore';
import { useObjectStore } from '@/stores/objectStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { useProfileStore } from '@/stores/profileStore';
import { applyTheme, listenForSystemTheme, stopListeningForSystemTheme } from '@/lib/theme';
import { useWindowSize } from '@/hooks/useWindowSize';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { confirm } from '@tauri-apps/plugin-dialog';
import { BootstrapPage } from '@/pages/auth/BootstrapPage';
import { LoginPage } from '@/pages/auth/LoginPage';
import { HomePage } from '@/pages/home/HomePage';
import { SettingsPage } from '@/pages/settings/SettingsPage';
import { SecuritySettingsPage } from '@/pages/settings/SecuritySettingsPage';
import { DataManagementPage } from '@/pages/settings/DataManagementPage';
import { TrashPage } from '@/pages/settings/TrashPage';
import { ObjectWorkspacePage } from '@/pages/workspace/ObjectWorkspacePage';
import { ObjectEditorPage } from '@/pages/editor/ObjectEditorPage';
import { ExportImportPage } from '@/pages/settings/ExportImportPage';
import { SearchPage } from '@/pages/search/SearchPage';
import { ToastContainer } from '@/components/ui/ToastContainer';
import { OperationLogPage } from '@/pages/settings/OperationLogPage';
import { AboutPage } from '@/pages/system/AboutPage';
import { DebugLogPage } from '@/pages/system/DebugLogPage';
import { AppearanceSettingsPage } from '@/pages/settings/AppearanceSettingsPage';
import { BackupConfigPage } from '@/pages/settings/BackupConfigPage';
import { PluginGatePage } from '@/pages/ai/PluginGatePage';
import { LlmChatPage } from '@/pages/ai/LlmChatPage';
import { LlmConfigPage } from '@/pages/ai/LlmConfigPage';
import { TemplateManagerPage } from '@/pages/settings/TemplateManagerPage';
import { OcrSettingsPage } from '@/pages/settings/OcrSettingsPage';
import { LlmStatsPage } from '@/pages/ai/LlmStatsPage';
import { HelpPage } from '@/pages/help/HelpPage';
import { UpdateBanner } from '@/components/ui/UpdateBanner';
import { OcrInstallBanner } from '@/components/ui/OcrInstallBanner';
import { OnboardingDialog } from '@/components/onboarding/OnboardingDialog';
import { checkForUpdate } from '@/lib/updater';
import {
  useOcrInstallStore,
  isOcrFirstInstallDone,
  markOcrFirstInstallDone,
} from '@/stores/ocrInstallStore';
import { commands } from '@/lib/ipc';
import { ScanLocalPage } from '@/pages/scan/ScanLocalPage';
import { OcrPage } from '@/pages/scan/OcrPage';
import { HistoryPage } from '@/pages/editor/HistoryPage';
import { SyncPage } from '@/pages/sync/SyncPage';

function AuthGuard({ children }: { children: React.ReactNode }) {
  const isAuthenticated = useAuthStore((s) => s.isAuthenticated);
  if (!isAuthenticated) return <Navigate to="/login" replace />;
  return <>{children}</>;
}

function AppRoutes() {
  const navigate = useNavigate();
  const { t } = useTranslation('ocr');
  const { checkHasAccount, hasAccount, isAuthenticated } = useAuthStore();
  const [updateBanner, setUpdateBanner] = useState<{ version: string } | null>(null);
  const [showOcrBanner, setShowOcrBanner] = useState(false);
  const { isInstalling, progress, error, startListening } = useOcrInstallStore();

  // 启动时检查更新并显示非侵入式横幅
  useEffect(() => {
    checkForUpdate().then((result) => {
      if (result.kind !== 'available') return;
      const skipped = localStorage.getItem('solosoul_skipped_version');
      if (skipped === result.info.version) return;
      setUpdateBanner({ version: result.info.version });
    });
  }, []);

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
      {(updateBanner || showOcrBanner) && (
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
          {updateBanner && (
            <UpdateBanner
              version={updateBanner.version}
              onUpdate={() => {
                setUpdateBanner(null);
                navigate('/about');
              }}
              onSkip={() => {
                localStorage.setItem('solosoul_skipped_version', updateBanner.version);
                setUpdateBanner(null);
              }}
              onClose={() => setUpdateBanner(null)}
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
        <Route
          path="/"
          element={
            <AuthGuard>
              <HomePage />
            </AuthGuard>
          }
        />
        <Route
          path="/search"
          element={
            <AuthGuard>
              <SearchPage />
            </AuthGuard>
          }
        />
        <Route
          path="/settings"
          element={
            <AuthGuard>
              <SettingsPage />
            </AuthGuard>
          }
        />
        <Route
          path="/settings/appearance"
          element={
            <AuthGuard>
              <AppearanceSettingsPage />
            </AuthGuard>
          }
        />
        <Route
          path="/settings/security"
          element={
            <AuthGuard>
              <SecuritySettingsPage />
            </AuthGuard>
          }
        />
        <Route
          path="/settings/export-import"
          element={
            <AuthGuard>
              <ExportImportPage />
            </AuthGuard>
          }
        />
        <Route
          path="/settings/data"
          element={
            <AuthGuard>
              <DataManagementPage />
            </AuthGuard>
          }
        />
        <Route
          path="/settings/trash"
          element={
            <AuthGuard>
              <TrashPage />
            </AuthGuard>
          }
        />
        <Route
          path="/settings/operation-log"
          element={
            <AuthGuard>
              <OperationLogPage />
            </AuthGuard>
          }
        />
        <Route
          path="/settings/backup"
          element={
            <AuthGuard>
              <BackupConfigPage />
            </AuthGuard>
          }
        />
        <Route
          path="/about"
          element={
            <AuthGuard>
              <AboutPage />
            </AuthGuard>
          }
        />
        <Route
          path="/debug-log"
          element={
            <AuthGuard>
              <DebugLogPage />
            </AuthGuard>
          }
        />
        <Route
          path="/plugins"
          element={
            <AuthGuard>
              <PluginGatePage />
            </AuthGuard>
          }
        />
        <Route
          path="/settings/templates"
          element={
            <AuthGuard>
              <TemplateManagerPage />
            </AuthGuard>
          }
        />
        <Route
          path="/settings/ocr"
          element={
            <AuthGuard>
              <OcrSettingsPage />
            </AuthGuard>
          }
        />
        <Route
          path="/settings/llm"
          element={
            <AuthGuard>
              <LlmConfigPage />
            </AuthGuard>
          }
        />
        <Route
          path="/settings/llm/stats"
          element={
            <AuthGuard>
              <LlmStatsPage />
            </AuthGuard>
          }
        />
        <Route
          path="/llm-chat"
          element={
            <AuthGuard>
              <LlmChatPage />
            </AuthGuard>
          }
        />
        <Route
          path="/local-import"
          element={
            <AuthGuard>
              <ScanLocalPage />
            </AuthGuard>
          }
        />
        <Route
          path="/history"
          element={
            <AuthGuard>
              <HistoryPage />
            </AuthGuard>
          }
        />
        <Route
          path="/ocr"
          element={
            <AuthGuard>
              <OcrPage />
            </AuthGuard>
          }
        />
        <Route
          path="/sync"
          element={
            <AuthGuard>
              <SyncPage />
            </AuthGuard>
          }
        />
        <Route
          path="/help"
          element={
            <AuthGuard>
              <HelpPage />
            </AuthGuard>
          }
        />
        <Route
          path="/workspace"
          element={
            <AuthGuard>
              <ObjectWorkspacePage />
            </AuthGuard>
          }
        />
        <Route
          path="/editor"
          element={
            <AuthGuard>
              <ObjectEditorPage />
            </AuthGuard>
          }
        />
        <Route
          path="/editor/:objectId"
          element={
            <AuthGuard>
              <ObjectEditorPage />
            </AuthGuard>
          }
        />
        <Route
          path="/workspace/custom/:pageId"
          element={
            <AuthGuard>
              <ObjectWorkspacePage />
            </AuthGuard>
          }
        />
        <Route path="*" element={<Navigate to="/" replace />} />
      </Routes>
    </>
  );
}

function App() {
  const [hasSeenOnboarding, setHasSeenOnboarding] = useState(() => {
    try {
      return localStorage.getItem('solosoul_onboarding_seen') === 'true';
    } catch {
      return true;
    }
  });

  useEffect(() => {
    let cancelled = false;
    Promise.all([
      invoke<{ hasSeenOnboarding?: boolean }>('ui_get_preferences').catch(() => ({
        hasSeenOnboarding: false,
      })),
      invoke<AccountInfo[]>('vault_list_accounts').catch(() => [] as AccountInfo[]),
    ])
      .then(([prefs, accounts]) => {
        if (cancelled) return;
        // If the user already has at least one account, they have clearly completed
        // onboarding before — hide the tutorial regardless of the UI pref flag.
        if (accounts.length > 0) {
          setHasSeenOnboarding(true);
          return;
        }
        const ipcSeen = prefs.hasSeenOnboarding === true;
        if (ipcSeen) {
          setHasSeenOnboarding(true);
          return;
        }
        // IPC is authoritative: if it says not seen, ignore stale localStorage
        setHasSeenOnboarding(false);
      })
      .catch(() => {
        if (cancelled) return;
        // Fallback to localStorage already applied in initial state
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const finishOnboarding = () => {
    try {
      localStorage.setItem('solosoul_onboarding_seen', 'true');
    } catch {
      /* ignore */
    }
    invoke('ui_update_preference', { key: 'hasSeenOnboarding', value: 'true' }).catch(() => {
      /* ignore persistence errors; localStorage fallback is already set */
    });
    setHasSeenOnboarding(true);
  };

  return (
    <BrowserRouter>
      <AppRoutes />
      <ToastContainer />
      {!hasSeenOnboarding && (
        <OnboardingDialog onComplete={finishOnboarding} onSkip={finishOnboarding} />
      )}
    </BrowserRouter>
  );
}

export default App;
