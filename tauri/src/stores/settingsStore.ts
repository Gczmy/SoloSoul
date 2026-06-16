import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import { z } from 'zod';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { PhysicalSize } from '@tauri-apps/api/dpi';
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

export interface WindowSize {
  width: number;
  height: number;
}

export interface AppSettings {
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
  sidebarPosition: 'left' | 'right' | 'top' | 'bottom';
  sidebarBottomActions: [string, string, string];
  windowSize?: WindowSize;
}

interface SettingsState {
  settings: AppSettings;
  isLoading: boolean;

  /** Load UI-only prefs (theme/language/accent) from plaintext ui_preferences.json.
   *  Can be called before Vault unlock — fixes login page theme bug. */
  loadUiPreferences: () => Promise<void>;
  loadSettings: (accountId: string) => Promise<void>;
  loadCustomPages: (accountId: string) => Promise<void>;
  updateSetting: <K extends keyof AppSettings>(
    accountId: string,
    key: K,
    value: AppSettings[K],
  ) => Promise<void>;
  clearOnVaultLock: () => void;
  addCustomPage: (accountId: string, name: string, iconId?: string) => Promise<CustomPage>;
  removeCustomPage: (accountId: string, pageId: string) => Promise<void>;
}

// F032: validate the localStorage UI prefs cache with a strict schema.
const uiPrefsSchema = z.object({
  theme: z.enum(['light', 'dark', 'system']).optional(),
  accentColor: z.enum(['ocean', 'amber', 'forest', 'rose', 'purple', 'custom']).optional(),
  defaultLightTheme: z.string().optional(),
  defaultDarkTheme: z.string().optional(),
  windowSize: z
    .object({
      width: z.number(),
      height: z.number(),
    })
    .optional(),
});

const customPageSchema = z.object({
  id: z.string(),
  name: z.string(),
  iconId: z.string(),
  createdAt: z.string(),
  sortOrder: z.number(),
  deletedAt: z.string().optional(),
});

const windowSizeSchema = z.object({
  width: z.number(),
  height: z.number(),
});

const accountPrefsSchema = z
  .object({
    theme: z.enum(['light', 'dark', 'system']).optional(),
    accentColor: z.enum(['ocean', 'amber', 'forest', 'rose', 'purple', 'custom']).optional(),
    defaultLightTheme: z.string().optional(),
    defaultDarkTheme: z.string().optional(),
    customAccentHex: z.string().optional(),
    backgroundType: z.enum(['solid', 'gradient', 'image']).optional(),
    backgroundValue: z.string().optional(),
    language: z.enum(['zh-CN', 'en-US']).optional(),
    locale: z.string().optional(),
    autoLockTimeoutMinutes: z.number().optional(),
    biometricEnabled: z.boolean().optional(),
    confirmDelete: z.boolean().optional(),
    sidebarPosition: z.enum(['left', 'right', 'top', 'bottom']).optional(),
    sidebarBottomActions: z.tuple([z.string(), z.string(), z.string()]).optional(),
    customPages: z.array(customPageSchema).optional(),
    windowSize: windowSizeSchema.optional(),
  })
  .passthrough();

