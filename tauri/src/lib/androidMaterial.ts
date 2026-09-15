import type { AccentPreset } from '@/types';

/** 手动色板，复用现有 accent 偏好；并非 Android 壁纸动态取色。 */
export const ANDROID_PALETTES = ['ocean', 'forest', 'amber'] as const;
const palettes = {
  ocean: {
    light: ['#405F82', '#FFFFFF', '#D9E7F8', '#223E5E'],
    dark: ['#AFCBEC', '#153451', '#2C4560', '#D3E5FA'],
  },
  forest: {
    light: ['#4C6846', '#FFFFFF', '#DBE9CC', '#2E482A'],
    dark: ['#B9D2A7', '#20351A', '#3A5332', '#DEEFD2'],
  },
  amber: {
    light: ['#875449', '#FFFFFF', '#F6DED5', '#653D34'],
    dark: ['#EDB5A4', '#382219', '#613E32', '#FAE0D2'],
  },
  rose: {
    light: ['#895064', '#FFFFFF', '#FAD9E5', '#6C3549'],
    dark: ['#F2B3CA', '#501D31', '#6C3549', '#FAD9E5'],
  },
  purple: {
    light: ['#69548A', '#FFFFFF', '#ECDCFF', '#503D70'],
    dark: ['#D3BBF5', '#392650', '#503D70', '#ECDCFF'],
  },
};

export function androidMaterialTokens(dark: boolean, accent: AccentPreset): Record<string, string> {
  const palette = palettes[accent === 'custom' ? 'ocean' : accent] ?? palettes.ocean;
  const [primary, onPrimary, container, onContainer] = palette[dark ? 'dark' : 'light'];
  const surface = dark ? '#181D1B' : '#F9F9F3';
  const low = dark ? '#222925' : '#F0F1EA';
  const high = dark ? '#303934' : '#E7E9E1';
  return {
    '--bg-base': surface,
    '--bg-elevated': low,
    '--bg-toolbar': surface,
    '--bg-inset': high,
    '--bg-hover': high,
    '--bg-active': container,
    '--bg-elevated-hover': high,
    '--text-primary': dark ? '#E5E9E0' : '#20251F',
    '--text-secondary': dark ? '#B6C1B4' : '#626B62',
    '--text-tertiary': dark ? '#A2AEA3' : '#687269',
    '--border-subtle': dark ? '#3F4942' : '#D6DCD1',
    '--border-default': dark ? '#67736A' : '#8A958A',
    '--accent-primary': primary,
    '--accent-hover': primary,
    '--accent-bg': container,
    '--accent-bg-hover': container,
    '--md-primary-container': container,
    '--md-on-primary-container': onContainer,
    '--md-on-primary': onPrimary,
    '--md-surface-high': high,
    '--md-secondary-container': dark ? '#394530' : '#E4EACA',
    '--md-on-secondary-container': dark ? '#DEE8C8' : '#414C2E',
    '--md-tertiary-container': dark ? '#513E33' : '#F3DFD2',
    '--md-on-tertiary-container': dark ? '#F4DDCC' : '#735344',
    '--shadow-card': 'none',
    '--shadow-card-hover': 'none',
  };
}

export function applyAndroidMaterial(accent: AccentPreset) {
  const root = document.documentElement;
  Object.entries(androidMaterialTokens(root.dataset.theme === 'dark', accent)).forEach(
    ([key, value]) => root.style.setProperty(key, value),
  );
}
