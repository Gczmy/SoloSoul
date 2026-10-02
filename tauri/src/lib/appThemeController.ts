import type { AppSettings } from '@/stores/settingsStore';
import type { ThemeConfig } from '@/types';
import { useSettingsStore } from '@/stores/settingsStore';
import { useAuthStore } from '@/stores/authStore';
import { onRequestSessionChange } from '@/lib/sessionRequests';
import { logger } from '@/lib/logger';
import { bindThemeController, themeController } from './themeController';

export function settingsThemeConfig(settings: AppSettings): ThemeConfig {
  return {
    preset:
      settings.theme === 'system'
        ? 'system'
        : settings.theme === 'dark'
          ? 'warm-stone-dark'
          : 'warm-stone-light',
    accentColor: settings.accentColor,
    customAccentHex: settings.customAccentHex,
    backgroundType: settings.backgroundType,
    backgroundValue: settings.backgroundValue,
    defaultLightTheme: settings.defaultLightTheme,
    defaultDarkTheme: settings.defaultDarkTheme,
  };
}

let session = 0;
export function prepareThemeController(): typeof themeController {
  bindThemeController({
    read: () => {
      const settings = useSettingsStore.getState().getConfirmedSettings();
      const auth = useAuthStore.getState();
      return {
        config: settingsThemeConfig(settings),
        context: JSON.stringify([session, auth.isAuthenticated, auth.currentAccount?.id ?? null]),
      };
    },
    subscribe: (refresh) => {
      const stopSettings = useSettingsStore.subscribe(refresh);
      const stopAuth = useAuthStore.subscribe(refresh);
      const stopSession = onRequestSessionChange(() => {
        session += 1;
        refresh();
      });
      return () => {
        stopSettings();
        stopAuth();
        stopSession();
      };
    },
    onError: (error) => logger.warn('[ThemeController] Failed to apply theme:', error),
  });
  return themeController;
}
