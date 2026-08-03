import { create } from 'zustand';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { z } from 'zod';
import i18next, { detectSystemLanguage } from '@/lib/i18n';
import type { TrashRetentionPeriod } from '@/stores/trashStore';
import { applyTheme } from '@/lib/theme';
import { DEFAULT_CUSTOM_ICON } from '@/lib/pageIcons';
import { ST_UI_PREFS } from '@/lib/constants';
import { logger } from '@/lib/logger';

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
  /** 切到后台/锁屏时是否立即自动锁定（默认关闭） */
  autoLockOnBackground: boolean;
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
    autoLockOnBackground: z.boolean().optional(),
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
  autoLockOnBackground: false,
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

// ─── P062: 设置四副本写入路径矩阵 ────────────────────────────────────────
// 主题/语言/accent 等 UI 设置在以下四个存储位置各存一份，任一写入遗漏都会
// 造成「登录页主题正确但解锁后主题跳变」类 bug。改动任一写入时请对照本矩阵。
//
// | 副本                     | 位置                                        | 写入点                                              |
// |--------------------------|---------------------------------------------|-----------------------------------------------------|
// | ① zustand store（主态）  | settingsStore.settings                     | loadUiPreferences / loadSettings / updateSetting /  |
// |                          |                                             | addCustomPage / removeCustomPage / clearOnVaultLock |
// | ② localStorage 缓存     | ST_UI_PREFS（theme/accent/…）              | writeUiPrefsCache()（唯一写入点，P129 集中化：       |
// |                          |                                             |  loadUiPreferences Step2 + updateSetting 共用）；   |
// |                          |                                             | i18nextLng 在 updateSetting(language) setItem       |
// | ③ ui_preferences.json   | 明文文件（登录前即可读，修复登录页主题）    | syncPlaintextPref()（唯一写入点，P129 集中化：      |
// |  （明文）               |                                             |  loadSettings 循环 + updateSetting UI 键共用）      |
// | ④ vault 加密 preferences| 账户级加密 JSON（user_data_get/update）     | updateSetting → user_data_update_preference         |
// |                          |                                             | loadCustomPages 迁移清理 customPages 也走此命令     |
//
// 读取优先级：登录前 loadUiPreferences（②③）保证主题正确；解锁后 loadSettings
// （④）以账户级加密偏好为准覆盖本地态。语言的实际生效由 initI18n() 经 Rust IPC
// 确认（zh-CN 验证通过），updateSetting 只负责用户显式切换。
//
// P129: ②③ 副本写入已代码级集中——任何页面/组件不再直接写 localStorage 或
// ui_preferences.json，统一走下方两个 helper，杜绝第 5 个漂移写入点。

/** ③ ui_preferences.json 明文副本涉及的键（含 language——initI18n 也读它）。 */
const PLAINTEXT_PREF_KEYS = new Set<string>([
  'theme',
  'accentColor',
  'language',
  'defaultLightTheme',
  'defaultDarkTheme',
]);

/** ② localStorage ST_UI_PREFS 缓存副本涉及的键（无 language——schema 不含）。 */
const CACHE_PREF_KEYS = new Set<string>([
  'theme',
  'accentColor',
  'defaultLightTheme',
  'defaultDarkTheme',
]);

/**
 * ② 写入 localStorage ST_UI_PREFS 缓存（P129 唯一写入点）。
 * 缓存缺失/损坏仅影响登录页首帧主题，不得影响主流程。
 */
function writeUiPrefsCache(settings: AppSettings): void {
  try {
    localStorage.setItem(
      ST_UI_PREFS,
      JSON.stringify({
        theme: settings.theme,
        accentColor: settings.accentColor,
        defaultLightTheme: settings.defaultLightTheme,
        defaultDarkTheme: settings.defaultDarkTheme,
      }),
    );
  } catch (e) {
    logger.warn('[settingsStore] Failed to cache UI prefs:', e);
  }
}

/**
 * ③ 写入 ui_preferences.json 明文副本（P129 唯一写入点）。
 * 明文副本缺失仅影响登录页主题预加载，失败记日志即可，不阻断主流程。
 *
 * 导出供 App/index.tsx（hasSeenOnboarding）与 lib/notification.ts
 * （notificationPermissionRequested）等跨模块 UI 偏好写入复用——N-8：
 * 两处原绕过 helper 直写③，现收敛到本唯一写入点。
 */
