import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import i18next from '@/lib/i18n';

interface AppSettings {
  theme: 'light' | 'dark' | 'system';
  accentColor: 'ocean' | 'amber' | 'forest' | 'rose' | 'custom';
  customAccentHex: string;
  backgroundType: 'solid' | 'gradient' | 'image';
  backgroundValue: string;
  language: string;
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
  accentColor: 'ocean',
  customAccentHex: '',
  backgroundType: 'solid',
  backgroundValue: '',
  language: 'en-US',
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
      if (prefs.accentColor && ['ocean', 'amber', 'forest', 'rose', 'custom'].includes(prefs.accentColor as string)) {
        parsed.accentColor = prefs.accentColor as AppSettings['accentColor'];
      }
      if (typeof prefs.customAccentHex === 'string') parsed.customAccentHex = prefs.customAccentHex;
      if (prefs.backgroundType && ['solid', 'gradient', 'image'].includes(prefs.backgroundType as string)) {
        parsed.backgroundType = prefs.backgroundType as AppSettings['backgroundType'];
      }
      if (typeof prefs.backgroundValue === 'string') parsed.backgroundValue = prefs.backgroundValue;
      if (prefs.language && ['zh-CN', 'en-US'].includes(prefs.language as string)) {
        parsed.language = prefs.language as string;
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
      // 15.8 — Language switch is instant
      if (key === 'language' && typeof value === 'string') {
        await i18next.changeLanguage(value);
      }
    } catch {
      set((s) => ({ settings: { ...s.settings, [key]: oldValue } }));
    }
  },

  clearOnVaultLock: () => set({ settings: DEFAULT_SETTINGS, isLoading: false }),
}));
