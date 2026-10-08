// ============================================================
// Theme Schemes — Preset color palettes for light/dark modes
//
// 色板只定义表面与边框；所有平台的正文文字由明暗模式统一生成。
// Accent colors are intentionally excluded; they remain controlled
// by the separate accent-color selector.
// ============================================================

import type { ThemePreset } from '@/types';

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

// 编译期限定色板的边界，新增色板不能自行定义正文/功能文字颜色。
type SurfaceToken =
  | '--bg-base'
  | '--bg-elevated'
  | '--bg-toolbar'
  | '--bg-elevated-hover'
  | '--bg-inset'
  | '--bg-hover'
  | '--bg-active'
  | '--border-subtle'
  | '--border-default'
  | '--border-strong';
type SurfaceScheme = Omit<ThemeScheme, 'variables' | 'preview'> & {
  variables: Record<SurfaceToken, string>;
  preview: Omit<ThemeScheme['preview'], 'text'>;
};

const NEUTRAL_TEXT = {
  light: {
    '--text-primary': '#111111',
    '--text-secondary': '#202020',
    '--text-tertiary': '#303030',
    '--text-inverse': '#ffffff',
  },
  dark: {
    '--text-primary': '#ffffff',
    '--text-secondary': '#f5f5f5',
    '--text-tertiary': '#ededed',
    '--text-inverse': '#111111',
  },
} as const;