export async function syncPlaintextPref(key: string, value: unknown): Promise<void> {
  try {
    await invoke('ui_update_preference', { key, value });
  } catch (e) {
    logger.warn('[settingsStore] Failed to sync UI pref:', key, e);
  }
}
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
      logger.warn('[settingsStore] Failed to load cached UI prefs:', e);
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
      // P129: ② 副本写入收敛到 writeUiPrefsCache（唯一写入点）
      writeUiPrefsCache(parsed);
      // Language is set by initI18n() via Rust IPC (confirmed working = zh-CN).
      // User changes via settings are applied in updateSetting() — skip here to avoid
      // overwriting correct IPC detection with stale/stored values from vault.
      // Theme/accent/bg are safe to apply immediately.
    } catch (e) {
      logger.warn('[settingsStore] No ui_preferences file yet:', e);
    }
  },

  loadSettings: async (accountId) => {
    set({ isLoading: true });
    try {
      const raw = await invoke<unknown>('user_data_get_preferences', { accountId: accountId });
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
      if (typeof prefs.autoLockOnBackground === 'boolean')
        parsed.autoLockOnBackground = prefs.autoLockOnBackground;
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
      // Sync UI prefs to plaintext file so next startup shows correct theme.
      // P129: ③ 副本写入收敛到 syncPlaintextPref（唯一写入点），原 5 段顺序 if 收敛为循环。
      for (const key of PLAINTEXT_PREF_KEYS) {
        const v = parsed[key as keyof AppSettings];
        if (v !== undefined && v !== '') {
          await syncPlaintextPref(key, v);
        }
      }
    } catch (e) {
      logger.error('[settingsStore] Failed to load settings:', e);
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
          // P053: object_list 的 ObjectSummary 已含完整解密 properties，
          // 直接读取 description，消除对每个页面单独 object_get 的 N+1 IPC。
          properties?: Record<string, unknown>;
        }>
      >('object_list', { accountId: accountId, filter: { collectionType: 'page', includeDeleted: true } });
      if (objects.length > 0) {
        // New-format pages exist in objects table — use them (including deleted pages so
        // templates referencing deleted pages can still show the original page name)
        const pages: CustomPage[] = objects.map((o, i) => {
          // 原实现仅对非 deleted 页面拉取 description，保持行为等价
          const desc = !o.isDeleted ? o.properties?.description : undefined;
          return {
            id: o.id,
            name: o.name,
            iconId: o.iconName || DEFAULT_CUSTOM_ICON,
            description: typeof desc === 'string' ? desc : undefined,
            createdAt: o.createdAt,
            sortOrder: i,
            deletedAt: o.isDeleted ? o.updatedAt : undefined,
          };
        });
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
            logger.warn('[settingsStore] Failed to migrate custom page:', p.name, e);
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
            logger.warn('[settingsStore] Failed to clear old-format custom pages:', e);
          }
        }
      }
    } catch (e) {
      logger.warn('[settingsStore] Failed to load custom pages:', e);
    }
  },

  updateSetting: async (accountId, key, value) => {
    const oldValue = get().settings[key];
    set((s) => ({ settings: { ...s.settings, [key]: value } }));
    try {
      await invoke('user_data_update_preference', {
        payload: { accountId, preferences: { [key]: value } },
      });
      // P129: UI 键变更时同步 ②③ 副本（唯一写入点，页面不再各自写）。
      // ④ vault 写入成功后才触发，失败回滚时不会产生副本漂移。
      if (PLAINTEXT_PREF_KEYS.has(key)) {
        void syncPlaintextPref(key, value);
        if (CACHE_PREF_KEYS.has(key)) {
          writeUiPrefsCache(get().settings);
        }
      }
      if (key === 'language' && typeof value === 'string') {
        await i18next.changeLanguage(value);
        // ③ 已由上方 PLAINTEXT_PREF_KEYS 分支同步；此处仅补 ② i18nextLng 冷启动缓存。
        try {
          localStorage.setItem('i18nextLng', value);
        } catch (e) {
          logger.warn('[settingsStore] Failed to cache language:', e);
        }
      }
    } catch (e) {
      logger.warn('[settingsStore] Failed to update setting:', key, e);
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
      logger.warn('[settingsStore] Failed to add custom page:', name, e);
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
      await invoke('page_delete', { accountId: accountId, sectionType: pageId, pageObjectId: pageId });
    } catch (e) {
      logger.warn('[settingsStore] Failed to remove custom page:', pageId, e);
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
        autoLockOnBackground: state.settings.autoLockOnBackground,
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
