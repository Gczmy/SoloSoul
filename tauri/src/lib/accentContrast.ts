type Rgb = [number, number, number];

function parseColor(value: string): Rgb | null {
  const color = value.trim();
  const hex = /^#([\da-f]{3}|[\da-f]{6})$/i.exec(color)?.[1];
  if (hex) {
    const full = hex.length === 3 ? [...hex].map((digit) => digit + digit).join('') : hex;
    return [0, 2, 4].map((offset) => parseInt(full.slice(offset, offset + 2), 16)) as Rgb;
  }
  // CSS 中的预设是 hex；同时接受浏览器序列化后的不透明 rgb。
  const rgb = /^rgb\(\s*([\d.]+)[,\s]+([\d.]+)[,\s]+([\d.]+)\s*\)$/i.exec(color);
  if (!rgb) return null;
  const channels = rgb.slice(1).map((channel) => Math.min(255, Number(channel)));
  return channels.every(Number.isFinite) ? (channels as Rgb) : null;
}

function luminance(rgb: Rgb): number {
  const linear = rgb.map((channel) => {
    const value = channel / 255;
    return value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
  });
  return linear[0] * 0.2126 + linear[1] * 0.7152 + linear[2] * 0.0722;
}

/** 在不透明强调色上选对比度更高的黑/白前景，普通与 hover 分别计算。 */
export function accentTextColor(background: string): '#000000' | '#ffffff' | null {
  const rgb = parseColor(background);
  if (!rgb) return null;
  const light = luminance(rgb);
  return (light + 0.05) / 0.05 >= 1.05 / (light + 0.05) ? '#000000' : '#ffffff';
}

/** 桌面浅色填色主按钮统一白字，仅在需要时等比例压低主题色亮度。
 * 共享强调色保持原值，深色模式保留较亮填色及其可读前景。 */
export function desktopAccentColors(background: string, theme: 'light' | 'dark') {
  const rgb = parseColor(background);
  if (!rgb) return null;
  if (theme === 'dark') return { background, foreground: accentTextColor(background)! };
  for (let percent = 100; percent >= 0; percent--) {
    const fill =
      percent === 100 ? rgb : (rgb.map((channel) => Math.floor((channel * percent) / 100)) as Rgb);
    if (1.05 / (luminance(fill) + 0.05) >= 4.5) {
      return {
        background: percent === 100 ? background : `rgb(${fill.join(', ')})`,
        foreground: '#ffffff',
      };
    }
  }
  return null;
}

/** 等价于原有的 sRGB + 12% black，但返回具体颜色，前景不依赖 color-mix 的解析时机。 */
export function customAccentHover(primary: string): string | null {
  const rgb = parseColor(primary);
  return rgb ? `rgb(${rgb.map((channel) => Math.round(channel * 0.88)).join(', ')})` : null;
}

export function applyAccentTextColors() {
  const root = document.documentElement;
  const computed = getComputedStyle(root);
  const theme = root.dataset.theme === 'dark' ? 'dark' : 'light';
  const desktopPrimary = desktopAccentColors(computed.getPropertyValue('--accent-primary'), theme);
  for (const state of ['primary', 'hover']) {
    const background = computed.getPropertyValue(`--accent-${state}`);
    const foreground = accentTextColor(background);
    if (foreground) root.style.setProperty(`--accent-${state}-text`, foreground);
    else root.style.removeProperty(`--accent-${state}-text`);
    // 浅色 hover 从已派生的按钮底色压低一级，避免亮色自定义主题两态都压到同一阈值。
    const desktopBackground =
      state === 'hover' && theme === 'light' && foreground && desktopPrimary
        ? customAccentHover(desktopPrimary.background)!
        : background;
    const pair = desktopAccentColors(desktopBackground, theme);
    const prefix = state === 'primary' ? '--desktop-primary' : '--desktop-primary-hover';
    for (const [suffix, color] of [
      ['fill', pair?.background],
      ['text', pair?.foreground],
    ]) {
      if (color) root.style.setProperty(`${prefix}-${suffix}`, color);
      else root.style.removeProperty(`${prefix}-${suffix}`);
    }
  }
}