const LIGHT_SCHEMES: SurfaceScheme[] = [
  {
    id: 'warm-stone',
    nameKey: 'settings:scheme_warm_stone',
    mode: 'light',
    preview: { bg: '#fafaf6', elevated: '#fdfcf9', accent: '#5b7c99' },
    variables: {
      '--bg-base': '#fafaf6',
      '--bg-elevated': '#fdfcf9',
      '--bg-toolbar': 'rgba(253, 252, 249, 0.85)',
      '--bg-elevated-hover': '#f7f6f1',
      '--bg-inset': '#f2f0e9',
      '--bg-hover': '#f2f0e9',
      '--bg-active': '#ece9e0',
      '--border-subtle': '#e5e1d6',
      '--border-default': '#d5d0c3',
      '--border-strong': '#b8b0a0',
    },
  },
  {
    id: 'clean-slate',
    nameKey: 'settings:scheme_clean_slate',
    mode: 'light',
    preview: { bg: '#f6f8fa', elevated: '#ffffff', accent: '#5b7c99' },
    variables: {
      '--bg-base': '#f6f8fa',
      '--bg-elevated': '#ffffff',
      '--bg-toolbar': 'rgba(255, 255, 255, 0.85)',
      '--bg-elevated-hover': '#f6f8f9',
      '--bg-inset': '#eef1f4',
      '--bg-hover': '#eef1f4',
      '--bg-active': '#e4e8ec',
      '--border-subtle': '#d1d9e0',
      '--border-default': '#c4cdd5',
      '--border-strong': '#9eaab6',
    },
  },
  {
    id: 'soft-cream',
    nameKey: 'settings:scheme_soft_cream',
    mode: 'light',
    preview: { bg: '#fcf9f2', elevated: '#fffefb', accent: '#c4925c' },
    variables: {
      '--bg-base': '#fcf9f2',
      '--bg-elevated': '#fffefb',
      '--bg-toolbar': 'rgba(255, 254, 251, 0.9)',
      '--bg-elevated-hover': '#faf7ef',
      '--bg-inset': '#f5f0e4',
      '--bg-hover': '#f5f0e4',
      '--bg-active': '#ede6d6',
      '--border-subtle': '#e8e0cc',
      '--border-default': '#d9ceb0',
      '--border-strong': '#b8aa8a',
    },
  },
  {
    id: 'fresh-mint',
    nameKey: 'settings:scheme_fresh_mint',
    mode: 'light',
    preview: { bg: '#f4f9f6', elevated: '#fbfcfb', accent: '#5b8c6f' },
    variables: {
      '--bg-base': '#f4f9f6',
      '--bg-elevated': '#fbfcfb',
      '--bg-toolbar': 'rgba(251, 252, 251, 0.88)',
      '--bg-elevated-hover': '#f1f6f3',
      '--bg-inset': '#e8f0eb',
      '--bg-hover': '#e8f0eb',
      '--bg-active': '#dde8e1',
      '--border-subtle': '#d2e0d6',
      '--border-default': '#bfcfbf',
      '--border-strong': '#94a894',
    },
  },
  {
    id: 'blush-pink',
    nameKey: 'settings:scheme_blush_pink',
    mode: 'light',
    preview: { bg: '#fcf5f6', elevated: '#fffafb', accent: '#b8838e' },
    variables: {
      '--bg-base': '#fcf5f6',
      '--bg-elevated': '#fffafb',
      '--bg-toolbar': 'rgba(255, 250, 251, 0.9)',
      '--bg-elevated-hover': '#faf1f2',
      '--bg-inset': '#f5e8ea',
      '--bg-hover': '#f5e8ea',
      '--bg-active': '#eddbde',
      '--border-subtle': '#eddce0',
      '--border-default': '#dec4c9',
      '--border-strong': '#bfa0a6',
    },
  },
  {
    id: 'lavender-mist',
    nameKey: 'settings:scheme_lavender_mist',
    mode: 'light',
    preview: { bg: '#f7f5fa', elevated: '#fdfcfe', accent: '#8b7aa8' },
    variables: {
      '--bg-base': '#f7f5fa',
      '--bg-elevated': '#fdfcfe',
      '--bg-toolbar': 'rgba(253, 252, 254, 0.9)',
      '--bg-elevated-hover': '#f5f3f8',
      '--bg-inset': '#edeaf2',
      '--bg-hover': '#edeaf2',
      '--bg-active': '#e3dee9',
      '--border-subtle': '#ddd6e5',
      '--border-default': '#c9bfd4',
      '--border-strong': '#a89bb5',
    },
  },
  {
    id: 'sunny-lemon',
    nameKey: 'settings:scheme_sunny_lemon',
    mode: 'light',
    preview: { bg: '#fcfbf2', elevated: '#fffef8', accent: '#c4a85c' },
    variables: {
      '--bg-base': '#fcfbf2',
      '--bg-elevated': '#fffef8',
      '--bg-toolbar': 'rgba(255, 254, 248, 0.9)',
      '--bg-elevated-hover': '#faf8ea',
      '--bg-inset': '#f5f2dc',
      '--bg-hover': '#f5f2dc',
      '--bg-active': '#ede8cd',
      '--border-subtle': '#e8e4c8',
      '--border-default': '#d9d2ab',
      '--border-strong': '#b8b086',
    },
  },
  {
    id: 'peach-coral',
    nameKey: 'settings:scheme_peach_coral',
    mode: 'light',
    preview: { bg: '#fcf7f4', elevated: '#fffdfb', accent: '#c98e72' },
    variables: {
      '--bg-base': '#fcf7f4',
      '--bg-elevated': '#fffdfb',
      '--bg-toolbar': 'rgba(255, 253, 251, 0.9)',
      '--bg-elevated-hover': '#faf4ef',
      '--bg-inset': '#f5ebe4',
      '--bg-hover': '#f5ebe4',
      '--bg-active': '#edddd2',
      '--border-subtle': '#e8d9cf',
      '--border-default': '#d9c3b5',
      '--border-strong': '#bfa390',
    },
  },
  {
    id: 'arctic-ice',
    nameKey: 'settings:scheme_arctic_ice',
    mode: 'light',
    preview: { bg: '#f2f6f8', elevated: '#f9fbfc', accent: '#6b8fa3' },
    variables: {
      '--bg-base': '#f2f6f8',
      '--bg-elevated': '#f9fbfc',
      '--bg-toolbar': 'rgba(249, 251, 252, 0.9)',
      '--bg-elevated-hover': '#eef3f5',
      '--bg-inset': '#e4ecef',
      '--bg-hover': '#e4ecef',
      '--bg-active': '#d5e1e6',
      '--border-subtle': '#c8d8df',
      '--border-default': '#b0c4cc',
      '--border-strong': '#8ba3ad',
    },
  },
  {
    id: 'paper-white',
    nameKey: 'settings:scheme_paper_white',
    mode: 'light',
    preview: { bg: '#f7f7f7', elevated: '#ffffff', accent: '#5b7c99' },
    variables: {
      '--bg-base': '#f7f7f7',
      '--bg-elevated': '#ffffff',
      '--bg-toolbar': 'rgba(255, 255, 255, 0.9)',
      '--bg-elevated-hover': '#f6f6f6',
      '--bg-inset': '#ededed',
      '--bg-hover': '#ededed',
      '--bg-active': '#e2e2e2',
      '--border-subtle': '#dbdbdb',
      '--border-default': '#c8c8c8',
      '--border-strong': '#a0a0a0',
    },
  },
];

