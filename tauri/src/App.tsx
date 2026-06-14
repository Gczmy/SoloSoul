import { useEffect, useState } from 'react';
import { BrowserRouter, Routes, Route, Navigate, useNavigate } from 'react-router-dom';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import type { AccountInfo } from '@/lib/ipc';
import { useAuthStore } from '@/stores/authStore';
import { useObjectStore } from '@/stores/objectStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { useProfileStore } from '@/stores/profileStore';
import { applyTheme, listenForSystemTheme, stopListeningForSystemTheme } from '@/lib/theme';
import { useWindowSize } from '@/hooks/useWindowSize';
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
import { LlmStatsPage } from '@/pages/ai/LlmStatsPage';
import { HelpPage } from '@/pages/help/HelpPage';
import { UpdateBanner } from '@/components/ui/UpdateBanner';
import { OnboardingDialog } from '@/components/onboarding/OnboardingDialog';
import { checkForUpdate } from '@/lib/updater';
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
  const { checkHasAccount, hasAccount, isAuthenticated } = useAuthStore();
  const [updateBanner, setUpdateBanner] = useState<{ version: string } | null>(null);

  // 启动时检查更新并显示非侵入式横幅
  useEffect(() => {
    checkForUpdate().then((info) => {
      if (!info) return;
      const skipped = localStorage.getItem('solosoul_skipped_version');
      if (skipped === info.version) return;
      setUpdateBanner({ version: info.version });
    });
  }, []);

  useEffect(() => {
    checkHasAccount();
  }, []);

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

  return (
    <>
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
        try {
          const localSeen = localStorage.getItem('solosoul_onboarding_seen') === 'true';
          if (localSeen) {
            // eslint-disable-next-line no-console
            console.warn(
              '[onboarding] ui_preferences says not seen but localStorage says seen; using IPC',
            );
          }
        } catch {
          /* ignore */
        }
        setHasSeenOnboarding(false);
      })
      .catch((err) => {
        if (cancelled) return;
        // eslint-disable-next-line no-console
        console.warn('[onboarding] Failed to read onboarding state:', err);
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
    invoke('ui_update_preference', { key: 'hasSeenOnboarding', value: 'true' }).catch((err) => {
      // eslint-disable-next-line no-console
      console.warn('[onboarding] Failed to persist hasSeenOnboarding:', err);
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
