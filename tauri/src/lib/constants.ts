// ============================================================================
// Shared timing / UX constants (from original constants.ts)
// ============================================================================

/** Duration (ms) to show "copied" feedback after a copy-to-clipboard action. */
export const COPY_FEEDBACK_DURATION_MS = 1500;

/** Default debounce delay (ms) for search inputs and similar live filters. */
export const DEBOUNCE_DELAY_MS = 300;

/** Current OCR model series displayed in the scan page. Change this when switching models. */
export const OCR_MODEL_SERIES = 'PP-OCRv6';

/** Prefix returned by the Rust backend when the active OCR model is not installed. */
export const OCR_MODEL_NOT_INSTALLED_PREFIX = '__OCR_MODEL_NOT_INSTALLED__';

/** TTL (ms) for search result cache — same keyword won't refetch within this window. */
export const SEARCH_CACHE_TTL_MS = 30_000;

// ============================================================================
// Icon sizes (from iconSizes.ts)
// ============================================================================

/** 语义化图标尺寸常量。 */
export const ICON_SIZE = {
  '2xs': 10,
  xs: 12,
  sm: 14,
  md: 16,
  lg: 18,
  xl: 20,
  '2xl': 24,
  '3xl': 32,
  '4xl': 40,
  '5xl': 48,
  '6xl': 72,
} as const;

// ============================================================================
// LocalStorage keys (from storageKeys.ts)
// ============================================================================

/** 用户界面偏好缓存（主题、强调色等） */
export const ST_UI_PREFS = 'solosoul_ui_prefs';

/** 用户已跳过的版本号（更新提示） */
export const ST_SKIPPED_VERSION = 'solosoul_skipped_version';

/** 用户是否已完成新手引导 */
export const ST_ONBOARDING_SEEN = 'solosoul_onboarding_seen';

/** OCR 首次安装已完成标记 */
export const ST_OCR_FIRST_INSTALL = 'solosoul_ocr_first_install_done';

/** 语言偏好（与 i18next 格式兼容） */
export const ST_I18NEXT_LANG = 'i18nextLng';

/** 快速聊天会话存储键前缀 */
export const ST_QUICK_CHAT_PREFIX = 'solosoul_quick_chat_conv_';
