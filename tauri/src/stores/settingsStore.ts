import { createSessionRequests, onRequestSessionChange } from '@/lib/sessionRequests';
import { withTimeout } from '@/lib/withTimeout';
import { getSchemeById } from '@/lib/themeSchemes';
import { create } from 'zustand';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { z } from 'zod';
import i18next, { detectSystemLanguage } from '@/lib/i18n';
import type { TrashRetentionPeriod } from '@/stores/trashStore';
import { applyTheme } from '@/lib/theme';
import { DEFAULT_CUSTOM_ICON } from '@/lib/pageIcons';
import { ST_UI_PREFS } from '@/lib/constants';
import { logger } from '@/lib/logger';
import { isAndroidGlassMode, type AndroidGlassMode } from '@/lib/androidGlass';

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
  /** 安卓动效偏好，系统减少动态效果仍优先生效。 */
  reduceMotion: boolean;
  /** Android 材质偏好，系统能力不足时独立降级。 */
  androidGlass: AndroidGlassMode;
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
  /** 仅来自旧 preferences，独立于已落库的页面列表，供部分迁移重试。 */
  legacyCustomPages: CustomPage[];
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
  reduceMotion: z.boolean().optional(),
  androidGlass: z.enum(['off', 'local', 'enhanced']).optional(),
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
    reduceMotion: z.boolean().optional(),
    androidGlass: z.enum(['off', 'local', 'enhanced']).optional(),
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
  reduceMotion: false,
  androidGlass: 'local',
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
//
// P029: 读路径顺序与回跳残余——解锁前 loadUiPreferences 已把 ②③ 的 UI 偏好
// 应用到 store（主题/语言等登录页正确）；解锁后 loadSettings 以 vault ④ 为准覆盖。
// 若 vault ④ 缺失某 UI 键（旧版升级/未持久化），旧实现以 DEFAULT_SETTINGS 为合并
// 基准会把该键回跳回默认值（如暗色主题跳回 system），造成解锁瞬间主题闪烁回跳。
// 修复：合并基准改为「当前 settings + DEFAULT 兜底」——vault 有值的键仍以 vault
// 为准，缺失键沿用登录前已应用的缓存值，读路径不再产生回跳。

/** ③ ui_preferences.json 明文副本涉及的键（含 language——initI18n 也读它）。 */
const PLAINTEXT_PREF_KEYS = new Set<string>([
  'theme',
  'accentColor',
  'language',
  'defaultLightTheme',
  'defaultDarkTheme',
  'reduceMotion',
  'androidGlass',
]);

/** ② localStorage ST_UI_PREFS 缓存副本涉及的键（无 language——schema 不含）。 */
const CACHE_PREF_KEYS = new Set<string>([
  'theme',
  'accentColor',
  'defaultLightTheme',
  'defaultDarkTheme',
  'reduceMotion',
  'androidGlass',
]);

/**
 * ② 写入 localStorage ST_UI_PREFS 缓存（P129 唯一写入点）。
 * 缓存缺失/损坏仅影响登录页首帧主题，不得影响主流程。
 */
