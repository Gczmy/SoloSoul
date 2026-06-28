/**
 * 集中式 localStorage 键常量。
 *
 * P215: 所有 localStorage 键统一在此定义，避免分散在各文件中的字符串字面量。
 * 命名规范：`ST_` 前缀表示 SoloSoul 应用的键。
 */

/** 用户界面偏好缓存（主题、强调色等） */
export const ST_UI_PREFS = 'solosoul_ui_prefs';

/** 窗口大小缓存 */
export const ST_WINDOW_SIZE = 'solosoul_window_size';

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
