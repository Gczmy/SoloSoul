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

/** 在不透明强调色上选对比度更高的黑/白前景，普通与 hover 分别计算。 */
export function accentTextColor(background: string): '#000000' | '#ffffff' | null {
  const rgb = parseColor(background);
  if (!rgb) return null;
  const linear = rgb.map((channel) => {
    const value = channel / 255;
    return value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
  });
  const luminance = linear[0] * 0.2126 + linear[1] * 0.7152 + linear[2] * 0.0722;
  return (luminance + 0.05) / 0.05 >= 1.05 / (luminance + 0.05) ? '#000000' : '#ffffff';
}

/** 等价于原有的 sRGB + 12% black，但返回具体颜色，前景不依赖 color-mix 的解析时机。 */
export function customAccentHover(primary: string): string | null {
  const rgb = parseColor(primary);
  return rgb ? `rgb(${rgb.map((channel) => Math.round(channel * 0.88)).join(', ')})` : null;
}

export function applyAccentTextColors() {
  const root = document.documentElement;
  const computed = getComputedStyle(root);
  for (const state of ['primary', 'hover']) {
    const foreground = accentTextColor(computed.getPropertyValue(`--accent-${state}`));
    if (foreground) root.style.setProperty(`--accent-${state}-text`, foreground);
    else root.style.removeProperty(`--accent-${state}-text`);
  }
}