const DARK_SCHEMES: SurfaceScheme[] = [
  {
    id: 'warm-stone-dark',
    nameKey: 'settings:scheme_warm_stone_dark',
    mode: 'dark',
    preview: { bg: '#1f1c18', elevated: '#2a2620', accent: '#7a9ab5' },
    variables: {
      '--bg-base': '#1f1c18',
      '--bg-elevated': '#2a2620',
      '--bg-toolbar': 'rgba(42, 38, 32, 0.92)',
      '--bg-elevated-hover': '#322d26',
      '--bg-inset': 'rgba(26, 23, 20, 0.5)',
      '--bg-hover': '#353029',
      '--bg-active': '#423b33',
      '--border-subtle': '#3d3831',
      '--border-default': '#4a433b',
      '--border-strong': '#5c5549',
    },
  },
  {
    id: 'deep-ocean',
    nameKey: 'settings:scheme_deep_ocean',
    mode: 'dark',
    preview: { bg: '#181c24', elevated: '#21262f', accent: '#7a9ab5' },
    variables: {
      '--bg-base': '#181c24',
      '--bg-elevated': '#21262f',
      '--bg-toolbar': 'rgba(33, 38, 47, 0.92)',
      '--bg-elevated-hover': '#282e38',
      '--bg-inset': 'rgba(16, 19, 24, 0.5)',
      '--bg-hover': '#2c3340',
      '--bg-active': '#373f4d',
      '--border-subtle': '#313742',
      '--border-default': '#3d4552',
      '--border-strong': '#4f5866',
    },
  },
  {
    id: 'midnight',
    nameKey: 'settings:scheme_midnight',
    mode: 'dark',
    preview: { bg: '#141416', elevated: '#1e1e20', accent: '#b06b7a' },
    variables: {
      '--bg-base': '#141416',
      '--bg-elevated': '#1e1e20',
      '--bg-toolbar': 'rgba(30, 30, 32, 0.92)',
      '--bg-elevated-hover': '#26262a',
      '--bg-inset': 'rgba(10, 10, 12, 0.5)',
      '--bg-hover': '#28282c',
      '--bg-active': '#333338',
      '--border-subtle': '#2c2c30',
      '--border-default': '#38383c',
      '--border-strong': '#4a4a4e',
    },
  },
  {
    id: 'forest-night',
    nameKey: 'settings:scheme_forest_night',
    mode: 'dark',
    preview: { bg: '#1a211d', elevated: '#242d28', accent: '#7aaf8f' },
    variables: {
      '--bg-base': '#1a211d',
      '--bg-elevated': '#242d28',
      '--bg-toolbar': 'rgba(36, 45, 40, 0.92)',
      '--bg-elevated-hover': '#2d3831',
      '--bg-inset': 'rgba(14, 18, 16, 0.5)',
      '--bg-hover': '#303c35',
      '--bg-active': '#3a4740',
      '--border-subtle': '#354039',
      '--border-default': '#3f4b43',
      '--border-strong': '#536058',
    },
  },
  {
    id: 'dusk-purple',
    nameKey: 'settings:scheme_dusk_purple',
    mode: 'dark',
    preview: { bg: '#211f26', elevated: '#2d2a34', accent: '#9e92b8' },
    variables: {
      '--bg-base': '#211f26',
      '--bg-elevated': '#2d2a34',
      '--bg-toolbar': 'rgba(45, 42, 52, 0.92)',
      '--bg-elevated-hover': '#36323e',
      '--bg-inset': 'rgba(18, 16, 22, 0.5)',
      '--bg-hover': '#393542',
      '--bg-active': '#454050',
      '--border-subtle': '#3f3a4a',
      '--border-default': '#4b4556',
      '--border-strong': '#5f586b',
    },
  },
  {
    id: 'charcoal-slate',
    nameKey: 'settings:scheme_charcoal_slate',
    mode: 'dark',
    preview: { bg: '#1c1e21', elevated: '#26282c', accent: '#8a9bb0' },
    variables: {
      '--bg-base': '#1c1e21',
      '--bg-elevated': '#26282c',
      '--bg-toolbar': 'rgba(38, 40, 44, 0.92)',
      '--bg-elevated-hover': '#2f3237',
      '--bg-inset': 'rgba(14, 15, 17, 0.5)',
      '--bg-hover': '#313439',
      '--bg-active': '#3c3f45',
      '--border-subtle': '#363a40',
      '--border-default': '#42464d',
      '--border-strong': '#555a61',
    },
  },
  {
    id: 'espresso-bean',
    nameKey: 'settings:scheme_espresso_bean',
    mode: 'dark',
    preview: { bg: '#231e1b', elevated: '#2f2824', accent: '#a88b6e' },
    variables: {
      '--bg-base': '#231e1b',
      '--bg-elevated': '#2f2824',
      '--bg-toolbar': 'rgba(47, 40, 36, 0.92)',
      '--bg-elevated-hover': '#383029',
      '--bg-inset': 'rgba(16, 13, 11, 0.5)',
      '--bg-hover': '#3b322c',
      '--bg-active': '#483d36',
      '--border-subtle': '#453d36',
      '--border-default': '#524840',
      '--border-strong': '#665c52',
    },
  },
  {
    id: 'burgundy-wine',
    nameKey: 'settings:scheme_burgundy_wine',
    mode: 'dark',
    preview: { bg: '#241b1e', elevated: '#32262a', accent: '#b07a85' },
    variables: {
      '--bg-base': '#241b1e',
      '--bg-elevated': '#32262a',
      '--bg-toolbar': 'rgba(50, 38, 42, 0.92)',
      '--bg-elevated-hover': '#3b2d32',
      '--bg-inset': 'rgba(16, 11, 13, 0.5)',
      '--bg-hover': '#3e3035',
      '--bg-active': '#4a3a40',
      '--border-subtle': '#46363b',
      '--border-default': '#544046',
      '--border-strong': '#6a545a',
    },
  },
  {
    id: 'teal-depth',
    nameKey: 'settings:scheme_teal_depth',
    mode: 'dark',
    preview: { bg: '#182322', elevated: '#222f2e', accent: '#6ba89c' },
    variables: {
      '--bg-base': '#182322',
      '--bg-elevated': '#222f2e',
      '--bg-toolbar': 'rgba(34, 47, 46, 0.92)',
      '--bg-elevated-hover': '#2a3937',
      '--bg-inset': 'rgba(10, 15, 15, 0.5)',
      '--bg-hover': '#2d3d3b',
      '--bg-active': '#364b48',
      '--border-subtle': '#2f403d',
      '--border-default': '#394f4a',
      '--border-strong': '#4a635d',
    },
  },
  {
    id: 'obsidian-black',
    nameKey: 'settings:scheme_obsidian_black',
    mode: 'dark',
    preview: { bg: '#121214', elevated: '#1c1c1e', accent: '#7a8a9a' },
    variables: {
      '--bg-base': '#121214',
      '--bg-elevated': '#1c1c1e',
      '--bg-toolbar': 'rgba(28, 28, 30, 0.92)',
      '--bg-elevated-hover': '#242426',
      '--bg-inset': 'rgba(8, 8, 10, 0.5)',
      '--bg-hover': '#262628',
      '--bg-active': '#303032',
      '--border-subtle': '#2a2a2c',
      '--border-default': '#353537',
      '--border-strong': '#47474a',
    },
  },
];

