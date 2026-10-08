// Theme system utilities (per 10_跨平台视觉规范与主题系统 §4.3)
// Applies accent colors and theme mode by setting CSS custom properties
// on <html>, driving light/dark via [data-theme] selectors.

import type { AccentPreset, ThemeConfig } from '@/types';
import { applyScheme, resolveActiveScheme, getSchemeById } from './themeSchemes';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { logger } from './logger';
import { withTimeout } from './withTimeout';
import { syncNativeAppearance } from './nativeWindow';
import { isAndroidSync, isMobilePlatformSync } from './platform';
import { applyAndroidMaterial } from './androidMaterial';
import { applyAccentTextColors, customAccentHover } from './accentContrast';

export const ACCENT_COLORS: Record<AccentPreset, string> = {
  ocean: '#5B7C99',
  amber: '#C4925C',
  forest: '#5B8C6F',
  rose: '#B06B7A',
  purple: '#8B7AA8',
  custom: '', // filled by customAccentHex
};

const SYSTEM_DARK_MQ = '(prefers-color-scheme: dark)';

function hexToRgb(hex: string): [number, number, number] | null {
  const cleaned = hex.replace('#', '');
  if (cleaned.length !== 3 && cleaned.length !== 6) return null;
  const full =
    cleaned.length === 3
      ? cleaned
          .split('')
          .map((c) => c + c)
          .join('')
      : cleaned;
  const num = parseInt(full, 16);
  if (Number.isNaN(num)) return null;
  return [(num >> 16) & 255, (num >> 8) & 255, num & 255];
}

/** Sync the native window background color with the active scheme so the
 *  system title bar (traffic lights area) matches the app theme. */
async function syncTitleBarColor(schemeId: string) {
  try {
    const scheme = getSchemeById(schemeId);
    const bg = scheme?.variables['--bg-base'] || '#1c1c1e';
    const rgb = hexToRgb(bg) || [28, 28, 30];
    await syncNativeAppearance({ red: rgb[0], green: rgb[1], blue: rgb[2] });
  } catch {
    // ignore when running in browser or API unavailable
  }
}

/** Sync Android status/navigation bar icon style with the active app theme.
 *  "dark" app theme → white icons/text; "light" app theme → black icons/text.
 *  非 Android 平台调用会被安全忽略。 */
export async function syncStatusBarStyle(theme: 'light' | 'dark') {
  try {
    await invoke('set_status_bar_style', { payload: { style: theme } });
  } catch (err) {
    logger.warn('[theme] syncStatusBarStyle failed:', err);
    // ignore when running in browser, desktop, or API unavailable
  }
}

/** Apply accent color as a CSS custom property on <html>.
 *  Preset accents also set [data-accent] so themes.css can provide the
 *  matching hover/focus/selected tokens; custom accents compute a hover
 *  variant inline. */
function applyAccentColor(accent: AccentPreset, customHex?: string) {
  const root = document.documentElement;
  const customHover = customHex ? customAccentHover(customHex) : null;
  if (accent === 'custom' && customHex && customHover) {
    root.setAttribute('data-accent', 'custom');
    root.style.setProperty('--accent-primary', customHex);
    root.style.setProperty('--accent-hover', customHover);
    return;
  }
  const preset = accent && ACCENT_COLORS[accent] ? accent : 'ocean';
  root.setAttribute('data-accent', preset);
  // Let themes.css [data-accent] selectors drive --accent-primary and --accent-hover.
  root.style.removeProperty('--accent-primary');
  root.style.removeProperty('--accent-hover');
}

/** 移动端使用 WebView 的真实系统外观；桌面优先使用 Rust 检测，IPC 不可用时回退。 */
export async function getSystemTheme(): Promise<'light' | 'dark'> {
  // 移动端由 WebView 提供真实系统外观；不能采用后端的占位检测结果。
  if (isMobilePlatformSync()) {
    return window.matchMedia(SYSTEM_DARK_MQ).matches ? 'dark' : 'light';
  }
  try {
    const mode = await withTimeout(invoke<string>('get_system_theme'), 600);
    return mode === 'dark' ? 'dark' : 'light';
  } catch {
    // Fallback to window.matchMedia if IPC fails
    return window.matchMedia(SYSTEM_DARK_MQ).matches ? 'dark' : 'light';
  }
}

/** Full theme application: mode (data-theme attr) + accent color + active scheme */
export async function applyTheme(config: ThemeConfig, isCurrent: () => boolean = () => true) {
  const root = document.documentElement;

  // 一次解析后由 DOM、色板和原生栏共用，避免 IPC 与 WebView 主题不一致。
  const resolvedMode =
    config.preset === 'system'
      ? (config.resolvedSystemTheme ?? (await getSystemTheme()))
      : config.preset === 'warm-stone-dark'
        ? 'dark'
        : 'light';
  // 系统解析可能跨越偏好、账户或挂载代次；必须在写 DOM/原生参数之前检查。
  if (!isCurrent()) return;
  const activeScheme = resolveActiveScheme(
    config.preset,
    config.defaultLightTheme || 'warm-stone',
    config.defaultDarkTheme || 'warm-stone-dark',
    resolvedMode,
  );
  const accent = config.accentColor;

  root.setAttribute('data-theme', resolvedMode);
  applyAccentColor(accent, config.customAccentHex);
  applyScheme(activeScheme);
  applyAccentTextColors();
  if (isAndroidSync()) applyAndroidMaterial();

  // Sync native title bar background with the active theme (desktop only)
  void syncTitleBarColor(activeScheme);

  // Sync Android status/navigation bar icon color with the active theme
  void syncStatusBarStyle(resolvedMode);
}

/** 同一媒体查询同时负责移动端切色和桌面 IPC 不可用时的回退。 */
function listenForMediaTheme(onThemeChange: (mode: 'light' | 'dark') => void): () => void {
  const mq = window.matchMedia(SYSTEM_DARK_MQ);
  const listener = (e: MediaQueryListEvent) => onThemeChange(e.matches ? 'dark' : 'light');
  mq.addEventListener('change', listener);
  return () => mq.removeEventListener('change', listener);
}

/** 移动端只监听 WebView；桌面优先监听原生轮询事件。
 * 调用者仅在跟随系统时应用事件，卸载时释放返回的监听。 */
export async function listenForSystemTheme(
  onThemeChange: (mode: 'light' | 'dark') => void,
): Promise<() => void> {
  // 移动端没有桌面轮询事件；Tauri listen 成功不代表这个事件源存在。
  if (isMobilePlatformSync()) return listenForMediaTheme(onThemeChange);
  try {
    const { listen } = await import('@tauri-apps/api/event');
    const unlisten = await listen<string>('system-theme-changed', (event) => {
      const mode = event.payload === 'dark' ? 'dark' : 'light';
      onThemeChange(mode);
    });
    return unlisten;
  } catch {
    // Fallback for non-Tauri environments (browser, Storybook, tests)
    return listenForMediaTheme(onThemeChange);
  }
}
