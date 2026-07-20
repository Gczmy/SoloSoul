import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import { z } from 'zod';
import i18next, { detectSystemLanguage } from '@/lib/i18n';
import type { TrashRetentionPeriod } from '@/stores/trashStore';
import { applyTheme } from '@/lib/theme';
import { DEFAULT_CUSTOM_ICON } from '@/lib/pageIcons';
import { ST_UI_PREFS } from '@/lib/constants';

// 9.8.3 — Custom page data structure
// Custom pages are now stored in the objects table (P0-1), not in preferences.
// iconId references CUSTOM_ICON_MAP from src/lib/pageIcons.ts (Single Source of Truth)
export interface CustomPage {
  id: string;
  name: string;
  iconId: string;
  description?: string;
  createdAt: string;
  sortOrder: number;
  deletedAt?: string;
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
  /** 自动锁定后是否发送系统通知（默认关闭） */
  autoLockNotificationEnabled: boolean;
  /** 备份提醒周期（天），≤0 表示关闭 */
  backupReminderDays: number;
  /** 上次备份提醒的时间戳（毫秒），null 表示从未提醒过 */
  lastBackupReminderAt: number | null;
  biometricEnabled: boolean;
  confirmDelete: boolean;
  customPages: CustomPage[];
  defaultLightTheme: string;
  defaultDarkTheme: string;
  sidebarPosition: 'left' | 'right' | 'top' | 'bottom';
  /** Per-button mode: 'card' (floating panel) or 'page' (navigate to dedicated page) */
  sidebarButtonModes: Record<string, 'card' | 'page'>;
  trashRetention: TrashRetentionPeriod;
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
  addCustomPage: (
    accountId: string,
    name: string,
    iconId?: string,
    description?: string,
  ) => Promise<CustomPage>;
  removeCustomPage: (accountId: string, pageId: string) => Promise<void>;
}

// F032: validate the localStorage UI prefs cache with a strict schema.
const uiPrefsSchema = z.object({
  theme: z.enum(['light', 'dark', 'system']).optional(),
  accentColor: z.enum(['ocean', 'amber', 'forest', 'rose', 'purple', 'custom']).optional(),
  defaultLightTheme: z.string().optional(),
  defaultDarkTheme: z.string().optional(),
});

