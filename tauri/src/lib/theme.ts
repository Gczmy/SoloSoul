// Theme system utilities (per 09_跨平台材质系统与视觉规范 §4.3)
// Applies accent colors and theme mode by setting CSS custom properties
// on <html>, driving light/dark via [data-theme] selectors.

import type { AccentPreset, ThemeConfig } from '@/types';
import { applyScheme, resolveActiveScheme } from './themeSchemes';

const ACCENT_COLORS: Record<AccentPreset, string> = {
  ocean: '#5B7C99',
  amber: '#C4925C',
  forest: '#5B8C6F',
  rose: '#B06B7A',
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

/** Apply accent color as a CSS custom property on <html> */
export function applyAccentColor(accent: AccentPreset, customHex?: string) {
  const root = document.documentElement;
  if (accent === 'custom' && customHex) {
    root.style.setProperty('--accent-primary', customHex);
  } else {
    const color = ACCENT_COLORS[accent] || ACCENT_COLORS.ocean;
    root.style.setProperty('--accent-primary', color);
  }
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
