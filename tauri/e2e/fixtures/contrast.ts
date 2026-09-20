import type { Locator } from '@playwright/test';

/** 测量实色控件的最终 sRGB；浏览器负责解析 color-mix / alpha，避免字符串推断颜色。 */
export async function measureControlContrast(control: Locator, surfaceSelector: string) {
  return control.evaluate((element, selector) => {
    const surface = element.closest(selector);
    if (!surface) throw new Error(`Missing surrounding surface: ${selector}`);
    const canvas = document.createElement('canvas');
    canvas.width = 1;
    canvas.height = 1;
    const context = canvas.getContext('2d', { willReadFrequently: true })!;
    const sample = (colors: string[]) => {
      context.clearRect(0, 0, 1, 1);
      for (const color of colors) {
        context.fillStyle = color;
        context.fillRect(0, 0, 1, 1);
      }
      return [...context.getImageData(0, 0, 1, 1).data].slice(0, 3);
    };
    const layers = (node: Element) => {
      const chain = [];
      for (let current: Element | null = node; current; current = current.parentElement) {
        chain.unshift(getComputedStyle(current).backgroundColor);
      }
      return ['#ffffff', ...chain];
    };
    const luminance = (rgb: number[]) => {
      const linear = rgb.map((channel) => {
        const value = channel / 255;
        return value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
      });
      return linear[0] * 0.2126 + linear[1] * 0.7152 + linear[2] * 0.0722;
    };
    const contrast = (first: number[], second: number[]) => {
      const a = luminance(first);
      const b = luminance(second);
      return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
    };
    const style = getComputedStyle(element);
    const backgroundLayers = layers(element);
    const background = sample(backgroundLayers);
    const foreground = sample([...backgroundLayers, style.color]);
    const surrounding = sample(layers(surface));
    const borderVisible =
      parseFloat(style.borderTopWidth) > 0 && !['none', 'hidden'].includes(style.borderTopStyle);
    const border = sample([...backgroundLayers, style.borderTopColor]);
    const icon = element.querySelector('svg');
    const iconStyle = icon ? getComputedStyle(icon) : null;
    const iconStroke = iconStyle?.stroke;
    const iconColor =
      iconStyle && iconStroke && iconStroke !== 'none'
        ? iconStroke.toLowerCase() === 'currentcolor'
          ? iconStyle.color
          : iconStroke
        : style.color;
    const rect = element.getBoundingClientRect();
    return {
      foreground,
      background,
      surrounding,
      foregroundContrast: contrast(foreground, background),
      iconContrast: contrast(sample([...backgroundLayers, iconColor]), background),
      surfaceContrast: contrast(background, surrounding),
      borderContrast: borderVisible ? contrast(border, surrounding) : 1,
      backgroundImage: style.backgroundImage,
      width: rect.width,
      height: rect.height,
      borderRadius: parseFloat(style.borderTopLeftRadius),
    };
  }, surfaceSelector);
}
