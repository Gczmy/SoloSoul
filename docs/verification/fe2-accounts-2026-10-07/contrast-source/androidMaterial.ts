import { accentTextColor } from './accentContrast';
import { getSchemeById } from './themeSchemes';

export interface AndroidThemeColors {
  background: string;
  surface: string;
  inset: string;
  foreground: string;
  secondary: string;
  accent: string;
}

type Rgb = [number, number, number];
function channels(value: string): { rgb: Rgb; alpha: number } | null {
  const hex = /^#([\da-f]{3}|[\da-f]{6})$/i.exec(value.trim())?.[1];
  if (hex) {
    const full = hex.length === 3 ? [...hex].map((c) => c + c).join('') : hex;
    return { rgb: [0, 2, 4].map((i) => parseInt(full.slice(i, i + 2), 16)) as Rgb, alpha: 1 };
  }
  const match = /^rgba?\(([^)]+)\)$/.exec(value.trim());
  if (!match) return null;
  const values = match[1]
    .split(/[,\s/]+/)
    .filter(Boolean)
    .map(Number);
  if (values.length < 3 || values.length > 4 || !values.every(Number.isFinite)) return null;
  return {
    rgb: values.slice(0, 3).map((n) => Math.max(0, Math.min(255, n))) as Rgb,
    alpha: Math.max(0, Math.min(1, values[3] ?? 1)),
  };
}

/** 只解析应用主题的数值颜色，输出不透明 hex，兼容 WebGL 与原生 payload。 */
export function materialHex(value: string, background = '#000000'): string {
  const base = channels(background)?.rgb ?? [0, 0, 0];
  const color = channels(value);
  const rgb = color ? color.rgb.map((n, i) => n * color.alpha + base[i] * (1 - color.alpha)) : base;
  return `#${rgb.map((n) => Math.round(n).toString(16).padStart(2, '0')).join('')}`;
}

function mix(a: string, b: string, amount: number) {
  const first = channels(a)!.rgb;
  const second = channels(b)!.rgb;
  return materialHex(
    `rgb(${first.map((n, i) => n * (1 - amount) + second[i] * amount).join(',')})`,
  );
}

/** 无填色操作的强调色文字需要对比中性表面，而不是对比强调色自身。 */
function accentInk(accent: string, backgrounds: string[], fallback: string) {
  const luminance = (hex: string) =>
    channels(hex)!.rgb.reduce((total, channel, index) => {
      const value = channel / 255;
      const linear = value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
      return total + linear * [0.2126, 0.7152, 0.0722][index];
    }, 0);
  const surfaces = backgrounds.map(luminance);
  const readable = (color: string) => {
    const ink = luminance(color);
    return surfaces.every(
      (surface) => (Math.max(ink, surface) + 0.05) / (Math.min(ink, surface) + 0.05) >= 4.5,
    );
  };
  if (readable(accent)) return accent;
  // 只调整文字明度，保持原强调色填色与账户偏好；取能达标的最小混合量。
  for (let step = 1; step <= 255; step++) {
    for (const target of ['#000000', '#ffffff']) {
      const candidate = mix(accent, target, step / 255);
      if (readable(candidate)) return candidate;
    }
  }
  return fallback;
}

/** Material 名称是共享主题的派生别名，不再覆盖桌面的背景、文字或强调色。 */
export function androidMaterialTokens(colors: AndroidThemeColors): Record<string, string> {
  const background = materialHex(colors.background);
  const surface = materialHex(colors.surface, background);
  const accent = materialHex(colors.accent, background);
  const container = mix(surface, accent, 0.12);
  return {
    '--bg-base': background,
    '--bg-elevated': surface,
    '--text-primary': materialHex(colors.foreground, background),
    '--text-secondary': materialHex(colors.secondary, surface),
    '--accent-primary': accent,
    '--md-primary-container': container,
    '--md-on-primary-container': materialHex(colors.foreground, container),
    '--md-on-primary': accentTextColor(accent)!,
    '--md-primary-ink': accentInk(
      accent,
      [background, surface],
      materialHex(colors.foreground, surface),
    ),
    '--md-surface-high': materialHex(colors.inset, surface),
    '--md-secondary-container': mix(surface, accent, 0.06),
    '--md-on-secondary-container': materialHex(colors.foreground, surface),
    '--md-tertiary-container': mix(surface, accent, 0.18),
    '--md-on-tertiary-container': materialHex(colors.foreground, surface),
    '--android-liquid-base': surface,
  };
}

function initialColors(): AndroidThemeColors {
  const scheme = getSchemeById('warm-stone')!;
  return {
    background: scheme.variables['--bg-base'],
    surface: scheme.variables['--bg-elevated'],
    inset: scheme.variables['--bg-inset'],
    foreground: scheme.variables['--text-primary'],
    secondary: scheme.variables['--text-secondary'],
    accent: scheme.preview.accent,
  };
}

let snapshot = { dark: false, colors: androidMaterialTokens(initialColors()) };
const listeners = new Set<() => void>();
export const getAndroidMaterialSnapshot = () => snapshot;
export function subscribeAndroidMaterial(listener: () => void) {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/** 同一次主题交付的 DOM 是唯一输入；不再次解析设置或系统模式。 */
export function readAndroidMaterial() {
  const root = document.documentElement;
  const style = getComputedStyle(root);
  const defaults = initialColors();
  const read = (key: string, fallback: string) => style.getPropertyValue(key).trim() || fallback;
  return {
    dark: root.dataset.theme === 'dark',
    colors: androidMaterialTokens({
      background: read('--bg-base', defaults.background),
      surface: read('--bg-elevated', defaults.surface),
      inset: read('--bg-inset', defaults.inset),
      foreground: read('--text-primary', defaults.foreground),
      secondary: read('--text-secondary', defaults.secondary),
      accent: read('--accent-primary', defaults.accent),
    }),
  };
}

export function applyAndroidMaterial() {
  const next = readAndroidMaterial();
  const root = document.documentElement;
  Object.entries(next.colors).forEach(([key, value]) => {
    if (key.startsWith('--md-') || key.startsWith('--android-liquid-'))
      root.style.setProperty(key, value);
  });
  if (JSON.stringify(next) !== JSON.stringify(snapshot)) {
    snapshot = next;
    listeners.forEach((listener) => listener());
  }
}
