// Theme system utilities (per 09_跨平台材质系统与视觉规范 §4.3)
// Applies accent colors and theme mode by setting CSS custom properties
// on <html>, driving light/dark via [data-theme] selectors.

import type { AccentPreset, ThemeConfig } from '@/types';
import { applyScheme, resolveActiveScheme, getSchemeById } from './themeSchemes';
import { invoke } from '@tauri-apps/api/core';

const ACCENT_COLORS: Record<AccentPreset, string> = {
  ocean: '#5B7C99',
  amber: '#C4925C',
  forest: '#5B8C6F',
  rose: '#B06B7A',
  purple: '#8B7AA8',
  custom: '', // filled by customAccentHex
};

const SYSTEM_DARK_MQ = '(prefers-color-scheme: dark)';

export function hexToRgb(hex: string): [number, number, number] | null {
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
async function syncTitleBarColor(config: ThemeConfig) {
  try {
    const schemeId = resolveActiveScheme(
      config.preset,
      config.defaultLightTheme || 'warm-stone',
      config.defaultDarkTheme || 'warm-stone-dark',
      config.resolvedSystemTheme,
    );
    const scheme = getSchemeById(schemeId);
    const bg = scheme?.variables['--bg-base'] || '#1c1c1e';
    const rgb = hexToRgb(bg) || [28, 28, 30];
    await invoke('set_titlebar_color', {
      color: { red: rgb[0], green: rgb[1], blue: rgb[2] },
    });
  } catch {
    // ignore when running in browser or API unavailable
  }
}

/** Sync Android status/navigation bar icon style with the active app theme.
 *  "dark" app theme → white icons/text; "light" app theme → black icons/text.
 *  非 Android 平台调用会被安全忽略。 */
export async function syncStatusBarStyle(theme: 'light' | 'dark') {
  try {
    console.log('[theme] syncStatusBarStyle:', theme);
    await invoke('set_status_bar_style', { payload: { style: theme } });
    console.log('[theme] syncStatusBarStyle success:', theme);
  } catch (err) {
    console.warn('[theme] syncStatusBarStyle failed:', err);
    // ignore when running in browser, desktop, or API unavailable
  }
}

/** Apply accent color as a CSS custom property on <html>.
 *  Preset accents also set [data-accent] so themes.css can provide the
 *  matching hover/focus/selected tokens; custom accents compute a hover
 *  variant inline. */
export function applyAccentColor(accent: AccentPreset, customHex?: string) {
  const root = document.documentElement;
  if (accent === 'custom' && customHex) {
    root.setAttribute('data-accent', 'custom');
    root.style.setProperty('--accent-primary', customHex);
    root.style.setProperty('--accent-hover', 'color-mix(in srgb, var(--accent-primary), black 12%)');
    return;
  }
  const preset = accent && ACCENT_COLORS[accent] ? accent : 'ocean';
  root.setAttribute('data-accent', preset);
  // Let themes.css [data-accent] selectors drive --accent-primary and --accent-hover.
  root.style.removeProperty('--accent-primary');
  root.style.removeProperty('--accent-hover');
}

/** Query the Rust backend for the actual OS theme.
 *  This is the fallback when window.matchMedia('prefers-color-scheme') does not
 *  work correctly inside the Tauri WebView (e.g. on macOS). */
export async function getSystemTheme(): Promise<'light' | 'dark'> {
  try {
    const mode = await invoke<string>('get_system_theme');
    return mode === 'dark' ? 'dark' : 'light';
  } catch {
    // Fallback to window.matchMedia if IPC fails
    return window.matchMedia(SYSTEM_DARK_MQ).matches ? 'dark' : 'light';
  }
}

/** Full theme application: mode (data-theme attr) + accent color + active scheme */
export async function applyTheme(config: ThemeConfig) {
  const root = document.documentElement;

  // When preset is 'system', resolve to actual light/dark so all compound
  // [data-theme] CSS selectors (e.g. [data-theme='dark'][data-accent='ocean'])
  // match correctly. Relying on @media (prefers-color-scheme) for
  // [data-theme='system'] leaves gaps for accent colors and other overrides.
  if (config.preset === 'warm-stone-light') {
    root.setAttribute('data-theme', 'light');
  } else if (config.preset === 'warm-stone-dark') {
    root.setAttribute('data-theme', 'dark');
  } else {
    // preset === 'system' — resolve to actual OS mode
    // Use backend-provided resolvedSystemTheme if available (more reliable on macOS),
    // otherwise fall back to window.matchMedia.
    const resolved = config.resolvedSystemTheme ?? (await getSystemTheme());
    root.setAttribute('data-theme', resolved === 'dark' ? 'dark' : 'light');
  }

  // Apply accent color
  applyAccentColor(
    config.accentColor as AccentPreset,
    (config as { customAccentHex?: string }).customAccentHex,
  );

  // Apply active scheme variables
  const activeScheme = resolveActiveScheme(
    config.preset,
    config.defaultLightTheme || 'warm-stone',
    config.defaultDarkTheme || 'warm-stone-dark',
    config.resolvedSystemTheme,
  );
  applyScheme(activeScheme);

  // Sync native title bar background with the active theme (desktop only)
  void syncTitleBarColor(config);

  // Sync Android status/navigation bar icon color with the active theme
  const resolvedTheme = root.getAttribute('data-theme') === 'dark' ? 'dark' : 'light';
  void syncStatusBarStyle(resolvedTheme);
}

/** Listen for system theme changes (4.3.5) via Tauri Event from Rust backend.
 *  The Rust side polls dark-light::detect() every second and emits
 *  'system-theme-changed' when the OS theme changes.
 *  Returns an unlisten function that should be called on cleanup.
 *  Falls back to window.matchMedia in non-Tauri environments.
 *  Callers should check their own settings (e.g. settings.theme === 'system')
 *  before applying changes. */
export async function listenForSystemTheme(
  onThemeChange: (mode: 'light' | 'dark') => void,
): Promise<() => void> {
  try {
    const { listen } = await import('@tauri-apps/api/event');
    const unlisten = await listen<string>('system-theme-changed', (event) => {
      const mode = event.payload === 'dark' ? 'dark' : 'light';
      onThemeChange(mode);
    });
    return unlisten;
  } catch {
    // Fallback for non-Tauri environments (browser, Storybook, tests)
    const mq = window.matchMedia(SYSTEM_DARK_MQ);
    const listener = (e: MediaQueryListEvent) => onThemeChange(e.matches ? 'dark' : 'light');
    mq.addEventListener('change', listener);
    return () => mq.removeEventListener('change', listener);
  }
}

