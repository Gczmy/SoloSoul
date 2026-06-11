import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import i18next, { detectSystemLanguage } from '@/lib/i18n';
import { applyTheme } from '@/lib/theme';
import { DEFAULT_CUSTOM_ICON } from '@/lib/pageIcons';


// 9.8.3 — Custom page data structure
// Custom pages are now stored in the objects table (P0-1), not in preferences.
// iconId references CUSTOM_ICON_MAP from src/lib/pageIcons.ts (Single Source of Truth)
export interface CustomPage {
  id: string;
  name: string;
  iconId: string;
  createdAt: string;
  sortOrder: number;
  deletedAt?: string;
}

interface AppSettings {
  theme: 'light' | 'dark' | 'system';
  accentColor: 'ocean' | 'amber' | 'forest' | 'rose' | 'purple' | 'custom';
  customAccentHex: string;
  backgroundType: 'solid' | 'gradient' | 'image';
  backgroundValue: string;
  language: string;
  locale: string;
  autoLockTimeoutMinutes: number;
  biometricEnabled: boolean;
  confirmDelete: boolean;
  customPages: CustomPage[];
  defaultLightTheme: string;
  defaultDarkTheme: string;
}

interface SettingsState {
  settings: AppSettings;
  isLoading: boolean;

  /** Load UI-only prefs (theme/language/accent) from plaintext ui_preferences.json.
   *  Can be called before Vault unlock — fixes login page theme bug. */
  loadUiPreferences: () => Promise<void>;
  loadSettings: (accountId: string) => Promise<void>;
  loadCustomPages: (accountId: string) => Promise<void>;
  updateSetting: <K extends keyof AppSettings>(accountId: string, key: K, value: AppSettings[K]) => Promise<void>;
  clearOnVaultLock: () => void;
  addCustomPage: (accountId: string, name: string, iconId?: string) => Promise<CustomPage>;
  removeCustomPage: (accountId: string, pageId: string) => Promise<void>;
}

const DEFAULT_SETTINGS: AppSettings = {
  theme: 'system',
  accentColor: 'ocean',
  customAccentHex: '',
  backgroundType: 'solid',
  backgroundValue: '',
  language: detectSystemLanguage(),
  locale: detectSystemLanguage().startsWith('zh') ? 'zh' : 'en',
  autoLockTimeoutMinutes: 5,
  biometricEnabled: false,
  confirmDelete: true,
  customPages: [],
  defaultLightTheme: 'warm-stone',
  defaultDarkTheme: 'warm-stone-dark',
};