const customPageSchema = z.object({
  id: z.string(),
  name: z.string(),
  iconId: z.string(),
  description: z.string().optional(),
  createdAt: z.string(),
  sortOrder: z.number(),
  deletedAt: z.string().optional(),
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
    autoLockNotificationEnabled: z.boolean().optional(),
    backupReminderDays: z.number().optional(),
    lastBackupReminderAt: z.number().nullable().optional(),
    biometricEnabled: z.boolean().optional(),
    confirmDelete: z.boolean().optional(),
    sidebarPosition: z.enum(['left', 'right', 'top', 'bottom']).optional(),
    trashRetention: z.enum(['30d', '60d', 'half_year', 'one_year', 'never']).optional(),
    customPages: z.array(customPageSchema).optional(),
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
  autoLockNotificationEnabled: false,
  backupReminderDays: 7,
  lastBackupReminderAt: null,
  biometricEnabled: false,
  confirmDelete: true,
  customPages: [],
  defaultLightTheme: 'warm-stone',
  defaultDarkTheme: 'warm-stone-dark',
  sidebarPosition: 'left',
  sidebarButtonModes: {
    ocr: 'card',
    plugins: 'card',
    ai_chat: 'card',
    search: 'card',
  },
  trashRetention: '30d',
};

export const useSettingsStore = create<SettingsState>((set, get) => ({
  settings: DEFAULT_SETTINGS,
  isLoading: false,

  /** Load UI-only prefs: read localStorage cache sync first (instant),
   *  then refresh from IPC asynchronously. */
  loadUiPreferences: async () => {
    // Step 1: apply cached prefs instantly from localStorage
    try {
      const raw = localStorage.getItem(ST_UI_PREFS);
      if (raw) {
        const parsed = uiPrefsSchema.safeParse(JSON.parse(raw));
        if (parsed.success) {
          const cached = parsed.data;
          const p = { ...get().settings };
          if (cached.theme) p.theme = cached.theme;
          if (cached.accentColor) p.accentColor = cached.accentColor;
          if (cached.defaultLightTheme) p.defaultLightTheme = cached.defaultLightTheme;
          if (cached.defaultDarkTheme) p.defaultDarkTheme = cached.defaultDarkTheme;
          await applyTheme({
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
    } catch (e) {
      console.warn('[settingsStore] Failed to load cached UI prefs:', e);
    }

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
      await applyTheme({
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
          ST_UI_PREFS,
          JSON.stringify({
            theme: parsed.theme,
            accentColor: parsed.accentColor,
            defaultLightTheme: parsed.defaultLightTheme,
            defaultDarkTheme: parsed.defaultDarkTheme,
          }),
        );
      } catch (e) {
        console.warn('[settingsStore] Failed to cache UI prefs:', e);
      }
      // Language is set by initI18n() via Rust IPC (confirmed working = zh-CN).
      // User changes via settings are applied in updateSetting() — skip here to avoid
      // overwriting correct IPC detection with stale/stored values from vault.
      // Theme/accent/bg are safe to apply immediately.
    } catch (e) {
      console.warn('[settingsStore] No ui_preferences file yet:', e);
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
      if (typeof prefs.autoLockNotificationEnabled === 'boolean')
        parsed.autoLockNotificationEnabled = prefs.autoLockNotificationEnabled;
      if (typeof prefs.backupReminderDays === 'number')
        parsed.backupReminderDays = prefs.backupReminderDays;
      if (typeof (prefs as Record<string, unknown>).lastBackupReminderAt === 'number')
        parsed.lastBackupReminderAt = (prefs as Record<string, unknown>).lastBackupReminderAt as number;
      if (prefs.trashRetention) parsed.trashRetention = prefs.trashRetention;
      if (typeof prefs.biometricEnabled === 'boolean')
        parsed.biometricEnabled = prefs.biometricEnabled;
      if (typeof prefs.confirmDelete === 'boolean') parsed.confirmDelete = prefs.confirmDelete;
      if (prefs.sidebarPosition) parsed.sidebarPosition = prefs.sidebarPosition;
      // sidebarButtonModes is stored in preferences; load it if present
      const storedModes = (raw as Record<string, unknown>)?.sidebarButtonModes;
      if (storedModes && typeof storedModes === 'object') {
        for (const [key, val] of Object.entries(storedModes)) {
          if (val === 'card' || val === 'page') {
            parsed.sidebarButtonModes[key] = val;
          }
        }
      }
      // Load old-format customPages from preferences for migration.
      // Once loaded, also try the new objects-table source via loadCustomPages().
      if (prefs.customPages) parsed.customPages = prefs.customPages;
      set({ settings: parsed, isLoading: false });
      // Sync UI prefs to plaintext file so next startup shows correct theme
      try {
        if (parsed.theme)
          await invoke('ui_update_preference', { key: 'theme', value: parsed.theme });
        if (parsed.accentColor)
          await invoke('ui_update_preference', { key: 'accentColor', value: parsed.accentColor });
        if (parsed.language)
          await invoke('ui_update_preference', { key: 'language', value: parsed.language });
        if (parsed.defaultLightTheme)
          await invoke('ui_update_preference', {
            key: 'defaultLightTheme',
            value: parsed.defaultLightTheme,
          });
        if (parsed.defaultDarkTheme)
          await invoke('ui_update_preference', {
            key: 'defaultDarkTheme',
            value: parsed.defaultDarkTheme,
          });
      } catch (e) {
        console.warn('[settingsStore] Failed to sync UI prefs:', e);
      }
    } catch (e) {
      console.error('[settingsStore] Failed to load settings:', e);
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
        const pages: CustomPage[] = await Promise.all(
          objects.map(async (o, i) => {
            let description: string | undefined;
            if (!o.isDeleted) {
              try {
                const detail = await invoke<{ properties?: Record<string, unknown> } | null>(
                  'object_get',
                  { accountId, objectId: o.id },
                );
                const desc = detail?.properties?.description;
                if (typeof desc === 'string') {
                  description = desc;
                }
              } catch (e) {
                console.warn('[settingsStore] Failed to load page description:', o.id, e);
              }
            }
            return {
              id: o.id,
              name: o.name,
              iconId: o.iconName || DEFAULT_CUSTOM_ICON,
              description,
              createdAt: o.createdAt,
              sortOrder: i,
              deletedAt: o.isDeleted ? o.updatedAt : undefined,
            };
          }),
        );
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
          } catch (e) {
            console.warn('[settingsStore] Failed to migrate custom page:', p.name, e);
          }
        }
        if (migrated.length > 0) {
          set((s) => ({ settings: { ...s.settings, customPages: migrated } }));
          // Clear old-format pages from preferences
          try {
            await invoke('user_data_update_preference', {
              payload: { accountId, preferences: { customPages: [] } },
            });
          } catch (e) {
            console.warn('[settingsStore] Failed to clear old-format custom pages:', e);
          }
        }
      }
    } catch (e) {
      console.warn('[settingsStore] Failed to load custom pages:', e);
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
        invoke('ui_update_preference', { key: 'language', value }).catch((e) =>
          console.warn('[settingsStore] Failed to sync language:', e),
        );
        // Persist to localStorage for next cold launch
        try {
          localStorage.setItem('i18nextLng', value);
        } catch (e) {
          console.warn('[settingsStore] Failed to cache language:', e);
        }
      }
    } catch (e) {
      console.warn('[settingsStore] Failed to update setting:', key, e);
      set((s) => ({ settings: { ...s.settings, [key]: oldValue } }));
    }
  },

  addCustomPage: async (accountId, name, iconId, description) => {
    const prevPages = get().settings.customPages;
    const id = crypto.randomUUID();
    const newPage: CustomPage = {
      id,
      name,
      iconId: iconId ?? DEFAULT_CUSTOM_ICON,
      description,
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
          properties: description ? { description } : {},
          id,
        },
      });
    } catch (e) {
      console.warn('[settingsStore] Failed to add custom page:', name, e);
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
      // sectionType must be the actual page UUID so that page_delete's sub-object
      // matching (section_type == section_type || collection_type == section_type)
      // correctly finds all child objects assigned to this custom page.
      await invoke('page_delete', { accountId, sectionType: pageId, pageObjectId: pageId });
    } catch (e) {
      console.warn('[settingsStore] Failed to remove custom page:', pageId, e);
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
        autoLockNotificationEnabled: state.settings.autoLockNotificationEnabled,
        backupReminderDays: state.settings.backupReminderDays,
        lastBackupReminderAt: state.settings.lastBackupReminderAt,
        defaultLightTheme: state.settings.defaultLightTheme,
        defaultDarkTheme: state.settings.defaultDarkTheme,
        sidebarPosition: state.settings.sidebarPosition,
        sidebarButtonModes: state.settings.sidebarButtonModes,
      },
      isLoading: false,
    })),
}));
