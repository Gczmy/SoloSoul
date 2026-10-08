import { afterEach, describe, expect, it, vi } from 'vitest';
import { accentTextColor, applyAccentTextColors, customAccentHover } from './accentContrast';
import { applyTheme } from './theme';
import { isAndroidSync } from './platform';

vi.mock('./platform', () => ({ isAndroidSync: vi.fn(() => false) }));
vi.mock('./nativeWindow', () => ({ syncNativeAppearance: vi.fn(async () => {}) }));
vi.mock('@/lib/ipcClient', () => ({ invokeCommand: vi.fn(async () => {}) }));

function contrast(foreground: string, background: string) {
  const luminance = (color: string) => {
    const channels = color.startsWith('#')
      ? [1, 3, 5].map((offset) => parseInt(color.slice(offset, offset + 2), 16))
      : (color.match(/[\d.]+/g) ?? []).map(Number);
    return channels.reduce((total, channel, index) => {
      const value = channel / 255;
      const linear = value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
      return total + linear * [0.2126, 0.7152, 0.0722][index];
    }, 0);
  };
  const a = luminance(foreground);
  const b = luminance(background);
  return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
}

describe('强调色上的文字对比', () => {
  afterEach(() => {
    document.documentElement.removeAttribute('style');
    document.documentElement.removeAttribute('data-theme');
    document.documentElement.removeAttribute('data-accent');
    vi.mocked(isAndroidSync).mockReturnValue(false);
  });

  it('各色相与亮度的填色均能选择至少 4.5 对比度的前景', () => {
    const levels = ['00', '20', '40', '60', '80', 'a0', 'c0', 'e0', 'ff'];
    for (const red of levels)
      for (const green of levels)
        for (const blue of levels) {
          const color = `#${red}${green}${blue}`;
          expect(contrast(accentTextColor(color)!, color)).toBeGreaterThanOrEqual(4.5);
        }
  });

  it.each(['#000000', '#ffffff', '#777777', '#808080', '#ffff00', '#0000ff', '#ff00ff'])(
    '自定义强调色 %s 与暗化 hover 各自使用可读前景',
    (color) => {
      const hover = customAccentHover(color)!;
      expect(contrast(accentTextColor(color)!, color)).toBeGreaterThanOrEqual(4.5);
      expect(contrast(accentTextColor(hover)!, hover)).toBeGreaterThanOrEqual(4.5);
    },
  );

  it('hover 跨过对比阈值时切换前景，不沿用普通态文字色', () => {
    expect(accentTextColor('#777777')).toBe('#000000');
    expect(accentTextColor(customAccentHover('#777777')!)).toBe('#ffffff');
  });

  it('读取最终 CSS 色值，更新普通与 hover token 并清理无法解析的旧值', () => {
    const root = document.documentElement;
    root.style.setProperty('--accent-primary', '#ffffff');
    root.style.setProperty('--accent-hover', 'rgb(0, 0, 0)');
    applyAccentTextColors();
    expect(root.style.getPropertyValue('--accent-primary-text')).toBe('#000000');
    expect(root.style.getPropertyValue('--accent-hover-text')).toBe('#ffffff');
    root.style.setProperty('--accent-primary', '#000000');
    root.style.setProperty('--accent-hover', 'unresolved');
    applyAccentTextColors();
    expect(root.style.getPropertyValue('--accent-primary-text')).toBe('#ffffff');
    expect(root.style.getPropertyValue('--accent-hover-text')).toBe('');
  });

  it('支持短 hex 与空格分隔 rgb，不把未解析的函数或错误 hex 当作黑色', () => {
    expect(accentTextColor('#fff')).toBe('#000000');
    expect(accentTextColor('rgb(255 255 255)')).toBe('#000000');
    for (const value of [
      '#12oops',
      'var(--accent-primary)',
      'color-mix(in srgb, white, black)',
      '',
    ]) {
      expect(accentTextColor(value)).toBeNull();
      expect(customAccentHover(value)).toBeNull();
    }
  });

  it('切换自定义/预设主题后重新读取最终色，Android 使用共享色板的可读前景', async () => {
    const css = document.createElement('style');
    // 通过 CSS 级联模拟预设，验证 JS 在主题切换完成后读取最终色，而非旧的 inline 值。
    css.textContent = `
      [data-accent='ocean'] { --accent-primary: #5b7c99; --accent-hover: #4a6a85; }
      [data-theme='dark'][data-accent='ocean'] { --accent-primary: #7a9ab5; --accent-hover: #8eafc8; }
      [data-theme='dark'][data-accent='amber'] { --accent-primary: #d4a76a; --accent-hover: #dbb88a; }
    `;
    document.head.append(css);
    const root = document.documentElement;
    const config = { backgroundType: 'solid', backgroundValue: '' } as const;
    try {
      await applyTheme({
        ...config,
        preset: 'warm-stone-light',
        accentColor: 'custom',
        customAccentHex: '#777',
      });
      expect(root.style.getPropertyValue('--accent-primary-text')).toBe('#000000');
      expect(root.style.getPropertyValue('--accent-hover-text')).toBe('#ffffff');
      await applyTheme({ ...config, preset: 'warm-stone-dark', accentColor: 'ocean' });
      expect(root.style.getPropertyValue('--accent-primary')).toBe('');
      expect(getComputedStyle(root).getPropertyValue('--accent-primary')).toBe('#7a9ab5');
      expect(root.style.getPropertyValue('--accent-primary-text')).toBe('#000000');
      expect(root.style.getPropertyValue('--accent-hover-text')).toBe('#000000');
      vi.mocked(isAndroidSync).mockReturnValue(true);
      await applyTheme({ ...config, preset: 'warm-stone-dark', accentColor: 'amber' });
      expect(root.style.getPropertyValue('--accent-primary-text')).toBe('#000000');
      expect(root.style.getPropertyValue('--accent-hover-text')).toBe('#000000');
      expect(root.style.getPropertyValue('--md-on-primary')).toBe('#000000');
      expect(getComputedStyle(root).getPropertyValue('--accent-primary')).toBe('#d4a76a');
    } finally {
      css.remove();
    }
  });
});
