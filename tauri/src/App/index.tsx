import { useState, useEffect } from 'react';
import { BrowserRouter } from 'react-router-dom';
import { invoke } from '@tauri-apps/api/core';
import type { AccountInfo } from '@/lib/ipc';
import { ST_ONBOARDING_SEEN } from '@/lib/storageKeys';
import { ToastContainer } from '@/components/ui/ToastContainer';
import { OcrScanNotificationListener } from '@/components/layout/OcrScanNotificationListener';
import { PluginQuickNotificationListener } from '@/components/plugin/PluginQuickNotificationListener';
import { OnboardingDialog } from '@/components/onboarding/OnboardingDialog';
import { AppRoutes } from './AppRoutes';

function App() {
  const [hasSeenOnboarding, setHasSeenOnboarding] = useState(() => {
    try {
      return localStorage.getItem(ST_ONBOARDING_SEEN) === 'true';
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
      localStorage.setItem(ST_ONBOARDING_SEEN, 'true');
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
      <OcrScanNotificationListener />
      <PluginQuickNotificationListener />
      {!hasSeenOnboarding && (
        <OnboardingDialog onComplete={finishOnboarding} onSkip={finishOnboarding} />
      )}
    </BrowserRouter>
  );
}

export default App;
