// ============================================================
// Theme Schemes — Preset color palettes for light/dark modes
//
// Each scheme overrides a subset of CSS variables on <html>.
// Accent colors are intentionally excluded; they remain controlled
// by the separate accent-color selector.
// ============================================================

export interface ThemeScheme {
  id: string;
  nameKey: string;
  mode: 'light' | 'dark';
  /** Preview colors for the thumbnail (accent here is illustrative only) */
  preview: {
    bg: string;
    elevated: string;
    accent: string;
    text: string;
  };
  /** CSS variable overrides applied to document.documentElement */
  variables: Record<string, string>;
}

const LIGHT_SCHEMES: ThemeScheme[] = [
  {
    id: 'warm-stone',
    nameKey: 'settings:scheme_warm_stone',
    mode: 'light',
    preview: {
      bg: '#fafaf6',
      elevated: '#fdfcf9',
      accent: '#5b7c99',
      text: '#1f1c18',
    },
    variables: {
      '--bg-base': '#fafaf6',
      '--bg-elevated': '#fdfcf9',
      '--bg-toolbar': 'rgba(253, 252, 249, 0.85)',
      '--bg-inset': '#f2f0e9',
      '--bg-hover': '#f2f0e9',
      '--bg-active': '#ece9e0',
      '--text-primary': '#1f1c18',
      '--text-secondary': '#7a7265',
      '--text-tertiary': '#9e9585',
      '--text-inverse': '#fdfcf9',
      '--text-accent': '#5b7c99',
      '--text-warm': '#c4925c',
      '--border-subtle': '#e5e1d6',
      '--border-default': '#d5d0c3',
      '--border-strong': '#b8b0a0',
    },
  },
  {
    id: 'clean-slate',
    nameKey: 'settings:scheme_clean_slate',
    mode: 'light',
    preview: {
      bg: '#f6f8fa',
      elevated: '#ffffff',
      accent: '#5b7c99',
      text: '#1f2328',
    },
    variables: {
      '--bg-base': '#f6f8fa',
      '--bg-elevated': '#ffffff',
      '--bg-toolbar': 'rgba(255, 255, 255, 0.85)',
      '--bg-inset': '#eef1f4',
      '--bg-hover': '#eef1f4',
      '--bg-active': '#e4e8ec',
      '--text-primary': '#1f2328',
      '--text-secondary': '#5d6a78',
      '--text-tertiary': '#8c9aab',
      '--text-inverse': '#ffffff',
      '--text-accent': '#4a6a85',
      '--text-warm': '#8f6b4a',
      '--border-subtle': '#d1d9e0',
      '--border-default': '#c4cdd5',
      '--border-strong': '#9eaab6',
    },
  },
  {
    id: 'soft-cream',
    nameKey: 'settings:scheme_soft_cream',
    mode: 'light',
    preview: {
      bg: '#fcf9f2',
      elevated: '#fffefb',
      accent: '#c4925c',
      text: '#3d3428',
    },
    variables: {
      '--bg-base': '#fcf9f2',
      '--bg-elevated': '#fffefb',
      '--bg-toolbar': 'rgba(255, 254, 251, 0.9)',
      '--bg-inset': '#f5f0e4',
      '--bg-hover': '#f5f0e4',
      '--bg-active': '#ede6d6',
      '--text-primary': '#3d3428',
      '--text-secondary': '#7d6f5a',
      '--text-tertiary': '#a89b84',
      '--text-inverse': '#fffefb',
      '--text-accent': '#9e6b3a',
      '--text-warm': '#b3804a',
      '--border-subtle': '#e8e0cc',
      '--border-default': '#d9ceb0',
      '--border-strong': '#b8aa8a',
    },
  },
  {
    id: 'fresh-mint',
    nameKey: 'settings:scheme_fresh_mint',
    mode: 'light',
    preview: {
      bg: '#f4f9f6',
      elevated: '#fbfcfb',
      accent: '#5b8c6f',
      text: '#1e2922',
    },
    variables: {
      '--bg-base': '#f4f9f6',
      '--bg-elevated': '#fbfcfb',
      '--bg-toolbar': 'rgba(251, 252, 251, 0.88)',
      '--bg-inset': '#e8f0eb',
      '--bg-hover': '#e8f0eb',
      '--bg-active': '#dde8e1',
      '--text-primary': '#1e2922',
      '--text-secondary': '#52685a',
      '--text-tertiary': '#849786',
      '--text-inverse': '#fbfcfb',
      '--text-accent': '#4d7a62',
      '--text-warm': '#7a6a4a',
      '--border-subtle': '#d2e0d6',
      '--border-default': '#bfcfbf',
      '--border-strong': '#94a894',
    },
  },
];