type AccountPrefs = z.infer<typeof accountPrefsSchema>;

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
  sidebarPosition: 'left',
  sidebarBottomActions: ['search', 'plugins', 'ai_chat'],
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
        const parsed = uiPrefsSchema.safeParse(JSON.parse(raw));
        if (parsed.success) {
          const cached = parsed.data;
          const p = { ...get().settings };
          if (cached.theme) p.theme = cached.theme;
          if (cached.accentColor) p.accentColor = cached.accentColor;
          if (cached.defaultLightTheme) p.defaultLightTheme = cached.defaultLightTheme;
          if (cached.defaultDarkTheme) p.defaultDarkTheme = cached.defaultDarkTheme;
          if (cached.windowSize) {
            p.windowSize = cached.windowSize;
            // Do not mirror back to solosoul_window_size here; restoreWindowSize()
            // owns that key and it may be newer than the ui_prefs snapshot.
          }
          applyTheme({
            preset:
              p.theme === 'dark'
                ? 'warm-stone-dark'
                : p.theme === 'light'
                  ? 'warm-stone-light'
                  : 'system',
            accentColor: p.accentColor,
            backgroundType: 'solid',
            backgroundValue: '',
            defaultLightTheme: p.defaultLightTheme,
            defaultDarkTheme: p.defaultDarkTheme,
          });
          set({ settings: p });
        }
      }
    } catch {
      /* ignore */
    }

    // Step 2: fetch fresh prefs from IPC (slow, async)
    try {
      const prefs = await invoke<{
        theme?: string;
        accentColor?: string;
        language?: string;
        defaultLightTheme?: string;
        defaultDarkTheme?: string;
        windowSize?: WindowSize;
      }>('ui_get_preferences');
      const parsed = { ...get().settings };
      if (prefs.theme) parsed.theme = prefs.theme as AppSettings['theme'];
      if (prefs.accentColor) parsed.accentColor = prefs.accentColor as AppSettings['accentColor'];
      if (prefs.language) parsed.language = prefs.language;
      if (prefs.defaultLightTheme) parsed.defaultLightTheme = prefs.defaultLightTheme;
      if (prefs.defaultDarkTheme) parsed.defaultDarkTheme = prefs.defaultDarkTheme;
      if (
        prefs.windowSize &&
        typeof prefs.windowSize.width === 'number' &&
        typeof prefs.windowSize.height === 'number'
      ) {
        // Only fall back to the on-disk UI preference when there is no localStorage
        // cache. The cache is updated synchronously on every resize, so it is always
        // at least as fresh as the debounced disk write.
        const hasCachedSize = !!localStorage.getItem('solosoul_window_size');
        if (!hasCachedSize) {
          parsed.windowSize = prefs.windowSize;
          try {
            localStorage.setItem('solosoul_window_size', JSON.stringify(prefs.windowSize));
          } catch {
            /* ignore */
          }
        }
      }
      applyTheme({
        preset:
          parsed.theme === 'dark'
            ? 'warm-stone-dark'
            : parsed.theme === 'light'
              ? 'warm-stone-light'
              : 'system',
        accentColor: parsed.accentColor,
        backgroundType: 'solid',
        backgroundValue: '',
        defaultLightTheme: parsed.defaultLightTheme,
        defaultDarkTheme: parsed.defaultDarkTheme,
      });
      set({ settings: parsed });
      try {
        localStorage.setItem(
          'solosoul_ui_prefs',
          JSON.stringify({
            theme: parsed.theme,
            accentColor: parsed.accentColor,
            defaultLightTheme: parsed.defaultLightTheme,
            defaultDarkTheme: parsed.defaultDarkTheme,
            windowSize: parsed.windowSize,
          }),
        );
      } catch {
        /* ignore */
      }
      // Language is set by initI18n() via Rust IPC (confirmed working = zh-CN).
      // User changes via settings are applied in updateSetting() — skip here to avoid
      // overwriting correct IPC detection with stale/stored values from vault.
      // Theme/accent/bg are safe to apply immediately.
    } catch {
      /* no ui_preferences file yet */
    }
  },

  loadSettings: async (accountId) => {
    set({ isLoading: true });
    try {
      const raw = await invoke<unknown>('user_data_get_preferences', { accountId });
      const parsedPrefsResult = accountPrefsSchema.safeParse(raw);
      const prefs: AccountPrefs = parsedPrefsResult.success ? parsedPrefsResult.data : {};
      const parsed = { ...DEFAULT_SETTINGS };
      if (prefs.theme) parsed.theme = prefs.theme;
      if (prefs.accentColor) parsed.accentColor = prefs.accentColor;
      if (prefs.defaultLightTheme) parsed.defaultLightTheme = prefs.defaultLightTheme;
      if (prefs.defaultDarkTheme) parsed.defaultDarkTheme = prefs.defaultDarkTheme;
      if (prefs.customAccentHex) parsed.customAccentHex = prefs.customAccentHex;
      if (prefs.backgroundType) parsed.backgroundType = prefs.backgroundType;
      if (prefs.backgroundValue) parsed.backgroundValue = prefs.backgroundValue;
      if (prefs.language) parsed.language = prefs.language;
      if (prefs.locale) parsed.locale = prefs.locale;
      if (typeof prefs.autoLockTimeoutMinutes === 'number')
        parsed.autoLockTimeoutMinutes = prefs.autoLockTimeoutMinutes;
      if (typeof prefs.biometricEnabled === 'boolean') parsed.biometricEnabled = prefs.biometricEnabled;
      if (typeof prefs.confirmDelete === 'boolean') parsed.confirmDelete = prefs.confirmDelete;
      if (prefs.sidebarPosition) parsed.sidebarPosition = prefs.sidebarPosition;
      if (prefs.sidebarBottomActions) parsed.sidebarBottomActions = prefs.sidebarBottomActions;
      // Load old-format customPages from preferences for migration.
      // Once loaded, also try the new objects-table source via loadCustomPages().
      if (prefs.customPages) parsed.customPages = prefs.customPages;
      // Window size: plaintext UI preference / localStorage cache is the freshest source
      // because it is updated synchronously on every resize. Prefer it over the encrypted
      // account preference to avoid reverting to a stale size after login.
      let effectiveWindowSize: WindowSize | undefined;
      try {
        const cachedRaw = localStorage.getItem('solosoul_window_size');
        if (cachedRaw) {
          const cached = windowSizeSchema.safeParse(JSON.parse(cachedRaw));
          if (cached.success) effectiveWindowSize = cached.data;
        }
      } catch {
        /* ignore */
      }
      if (!effectiveWindowSize) {
        effectiveWindowSize = prefs.windowSize;
      }
      if (
        effectiveWindowSize &&
        typeof effectiveWindowSize.width === 'number' &&
        typeof effectiveWindowSize.height === 'number'
      ) {
        parsed.windowSize = effectiveWindowSize;
        try {
          const window = getCurrentWindow();
          const current = await window.innerSize();
          if (
            Math.abs(current.width - effectiveWindowSize.width) > 1 ||
            Math.abs(current.height - effectiveWindowSize.height) > 1
          ) {
            await window.setSize(new PhysicalSize(effectiveWindowSize));
          }
        } catch {
          /* ignore */
        }
        // Sync the effective size back to encrypted account prefs if it differs from what was stored.
        const encryptedWindowSize = prefs.windowSize as WindowSize | undefined;
        if (
          !encryptedWindowSize ||
          encryptedWindowSize.width !== effectiveWindowSize.width ||
          encryptedWindowSize.height !== effectiveWindowSize.height
        ) {
          invoke('user_data_update_preference', {
            payload: { accountId, preferences: { windowSize: effectiveWindowSize } },
          }).catch(() => {});
        }
      }
      set({ settings: parsed, isLoading: false });
      // Sync UI prefs to plaintext file so next startup shows correct theme
      if (parsed.theme)
        invoke('ui_update_preference', { key: 'theme', value: parsed.theme }).catch(() => {});
      if (parsed.accentColor)
        invoke('ui_update_preference', { key: 'accentColor', value: parsed.accentColor }).catch(
          () => {},
        );
      if (parsed.language)
        invoke('ui_update_preference', { key: 'language', value: parsed.language }).catch(() => {});
      if (parsed.defaultLightTheme)
        invoke('ui_update_preference', {
          key: 'defaultLightTheme',
          value: parsed.defaultLightTheme,
        }).catch(() => {});
      if (parsed.defaultDarkTheme)
        invoke('ui_update_preference', {
          key: 'defaultDarkTheme',
          value: parsed.defaultDarkTheme,
        }).catch(() => {});
    } catch {
      set({ isLoading: false });
    }
  },

  /** Load custom pages from the objects table (P0-1 — objects storage layer).
   *  If the objects table has pages, use those (new format).
   *  If empty but old-format pages exist in preferences, migrate them automatically. */
  loadCustomPages: async (accountId) => {
    try {
      const objects = await invoke<
        Array<{
          id: string;
          name: string;
          collectionType: string;
          iconName?: string;
          createdAt: string;
          updatedAt: string;
          isDeleted?: boolean;
        }>
      >('object_list', { accountId, filter: { collectionType: 'page', includeDeleted: true } });
      if (objects.length > 0) {
        // New-format pages exist in objects table — use them (including deleted pages so
        // templates referencing deleted pages can still show the original page name)
        const pages: CustomPage[] = objects.map((o, i) => ({
          id: o.id,
          name: o.name,
          iconId: o.iconName || DEFAULT_CUSTOM_ICON,
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
          } catch {
            /* silent */
          }
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
      // Window size is a non-sensitive UI preference that must be available before login.
      if (key === 'windowSize') {
        try {
          localStorage.setItem('solosoul_window_size', JSON.stringify(value));
        } catch {
          /* ignore */
        }
        await invoke('ui_update_preference', {
          key: 'windowSize',
          value: JSON.stringify(value),
        });
      } else {
        await invoke('user_data_update_preference', {
          payload: { accountId, preferences: { [key]: value } },
        });
      }
      if (key === 'language' && typeof value === 'string') {
        await i18next.changeLanguage(value);
        // Sync to plaintext UI prefs so backend can read the current language immediately
        invoke('ui_update_preference', { key: 'language', value }).catch(() => {});
        // Persist to localStorage for next cold launch
        try {
          localStorage.setItem('i18nextLng', value);
        } catch {
          /* ignore */
        }
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
      // Pass the client-generated id so frontend state stays in sync with the database record.
      await invoke('object_create', {
        input: {
          accountId,
          name,
          collectionType: 'page',
          iconName: iconId ?? DEFAULT_CUSTOM_ICON,
          properties: {},
          id,
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
    const pages = prevPages.map((p) => (p.id === pageId ? { ...p, deletedAt: now } : p));
    set((s) => ({ settings: { ...s.settings, customPages: pages } }));
    try {
      // P0-1: Use page_delete to create a "page" type trash item
      await invoke('page_delete', { accountId, sectionType: 'custom', pageObjectId: pageId });
    } catch {
      set((s) => ({ settings: { ...s.settings, customPages: prevPages } }));
    }
  },

  clearOnVaultLock: () =>
    set((state) => ({
      // Keep UI-only preferences so lock screen retains user's language/theme/accent
      settings: {
        ...DEFAULT_SETTINGS,
        theme: state.settings.theme,
        accentColor: state.settings.accentColor,
        customAccentHex: state.settings.customAccentHex,
        backgroundType: state.settings.backgroundType,
        backgroundValue: state.settings.backgroundValue,
        language: state.settings.language,
        locale: state.settings.locale,
        autoLockTimeoutMinutes: state.settings.autoLockTimeoutMinutes,
        defaultLightTheme: state.settings.defaultLightTheme,
        defaultDarkTheme: state.settings.defaultDarkTheme,
        sidebarPosition: state.settings.sidebarPosition,
        sidebarBottomActions: state.settings.sidebarBottomActions,
        windowSize: state.settings.windowSize,
      },
      isLoading: false,
    })),
}));