function writeUiPrefsCache(settings: AppSettings): void {
  try {
    const startupColors = (schemeId: string, fallback: string) => {
      const variables = (getSchemeById(schemeId) ?? getSchemeById(fallback))!.variables;
      return {
        background: variables['--bg-base'],
        foreground: variables['--text-primary'],
        secondary: variables['--text-secondary'],
      };
    };
    localStorage.setItem(
      ST_UI_PREFS,
      JSON.stringify({
        theme: settings.theme,
        accentColor: settings.accentColor,
        reduceMotion: settings.reduceMotion,
        androidGlass: settings.androidGlass,
        defaultLightTheme: settings.defaultLightTheme,
        defaultDarkTheme: settings.defaultDarkTheme,
        startupThemes: {
          light: startupColors(settings.defaultLightTheme, 'warm-stone'),
          dark: startupColors(settings.defaultDarkTheme, 'warm-stone-dark'),
        },
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
export async function syncPlaintextPref(
  key: string,
  value: unknown,
  requestIsCurrent?: () => boolean,
): Promise<void> {
  try {
    await invoke('ui_update_preference', { key, value }, { requestIsCurrent });
  } catch (e) {
    logger.warn('[settingsStore] Failed to sync UI pref:', key, e);
  }
}
const requests = createSessionRequests();

export const useSettingsStore = create<SettingsState>((set, get) => ({
  legacyCustomPages: [],
  settings: DEFAULT_SETTINGS,
  isLoading: false,

  /** Load UI-only prefs: read localStorage cache sync first (instant),
   *  then refresh from IPC asynchronously. */
  loadUiPreferences: async () => {
    const request = requests.begin('ui');
    const setCurrent = request.guardSet<SettingsState>(set);
    // Step 1: apply cached prefs instantly from localStorage
    try {
      const raw = localStorage.getItem(ST_UI_PREFS);
      if (raw) {
        const parsed = uiPrefsSchema.safeParse(JSON.parse(raw));
        if (parsed.success) {
          const cached = parsed.data;
          const p = { ...get().settings };
          if (typeof cached.reduceMotion === 'boolean') p.reduceMotion = cached.reduceMotion;
          if (isAndroidGlassMode(cached.androidGlass)) p.androidGlass = cached.androidGlass;
          if (cached.theme) p.theme = cached.theme;
          if (cached.accentColor) p.accentColor = cached.accentColor;
          if (cached.defaultLightTheme) p.defaultLightTheme = cached.defaultLightTheme;
          if (cached.defaultDarkTheme) p.defaultDarkTheme = cached.defaultDarkTheme;
          request.assertCurrent();
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
          request.assertCurrent();
          setCurrent({ settings: p });
        }
      }
    } catch (e) {
      if (!request.isCurrent()) return;
      logger.warn('[settingsStore] Failed to load cached UI prefs:', e);
    }

    // Step 2: fetch fresh prefs from IPC (slow, async)
    try {
      request.assertCurrent();
      const prefs = await withTimeout(
        request.invoke<{
          theme?: string;
          reduceMotion?: boolean;
          androidGlass?: AndroidGlassMode;
          accentColor?: string;
          language?: string;
          defaultLightTheme?: string;
          defaultDarkTheme?: string;
        }>('ui_get_preferences'),
        1200,
      );
      request.assertCurrent();
      const parsed = { ...get().settings };
      if (typeof prefs.reduceMotion === 'boolean') parsed.reduceMotion = prefs.reduceMotion;
      if (isAndroidGlassMode(prefs.androidGlass)) parsed.androidGlass = prefs.androidGlass;
      if (prefs.theme) parsed.theme = prefs.theme as AppSettings['theme'];
      if (prefs.accentColor) parsed.accentColor = prefs.accentColor as AppSettings['accentColor'];
      if (prefs.language) parsed.language = prefs.language;
      if (prefs.defaultLightTheme) parsed.defaultLightTheme = prefs.defaultLightTheme;
      if (prefs.defaultDarkTheme) parsed.defaultDarkTheme = prefs.defaultDarkTheme;
      request.assertCurrent();
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
      request.assertCurrent();
      setCurrent({ settings: parsed });
      // P129: ② 副本写入收敛到 writeUiPrefsCache（唯一写入点）
      writeUiPrefsCache(parsed);
      // Language is set by initI18n() via Rust IPC (confirmed working = zh-CN).
      // User changes via settings are applied in updateSetting() — skip here to avoid
      // overwriting correct IPC detection with stale/stored values from vault.
      // Theme/accent/bg are safe to apply immediately.
    } catch (e) {
      if (!request.isCurrent()) return;
      logger.warn('[settingsStore] No ui_preferences file yet:', e);
    }
  },

  loadSettings: async (accountId) => {
    const request = requests.begin('settings', accountId);
    const setCurrent = request.guardSet<SettingsState>(set);
    setCurrent({ isLoading: true });
    try {
      const raw = await request.invoke<unknown>('user_data_get_preferences', {
        accountId: accountId,
      });
      request.assertCurrent();
      const parsedPrefsResult = accountPrefsSchema.safeParse(raw);
      const prefs: AccountPrefs = parsedPrefsResult.success ? parsedPrefsResult.data : {};
      // P029: 合并基准为「当前 settings + DEFAULT 兜底」——vault ④ 缺失的键沿用
      // 登录前 loadUiPreferences 已应用的缓存值，避免解锁瞬间回跳默认（旧实现用
      // DEFAULT_SETTINGS 作基准，vault 缺键时把已设好的主题等打回默认）。
      // sidebarButtonModes 显式拷贝，避免下方原地赋值污染 store 中既有对象。
      const parsed: AppSettings = {
        ...DEFAULT_SETTINGS,
        ...get().settings,
        sidebarButtonModes: { ...get().settings.sidebarButtonModes },
      };
      if (typeof prefs.reduceMotion === 'boolean') parsed.reduceMotion = prefs.reduceMotion;
      if (isAndroidGlassMode(prefs.androidGlass)) parsed.androidGlass = prefs.androidGlass;
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
        parsed.lastBackupReminderAt = (prefs as Record<string, unknown>)
          .lastBackupReminderAt as number;
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
      setCurrent({
        settings: parsed,
        legacyCustomPages: prefs.customPages ?? [],
        isLoading: false,
      });
      // Sync UI prefs to plaintext file so next startup shows correct theme.
      // P129: ③ 副本写入收敛到 syncPlaintextPref（唯一写入点），原 5 段顺序 if 收敛为循环。
      for (const key of PLAINTEXT_PREF_KEYS) {
        const v = parsed[key as keyof AppSettings];
        if (v !== undefined && v !== '') {
          request.assertCurrent();
          await syncPlaintextPref(key, v, request.isCurrent);
          request.assertCurrent();
        }
      }
    } catch (e) {
      if (!request.isCurrent()) return;
      logger.error('[settingsStore] Failed to load settings:', e);
      setCurrent({ isLoading: false });
    }
  },

  /** 按稳定 ID 补齐旧页面；成功列表与待迁移来源分离，部分失败可再次加载重试。 */
  loadCustomPages: async (accountId) => {
    const request = requests.begin('pages', accountId);
    const setCurrent = request.guardSet<SettingsState>(set);
    try {
      const objects = await request.invoke<
        Array<{
          id: string;
          name: string;
          typeId: string;
          iconName?: string;
          createdAt: string;
          updatedAt: string;
          isDeleted?: boolean;
          // P053: object_list 的 ObjectSummary 已含完整解密 properties，
          // 直接读取 description，消除对每个页面单独 object_get 的 N+1 IPC。
          properties?: Record<string, unknown>;
        }>
      >('object_list', {
        accountId: accountId,
        filter: { typeId: 'page', includeDeleted: true },
      });
      request.assertCurrent();
      const oldPages = get().legacyCustomPages;
      const pages: CustomPage[] = objects.map((o, i) => {
        const metadata = o.properties;
        return {
          id: o.id,
          name: o.name,
          iconId: o.iconName || DEFAULT_CUSTOM_ICON,
          description: typeof metadata?.description === 'string' ? metadata.description : undefined,
          createdAt:
            typeof metadata?.legacyCreatedAt === 'string' ? metadata.legacyCreatedAt : o.createdAt,
          sortOrder: typeof metadata?.sortOrder === 'number' ? metadata.sortOrder : i,
          deletedAt: o.isDeleted ? o.updatedAt : undefined,
        };
      });
      const present = new Set(pages.map((p) => p.id));
      const missing = oldPages.filter((p) => !present.has(p.id) && !p.deletedAt);
      request.assertCurrent();
      const results = await Promise.allSettled(
        missing.map((p) =>
          request.invoke('object_create', {
            input: {
              id: p.id,
              accountId,
              name: p.name,
              typeId: 'page',
              iconName: p.iconId || DEFAULT_CUSTOM_ICON,
              properties: {
                description: p.description,
                legacyCreatedAt: p.createdAt,
                sortOrder: p.sortOrder,
              },
            },
          }),
        ),
      );
      request.assertCurrent();
      results.forEach((r, i) => {
        if (r.status === 'fulfilled') {
          pages.push(missing[i]);
          present.add(missing[i].id);
        } else {
          logger.warn('[settingsStore] Failed to migrate custom page:', missing[i].id, r.reason);
        }
      });
      // 未落库的旧删除页仅作为引用标签保留，绝不新建成活跃页面。
      const deletedLegacy = oldPages.filter((p) => p.deletedAt && !present.has(p.id));
      setCurrent((state) => ({
        settings: {
          ...state.settings,
          customPages: [...pages, ...deletedLegacy].sort((a, b) => a.sortOrder - b.sortOrder),
        },
      }));
      if (oldPages.length > 0 && results.every((r) => r.status === 'fulfilled')) {
        // 已存在（含软删除）或本次成功落库的 ID 才能从旧来源删除。
        // 旧删除页仍保留原记录，避免丢失模板引用和删除状态。
        try {
          await request.invoke('user_data_update_preference', {
            payload: { accountId, preferences: { customPages: deletedLegacy } },
          });
          request.assertCurrent();
          setCurrent({ legacyCustomPages: deletedLegacy });
        } catch (e) {
          if (!request.isCurrent()) return;
          logger.warn('[settingsStore] Failed to clear old-format custom pages:', e);
        }
      }
    } catch (e) {
      if (!request.isCurrent()) return;
      logger.warn('[settingsStore] Failed to load custom pages:', e);
    }
  },

  updateSetting: async (accountId, key, value) => {
    const request = requests.begin(`setting:${key}`, accountId);
    const setCurrent = request.guardSet<SettingsState>(set);
    const oldValue = get().settings[key];
    setCurrent((s) => ({ settings: { ...s.settings, [key]: value } }));
    try {
      await request.invoke('user_data_update_preference', {
        payload: { accountId, preferences: { [key]: value } },
      });
      request.assertCurrent();
      // P129: UI 键变更时同步 ②③ 副本（唯一写入点，页面不再各自写）。
      // ④ vault 写入成功后才触发，失败回滚时不会产生副本漂移。
      if (PLAINTEXT_PREF_KEYS.has(key)) {
        void syncPlaintextPref(key, value, request.isCurrent);
        if (CACHE_PREF_KEYS.has(key)) {
          writeUiPrefsCache(get().settings);
        }
      }
      if (key === 'language' && typeof value === 'string') {
        request.assertCurrent();
        await i18next.changeLanguage(value);
        request.assertCurrent();
        // ③ 已由上方 PLAINTEXT_PREF_KEYS 分支同步；此处仅补 ② i18nextLng 冷启动缓存。
        try {
          localStorage.setItem('i18nextLng', value);
        } catch (e) {
          if (!request.isCurrent()) return;
          logger.warn('[settingsStore] Failed to cache language:', e);
        }
      }
    } catch (e) {
      if (!request.isCurrent()) return;
      logger.warn('[settingsStore] Failed to update setting:', key, e);
      setCurrent((s) => ({ settings: { ...s.settings, [key]: oldValue } }));
    }
  },

  addCustomPage: async (accountId, name, iconId, description) => {
    const request = requests.begin(undefined, accountId);
    const setCurrent = request.guardSet<SettingsState>(set);
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
    setCurrent((s) => ({ settings: { ...s.settings, customPages: [...prevPages, newPage] } }));
    try {
      // P0-1: Store in objects table (not preferences JSON)
      // Pass the client-generated id so frontend state stays in sync with the database record.
      await request.invoke('object_create', {
        input: {
          accountId,
          name,
          typeId: 'page',
          iconName: iconId ?? DEFAULT_CUSTOM_ICON,
          properties: description ? { description } : {},
          id,
        },
      });
      request.assertCurrent();
    } catch (e) {
      request.assertCurrent();
      logger.warn('[settingsStore] Failed to add custom page:', name, e);
      // Rollback
      setCurrent((s) => ({ settings: { ...s.settings, customPages: prevPages } }));
      // P003: 失败必须向上抛——调用方（AddPageButton）依赖异常进入 catch 提示错误；
      // 此前无条件 return newPage，调用方 onCreate 导航到后端不存在的页面（刷新即消失）。
      throw e;
    }
    return newPage;
  },

  removeCustomPage: async (accountId, pageId) => {
    const request = requests.begin(undefined, accountId);
    const setCurrent = request.guardSet<SettingsState>(set);
    const prevPages = get().settings.customPages;
    const now = new Date().toISOString();
    // Mark as deleted locally (keep in array so templates can still reference the name)
    const pages = prevPages.map((p) => (p.id === pageId ? { ...p, deletedAt: now } : p));
    setCurrent((s) => ({ settings: { ...s.settings, customPages: pages } }));
    try {
      // P0-1: Use page_delete to create a "page" type trash item
      // sectionType must be the actual page UUID so that page_delete's sub-object
      // matching (section_type == section_type || collection_type == section_type)
      // correctly finds all child objects assigned to this custom page.
      await request.invoke('page_delete', {
        accountId: accountId,
        sectionType: pageId,
        pageObjectId: pageId,
      });
      request.assertCurrent();
    } catch (e) {
      if (!request.isCurrent()) return;
      logger.warn('[settingsStore] Failed to remove custom page:', pageId, e);
      setCurrent((s) => ({ settings: { ...s.settings, customPages: prevPages } }));
    }
  },

  clearOnVaultLock: () => {
    requests.invalidate();
    return set((state) => ({
      legacyCustomPages: [],
      // Keep UI-only preferences so lock screen retains user's language/theme/accent
      settings: {
        ...DEFAULT_SETTINGS,
        theme: state.settings.theme,
        accentColor: state.settings.accentColor,
        customAccentHex: state.settings.customAccentHex,
        reduceMotion: state.settings.reduceMotion,
        androidGlass: state.settings.androidGlass,
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
    }));
  },
}));

onRequestSessionChange(() => useSettingsStore.getState().clearOnVaultLock());