const DARK_SCHEMES: ThemeScheme[] = [
  {
    id: 'warm-stone-dark',
    nameKey: 'settings:scheme_warm_stone_dark',
    mode: 'dark',
    preview: {
      bg: '#1f1c18',
      elevated: '#2a2620',
      accent: '#7a9ab5',
      text: '#ddd8c8',
    },
    variables: {
      '--bg-base': '#1f1c18',
      '--bg-elevated': '#2a2620',
      '--bg-toolbar': 'rgba(42, 38, 32, 0.92)',
      '--bg-inset': 'rgba(26, 23, 20, 0.5)',
      '--bg-hover': '#353029',
      '--bg-active': '#423b33',
      '--text-primary': '#ddd8c8',
      '--text-secondary': '#a69d8a',
      '--text-tertiary': '#7a7265',
      '--text-inverse': '#1f1c18',
      '--text-accent': '#a8bfd4',
      '--text-warm': '#d4b88a',
      '--border-subtle': '#3d3831',
      '--border-default': '#4a433b',
      '--border-strong': '#5c5549',
    },
  },
  {
    id: 'deep-ocean',
    nameKey: 'settings:scheme_deep_ocean',
    mode: 'dark',
    preview: {
      bg: '#181c24',
      elevated: '#21262f',
      accent: '#7a9ab5',
      text: '#d8dde6',
    },
    variables: {
      '--bg-base': '#181c24',
      '--bg-elevated': '#21262f',
      '--bg-toolbar': 'rgba(33, 38, 47, 0.92)',
      '--bg-inset': 'rgba(16, 19, 24, 0.5)',
      '--bg-hover': '#2c3340',
      '--bg-active': '#373f4d',
      '--text-primary': '#d8dde6',
      '--text-secondary': '#96a0b0',
      '--text-tertiary': '#677285',
      '--text-inverse': '#181c24',
      '--text-accent': '#9ab8d4',
      '--text-warm': '#c4a574',
      '--border-subtle': '#313742',
      '--border-default': '#3d4552',
      '--border-strong': '#4f5866',
    },
  },
  {
    id: 'midnight',
    nameKey: 'settings:scheme_midnight',
    mode: 'dark',
    preview: {
      bg: '#141416',
      elevated: '#1e1e20',
      accent: '#b06b7a',
      text: '#e2e2e4',
    },
    variables: {
      '--bg-base': '#141416',
      '--bg-elevated': '#1e1e20',
      '--bg-toolbar': 'rgba(30, 30, 32, 0.92)',
      '--bg-inset': 'rgba(10, 10, 12, 0.5)',
      '--bg-hover': '#28282c',
      '--bg-active': '#333338',
      '--text-primary': '#e2e2e4',
      '--text-secondary': '#96969a',
      '--text-tertiary': '#6a6a70',
      '--text-inverse': '#141416',
      '--text-accent': '#d48a9a',
      '--text-warm': '#c4a574',
      '--border-subtle': '#2c2c30',
      '--border-default': '#38383c',
      '--border-strong': '#4a4a4e',
    },
  },
  {
    id: 'forest-night',
    nameKey: 'settings:scheme_forest_night',
    mode: 'dark',
    preview: {
      bg: '#1a211d',
      elevated: '#242d28',
      accent: '#7aaf8f',
      text: '#d6ddd8',
    },
    variables: {
      '--bg-base': '#1a211d',
      '--bg-elevated': '#242d28',
      '--bg-toolbar': 'rgba(36, 45, 40, 0.92)',
      '--bg-inset': 'rgba(14, 18, 16, 0.5)',
      '--bg-hover': '#303c35',
      '--bg-active': '#3a4740',
      '--text-primary': '#d6ddd8',
      '--text-secondary': '#92a095',
      '--text-tertiary': '#69796e',
      '--text-inverse': '#1a211d',
      '--text-accent': '#9eceb0',
      '--text-warm': '#c4a574',
      '--border-subtle': '#354039',
      '--border-default': '#3f4b43',
      '--border-strong': '#536058',
    },
  },
];

export const THEME_SCHEMES: ThemeScheme[] = [...LIGHT_SCHEMES, ...DARK_SCHEMES];

export function getSchemeById(id: string | null): ThemeScheme | undefined {
  return THEME_SCHEMES.find((s) => s.id === id);
}

export function getSchemesByMode(mode: 'light' | 'dark'): ThemeScheme[] {
  return THEME_SCHEMES.filter((s) => s.mode === mode);
}

/** Apply a scheme's CSS variables to the document root. */
export function applyScheme(id: string | null) {
  const root = document.documentElement;
  if (!id) {
    clearScheme();
    return;
  }
  const scheme = getSchemeById(id);
  if (!scheme) {
    clearScheme();
    return;
  }
  Object.entries(scheme.variables).forEach(([key, value]) => {
    root.style.setProperty(key, value);
  });
}

/** Remove all scheme-specific overrides from the document root. */
export function clearScheme() {
  const root = document.documentElement;
  const firstScheme = THEME_SCHEMES[0];
  if (!firstScheme) return;
  Object.keys(firstScheme.variables).forEach((key) => {
    root.style.removeProperty(key);
  });
}

import type { ThemePreset } from '@/types';

/** Resolve which scheme should currently be active. */
export function resolveActiveScheme(
  themePreset: ThemePreset,
  defaultLightTheme: string,
  defaultDarkTheme: string,
): string {
  if (themePreset === 'system') {
    const isDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
    return isDark ? defaultDarkTheme : defaultLightTheme;
  }
  return themePreset === 'warm-stone-dark' ? defaultDarkTheme : defaultLightTheme;
}
