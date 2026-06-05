import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';

interface AppSettings {
  theme: 'light' | 'dark' | 'system';
  locale: string;
  autoLockTimeoutMinutes: number;
  biometricEnabled: boolean;
  confirmDelete: boolean;
}

interface SettingsState {
  settings: AppSettings;
  isLoading: boolean;

  loadSettings: (accountId: string) => Promise<void>;
  updateSetting: (accountId: string, key: keyof AppSettings, value: string | number | boolean) => Promise<void>;
  clearOnVaultLock: () => void;
}

const DEFAULT_SETTINGS: AppSettings = {
  theme: 'system',
  locale: 'en',
  autoLockTimeoutMinutes: 5,
  biometricEnabled: false,
  confirmDelete: true,
};

export const useSettingsStore = create<SettingsState>((set, get) => ({
  settings: DEFAULT_SETTINGS,
  isLoading: false,

  loadSettings: async (accountId) => {
    set({ isLoading: true });
    try {
      const prefs = await invoke<Record<string, unknown>>('user_data_get_preferences', { accountId });
      const parsed = { ...DEFAULT_SETTINGS };
      if (prefs.theme && ['light', 'dark', 'system'].includes(prefs.theme as string)) {
        parsed.theme = prefs.theme as AppSettings['theme'];
      }
      if (prefs.locale) parsed.locale = prefs.locale as string;
      if (typeof prefs.autoLockTimeoutMinutes === 'number') parsed.autoLockTimeoutMinutes = prefs.autoLockTimeoutMinutes;
      if (typeof prefs.biometricEnabled === 'boolean') parsed.biometricEnabled = prefs.biometricEnabled;
      if (typeof prefs.confirmDelete === 'boolean') parsed.confirmDelete = prefs.confirmDelete;
      set({ settings: parsed, isLoading: false });
    } catch {
      set({ isLoading: false });
    }
  },

  updateSetting: async (accountId, key, value) => {
    const oldValue = get().settings[key];
    set((s) => ({ settings: { ...s.settings, [key]: value } }));
    try {
      await invoke('user_data_update_preference', {
        payload: { accountId, preferences: { [key]: value } },
      });
    } catch {
      set((s) => ({ settings: { ...s.settings, [key]: oldValue } }));
    }
  },

  clearOnVaultLock: () => set({ settings: DEFAULT_SETTINGS, isLoading: false }),
}));