export const THEME_SCHEMES: ThemeScheme[] = [...LIGHT_SCHEMES, ...DARK_SCHEMES].map((scheme) => ({
  ...scheme,
  preview: { ...scheme.preview, text: NEUTRAL_TEXT[scheme.mode]['--text-primary'] },
  variables: { ...scheme.variables, ...NEUTRAL_TEXT[scheme.mode] },
}));

export function getSchemeById(id: string | null): ThemeScheme | undefined {
  return THEME_SCHEMES.find((s) => s.id === id);
}

export function getSchemesByMode(mode: 'light' | 'dark'): ThemeScheme[] {
  return THEME_SCHEMES.filter((s) => s.mode === mode);
}

/** Apply a scheme's CSS variables to the document root. */
export function applyScheme(id: string | null) {
  const root = document.documentElement;
  // 清理旧版色板遗留的行内装饰文字色；强调色仍由独立选择器管理。
  root.style.removeProperty('--text-accent');
  root.style.removeProperty('--text-warm');
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
function clearScheme() {
  const root = document.documentElement;
  const firstScheme = THEME_SCHEMES[0];
  if (!firstScheme) return;
  Object.keys(firstScheme.variables).forEach((key) => {
    root.style.removeProperty(key);
  });
}

/** Resolve which scheme should currently be active.
 *  When themePreset is 'system', use the provided systemTheme if available
 *  (detected by Rust backend), otherwise fall back to matchMedia. */

export function resolveActiveScheme(
  themePreset: ThemePreset,
  defaultLightTheme: string,
  defaultDarkTheme: string,
  resolvedSystemTheme?: 'light' | 'dark',
): string {
  if (themePreset === 'system') {
    const isDark =
      resolvedSystemTheme !== undefined
        ? resolvedSystemTheme === 'dark'
        : window.matchMedia('(prefers-color-scheme: dark)').matches;
    return isDark ? defaultDarkTheme : defaultLightTheme;
  }
  return themePreset === 'warm-stone-dark' ? defaultDarkTheme : defaultLightTheme;
}
