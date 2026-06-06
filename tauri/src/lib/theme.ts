// Theme system utilities (per 09_ 4.3)
// Applies accent colors and theme mode via data attributes on <html>.

import type { AccentPreset, ThemeConfig } from '@/types';

const THEME_ATTR = 'data-theme';
const ACCENT_ATTR = 'data-accent';
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

/** Apply theme mode (light/dark) as a data attribute */
export function applyThemeMode(mode: 'light' | 'dark') {
  document.documentElement.setAttribute(THEME_ATTR, mode);
  // Also set a CSS variable so dark-mode selectors work via attribute
  document.documentElement.style.setProperty('--data-theme', mode);
}

/** Apply accent color preset as a data attribute */
export function applyAccentColor(accent: AccentPreset) {
  document.documentElement.setAttribute(ACCENT_ATTR, accent);
  document.documentElement.style.setProperty('--data-accent', accent);
}

/** Listen for system theme changes and auto-switch (4.3.5) */
export function listenForSystemTheme(
  config: ThemeConfig,
  onThemeChange: (mode: 'light' | 'dark') => void,
) {
  // Clean up previous listener
  if (systemMediaQuery && systemListener) {
    systemMediaQuery.removeEventListener('change', systemListener);
  }

  systemMediaQuery = window.matchMedia(SYSTEM_DARK_MQ);
  systemListener = (e: MediaQueryListEvent) => {
    if (config.preset === 'system') {
      onThemeChange(e.matches ? 'dark' : 'light');
    }
  };
  systemMediaQuery.addEventListener('change', systemListener);
}

/** Remove system theme listener */
export function stopListeningForSystemTheme() {
  if (systemMediaQuery && systemListener) {
    systemMediaQuery.removeEventListener('change', systemListener);
    systemMediaQuery = null;
    systemListener = null;
  }
}

/** Full theme application: mode + accent (4.3.5 — instant, no refresh) */
export function applyTheme(config: ThemeConfig) {
  const mode = resolveEffectiveTheme(config.preset);
  applyThemeMode(mode);
  applyAccentColor(config.accentColor);
}

/** Apply custom accent hex via CSS variable (for "custom" preset) */
export function applyCustomAccent(hex: string) {
  document.documentElement.style.setProperty('--accent-primary', hex);
}
