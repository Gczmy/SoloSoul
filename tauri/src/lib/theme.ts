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
let systemMediaQuery: MediaQueryList | null = null;
let systemListener: ((e: MediaQueryListEvent) => void) | null = null;

/** Resolve the effective theme (light or dark) given a ThemeConfig preset */
export function resolveEffectiveTheme(preset: ThemeConfig['preset']): 'light' | 'dark' {
  if (preset === 'system') {
    return window.matchMedia(SYSTEM_DARK_MQ).matches ? 'dark' : 'light';
  }
  return preset === 'warm-stone-dark' ? 'dark' : 'light';
}

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

function rgbToHex(r: number, g: number, b: number): string {
  return `#${[r, g, b]
    .map((v) => Math.max(0, Math.min(255, Math.round(v))).toString(16).padStart(2, '0'))
    .join('')}`;
}

/** Generate a slightly darker hover variant for a custom accent hex. */
function adjustAccentHover(hex: string): string {
  const rgb = hexToRgb(hex);
  if (!rgb) return hex;
  const factor = -0.12;
  const [r, g, b] = rgb.map((c) => c + c * factor);
  return rgbToHex(r, g, b);
}

/** Sync the native window background color with the active scheme so the
 *  system title bar (traffic lights area) matches the app theme. */
async function syncTitleBarColor(config: ThemeConfig) {
  try {
    const schemeId = resolveActiveScheme(
      config.preset,
      config.defaultLightTheme || 'warm-stone',
      config.defaultDarkTheme || 'warm-stone-dark',
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

/** Apply accent color as a CSS custom property on <html>.
 *  Preset accents also set [data-accent] so themes.css can provide the
 *  matching hover/focus/selected tokens; custom accents compute a hover
 *  variant inline. */
export function applyAccentColor(accent: AccentPreset, customHex?: string) {
  const root = document.documentElement;
  if (accent === 'custom' && customHex) {
    root.setAttribute('data-accent', 'custom');
    root.style.setProperty('--accent-primary', customHex);
    root.style.setProperty('--accent-hover', adjustAccentHover(customHex));
    return;
  }
  const preset = accent && ACCENT_COLORS[accent] ? accent : 'ocean';
  root.setAttribute('data-accent', preset);
  // Let themes.css [data-accent] selectors drive --accent-primary and --accent-hover.
  root.style.removeProperty('--accent-primary');
  root.style.removeProperty('--accent-hover');
}

/** Full theme application: mode (data-theme attr) + accent color + active scheme */
export function applyTheme(config: ThemeConfig) {
  const root = document.documentElement;
  // Set theme data attribute for CSS selectors
  if (config.preset === 'warm-stone-light') {
    root.setAttribute('data-theme', 'light');
  } else if (config.preset === 'warm-stone-dark') {
    root.setAttribute('data-theme', 'dark');
  } else {
    root.setAttribute('data-theme', 'system');
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
  );
  applyScheme(activeScheme);

  // Sync native title bar background with the active theme
  void syncTitleBarColor(config);
}

/** Listen for system theme changes (4.3.5) */
export function listenForSystemTheme(
  config: ThemeConfig,
  onThemeChange: (mode: 'light' | 'dark') => void,
) {
  if (systemMediaQuery && systemListener) {
    systemMediaQuery.removeEventListener('change', systemListener);
  }
  systemMediaQuery = window.matchMedia(SYSTEM_DARK_MQ);
  systemListener = (e: MediaQueryListEvent) => {
    if (config.preset === 'system') {
      onThemeChange(e.matches ? 'dark' : 'light');
      // Re-apply the default scheme for the new system mode
      const activeScheme = resolveActiveScheme(
        config.preset,
        config.defaultLightTheme || 'warm-stone',
        config.defaultDarkTheme || 'warm-stone-dark',
      );
      applyScheme(activeScheme);
      void syncTitleBarColor(config);
    }
  };
  systemMediaQuery.addEventListener('change', systemListener);
}

export function stopListeningForSystemTheme() {
  if (systemMediaQuery && systemListener) {
    systemMediaQuery.removeEventListener('change', systemListener);
    systemMediaQuery = null;
    systemListener = null;
  }
}
