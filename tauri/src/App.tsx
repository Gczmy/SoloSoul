import { useEffect } from 'react';
import { BrowserRouter, Routes, Route, Navigate, useNavigate } from 'react-router-dom';
import { listen } from '@tauri-apps/api/event';
import { useAuthStore } from '@/stores/authStore';
import { useObjectStore } from '@/stores/objectStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { useProfileStore } from '@/stores/profileStore';
import { applyTheme, listenForSystemTheme } from '@/lib/theme';
import { BootstrapPage } from '@/pages/auth/BootstrapPage';
import { LoginPage } from '@/pages/auth/LoginPage';
import { HomePage } from '@/pages/home/HomePage';
import { SettingsPage } from '@/pages/settings/SettingsPage';
import { SecuritySettingsPage } from '@/pages/settings/SecuritySettingsPage';
import { DataManagementPage } from '@/pages/settings/DataManagementPage';
import { TrashPage } from '@/pages/settings/TrashPage';
import { ObjectWorkspacePage } from '@/pages/workspace/ObjectWorkspacePage';
import { ObjectEditorPage } from '@/pages/editor/ObjectEditorPage';
import { SensitivitySettingsPage } from '@/pages/settings/SensitivitySettingsPage';
import { ExportImportPage } from '@/pages/settings/ExportImportPage';
import { SearchPage } from '@/pages/search/SearchPage';
import { ToastContainer } from '@/components/ui/ToastContainer';
import { OperationLogPage } from '@/pages/settings/OperationLogPage';
import { AboutPage } from '@/pages/system/AboutPage';
import { DebugLogPage } from '@/pages/system/DebugLogPage';
import { AppearanceSettingsPage } from '@/pages/settings/AppearanceSettingsPage';
import { BackupConfigPage } from '@/pages/settings/BackupConfigPage';
import { PluginDashboardPage } from '@/pages/ai/PluginDashboardPage';
import { LlmChatPage } from '@/pages/ai/LlmChatPage';
import { LlmConfigPage } from '@/pages/ai/LlmConfigPage';
import { LlmStatsPage } from '@/pages/ai/LlmStatsPage';
import { HelpPage } from '@/pages/help/HelpPage';
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

  useEffect(() => {
    checkHasAccount();
  }, []);

  // Load settings and profile after authentication
  useEffect(() => {
    const account = useAuthStore.getState().currentAccount;
    if (isAuthenticated && account) {
      useProfileStore.getState().loadProfile(account.id);
      useSettingsStore.getState().loadSettings(account.id).then(() => {
        // Re-apply theme with loaded settings (otherwise stays at defaults)
        const s = useSettingsStore.getState().settings;
        applyTheme({
          preset: s.theme === 'dark' ? 'warm-stone-dark' :
                  s.theme === 'light' ? 'warm-stone-light' : 'system',
          accentColor: s.accentColor,
          backgroundType: s.backgroundType,
          backgroundValue: s.backgroundValue,
          defaultLightTheme: s.defaultLightTheme,
          defaultDarkTheme: s.defaultDarkTheme,
        });
        if (s.language) {
          import('@/lib/i18n').then((mod) => {
            mod.default.changeLanguage(s.language);
          });
        }
      });
      // P0-1: Load custom pages from objects table (separate from profile preferences)
      useSettingsStore.getState().loadCustomPages(account.id);
    }
  }, [isAuthenticated]);

  // Apply theme on mount (4.3.5 — instant, no refresh needed)
  useEffect(() => {
    const settings = useSettingsStore.getState().settings;
    applyTheme({
      preset: settings.theme === 'dark' ? 'warm-stone-dark' :
              settings.theme === 'light' ? 'warm-stone-light' : 'system',
      accentColor: settings.accentColor,
      backgroundType: settings.backgroundType,
      backgroundValue: settings.backgroundValue,
      defaultLightTheme: settings.defaultLightTheme,
      defaultDarkTheme: settings.defaultDarkTheme,
    });

    // Apply saved language on startup
    if (settings.language) {
      import('@/lib/i18n').then((mod) => {
        mod.default.changeLanguage(settings.language);
      });
    }

    // Listen for system theme changes
    const config = {
      preset: (settings.theme === 'dark' ? 'warm-stone-dark' :
              settings.theme === 'light' ? 'warm-stone-light' : 'system') as import('@/types').ThemePreset,
      accentColor: settings.accentColor as 'ocean' | 'amber' | 'forest' | 'rose' | 'custom',
      backgroundType: settings.backgroundType as 'solid' | 'gradient' | 'image',
      backgroundValue: settings.backgroundValue,
      defaultLightTheme: settings.defaultLightTheme,
      defaultDarkTheme: settings.defaultDarkTheme,
    };
    listenForSystemTheme(config, () => {
      const s = useSettingsStore.getState().settings;
      applyTheme({
        preset: s.theme === 'dark' ? 'warm-stone-dark' :
                s.theme === 'light' ? 'warm-stone-light' : 'system',
        accentColor: s.accentColor,
        backgroundType: s.backgroundType,
        backgroundValue: s.backgroundValue,
        defaultLightTheme: s.defaultLightTheme,
        defaultDarkTheme: s.defaultDarkTheme,
      });
    });
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
    return () => { unlisten.then((f) => f()); };
  }, [navigate]);

  return (
    <Routes>
      <Route
        path="/bootstrap"
        element={
          hasAccount === true ? <Navigate to="/login" replace /> :
          hasAccount === false ? <BootstrapPage /> :
          <div style={{display:'flex',alignItems:'center',justifyContent:'center',height:'100vh',color:'var(--text-secondary)',fontSize:14}}>
            Connecting to backend...
          </div>
        }
      />
      <Route path="/login" element={hasAccount === null ? <div style={{display:'flex',alignItems:'center',justifyContent:'center',height:'100vh',color:'var(--text-secondary)',fontSize:14}}>Connecting...</div> : <LoginPage />} />
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
        path="/settings/sensitivity"
        element={
          <AuthGuard>
            <SensitivitySettingsPage />
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
            <PluginDashboardPage />
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
  );
}

function App() {
  return (
    <BrowserRouter>
      <AppRoutes />
      <ToastContainer />
    </BrowserRouter>
  );
}

export default App;