export const useSettingsStore = create<SettingsState>((set, get) => ({
  settings: DEFAULT_SETTINGS,
  isLoading: false,

  /** Load UI-only prefs: read localStorage cache sync first (instant),
   *  then refresh from IPC asynchronously. */
  loadUiPreferences: async () => {
    // Step 1: apply cached prefs instantly from localStorage
    try {
      const raw = localStorage.getItem('solosoul_ui_prefs');
      if (raw) {
        const cached = JSON.parse(raw);
        const p = { ...get().settings };
        if (cached.theme) p.theme = cached.theme;
        if (cached.accentColor) p.accentColor = cached.accentColor;
        if (cached.defaultLightTheme) p.defaultLightTheme = cached.defaultLightTheme;
        if (cached.defaultDarkTheme) p.defaultDarkTheme = cached.defaultDarkTheme;
        applyTheme({
          preset: p.theme === 'dark' ? 'warm-stone-dark' :
                  p.theme === 'light' ? 'warm-stone-light' : 'system',
          accentColor: p.accentColor,
          backgroundType: 'solid',
          backgroundValue: '',
          defaultLightTheme: p.defaultLightTheme,
          defaultDarkTheme: p.defaultDarkTheme,
        });
        set({ settings: p });
      }
    } catch { /* ignore */ }

    // Step 2: fetch fresh prefs from IPC (slow, async)
    try {
      const prefs = await invoke<{
        theme?: string;
        accentColor?: string;
        language?: string;
        defaultLightTheme?: string;
        defaultDarkTheme?: string;
      }>('ui_get_preferences');
      const parsed = { ...get().settings };
      if (prefs.theme) parsed.theme = prefs.theme as AppSettings['theme'];
      if (prefs.accentColor) parsed.accentColor = prefs.accentColor as AppSettings['accentColor'];
      if (prefs.language) parsed.language = prefs.language;
      if (prefs.defaultLightTheme) parsed.defaultLightTheme = prefs.defaultLightTheme;
      if (prefs.defaultDarkTheme) parsed.defaultDarkTheme = prefs.defaultDarkTheme;
      applyTheme({
        preset: parsed.theme === 'dark' ? 'warm-stone-dark' :
                parsed.theme === 'light' ? 'warm-stone-light' : 'system',
        accentColor: parsed.accentColor,
        backgroundType: 'solid',
        backgroundValue: '',
        defaultLightTheme: parsed.defaultLightTheme,
        defaultDarkTheme: parsed.defaultDarkTheme,
      });
      set({ settings: parsed });
      try {
        localStorage.setItem('solosoul_ui_prefs', JSON.stringify({
          theme: parsed.theme,
          accentColor: parsed.accentColor,
          defaultLightTheme: parsed.defaultLightTheme,
          defaultDarkTheme: parsed.defaultDarkTheme,
        }));
      } catch { /* ignore */ }
      if (prefs.language) {
        import('@/lib/i18n').then((mod) => { mod.default.changeLanguage(prefs.language!); });
      }
    } catch { /* no ui_preferences file yet */ }
  },

  loadSettings: async (accountId) => {
    set({ isLoading: true });
    try {
      const prefs = await invoke<Record<string, unknown>>('user_data_get_preferences', { accountId });
      const parsed = { ...DEFAULT_SETTINGS };
      if (prefs.theme && ['light', 'dark', 'system'].includes(prefs.theme as string)) {
        parsed.theme = prefs.theme as AppSettings['theme'];
      }
      if (prefs.accentColor && ['ocean', 'amber', 'forest', 'rose', 'purple', 'custom'].includes(prefs.accentColor as string)) {
        parsed.accentColor = prefs.accentColor as AppSettings['accentColor'];
      }
      if (typeof prefs.defaultLightTheme === 'string') parsed.defaultLightTheme = prefs.defaultLightTheme;
      if (typeof prefs.defaultDarkTheme === 'string') parsed.defaultDarkTheme = prefs.defaultDarkTheme;
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
      // Load old-format customPages from preferences for migration.
      // Once loaded, also try the new objects-table source via loadCustomPages().
      if (Array.isArray(prefs.customPages)) {
        parsed.customPages = prefs.customPages as CustomPage[];
      }
      set({ settings: parsed, isLoading: false });
      // Sync UI prefs to plaintext file so next startup shows correct theme
      if (parsed.theme) invoke('ui_update_preference', { key: 'theme', value: parsed.theme }).catch(() => {});
      if (parsed.accentColor) invoke('ui_update_preference', { key: 'accentColor', value: parsed.accentColor }).catch(() => {});
      if (parsed.language) invoke('ui_update_preference', { key: 'language', value: parsed.language }).catch(() => {});
      if (parsed.defaultLightTheme) invoke('ui_update_preference', { key: 'defaultLightTheme', value: parsed.defaultLightTheme }).catch(() => {});
      if (parsed.defaultDarkTheme) invoke('ui_update_preference', { key: 'defaultDarkTheme', value: parsed.defaultDarkTheme }).catch(() => {});
    } catch {
      set({ isLoading: false });
    }
  },

  /** Load custom pages from the objects table (P0-1 — objects storage layer).
   *  If the objects table has pages, use those (new format).
   *  If empty but old-format pages exist in preferences, migrate them automatically. */
  loadCustomPages: async (accountId) => {
    try {
      const objects = await invoke<Array<{ id: string; name: string; collectionType: string; createdAt: string; updatedAt: string; isDeleted?: boolean }>>(
        'object_list',
        { accountId, filter: { collectionType: 'page', includeDeleted: true } }
      );
      if (objects.length > 0) {
        // New-format pages exist in objects table — use them (including deleted pages so
        // templates referencing deleted pages can still show the original page name)
        const pages: CustomPage[] = objects.map((o, i) => ({
          id: o.id,
          name: o.name,
          iconId: DEFAULT_CUSTOM_ICON,
          createdAt: o.createdAt,
          sortOrder: i,
          deletedAt: o.isDeleted ? o.updatedAt : undefined,
        }));
        set((s) => ({ settings: { ...s.settings, customPages: pages } }));
        return;
      }

      // No pages in objects table — check for old-format pages from preferences
      const oldPages = get().settings.customPages;
      if (oldPages.length > 0) {
        // Migrate each old page into the objects table
        const migrated: CustomPage[] = [];
        for (const p of oldPages) {
          try {
            await invoke('object_create', {
              input: {
                accountId,
                name: p.name,
                collectionType: 'page',
                iconName: p.iconId || DEFAULT_CUSTOM_ICON,
                properties: {},
              },
            });
            migrated.push(p);
          } catch {
            // If migration fails for one, skip it but continue
          }
        }
        if (migrated.length > 0) {
          set((s) => ({ settings: { ...s.settings, customPages: migrated } }));
          // Clear old-format pages from preferences
          try {
            await invoke('user_data_update_preference', {
              payload: { accountId, preferences: { customPages: [] } },
            });
          } catch { /* silent */ }
        }
      }
    } catch {
      // objects table might be empty — keep whatever loadSettings found
    }
  },

  updateSetting: async (accountId, key, value) => {
    const oldValue = get().settings[key];
    set((s) => ({ settings: { ...s.settings, [key]: value } }));
    try {
      await invoke('user_data_update_preference', {
        payload: { accountId, preferences: { [key]: value } },
      });
      if (key === 'language' && typeof value === 'string') {
        await i18next.changeLanguage(value);
        // Sync to plaintext UI prefs so backend can read the current language immediately
        invoke('ui_update_preference', { key: 'language', value }).catch(() => {});
      }
    } catch {
      set((s) => ({ settings: { ...s.settings, [key]: oldValue } }));
    }
  },

  addCustomPage: async (accountId, name, iconId) => {
    const prevPages = get().settings.customPages;
    const id = crypto.randomUUID();
    const newPage: CustomPage = {
      id,
      name,
      iconId: iconId ?? DEFAULT_CUSTOM_ICON,
      createdAt: new Date().toISOString(),
      sortOrder: prevPages.length,
    };
    // Optimistic UI update
    set((s) => ({ settings: { ...s.settings, customPages: [...prevPages, newPage] } }));
    try {
      // P0-1: Store in objects table (not preferences JSON)
      await invoke('object_create', {
        input: {
          accountId,
          name,
          collectionType: 'page',
          iconName: iconId ?? DEFAULT_CUSTOM_ICON,
          properties: {},
        },
      });
    } catch {
      // Rollback
      set((s) => ({ settings: { ...s.settings, customPages: prevPages } }));
    }
    return newPage;
  },

  removeCustomPage: async (accountId, pageId) => {
    const prevPages = get().settings.customPages;
    const now = new Date().toISOString();
    // Mark as deleted locally (keep in array so templates can still reference the name)
    const pages = prevPages.map((p) =>
      p.id === pageId ? { ...p, deletedAt: now } : p
    );
    set((s) => ({ settings: { ...s.settings, customPages: pages } }));
    try {
      // P0-1: Use page_delete to create a "page" type trash item
      await invoke('page_delete', { accountId, sectionType: 'custom', pageObjectId: pageId });
    } catch {
      set((s) => ({ settings: { ...s.settings, customPages: prevPages } }));
    }
  },

  clearOnVaultLock: () => set({ settings: DEFAULT_SETTINGS, isLoading: false }),
}));
