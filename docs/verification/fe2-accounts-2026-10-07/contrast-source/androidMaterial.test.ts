import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  androidMaterialTokens,
  applyAndroidMaterial,
  getAndroidMaterialSnapshot,
  materialHex,
  subscribeAndroidMaterial,
} from './androidMaterial';
import { THEME_SCHEMES } from './themeSchemes';

function luminance(hex: string) {
  const channels = [1, 3, 5].map((offset) => {
    const value = parseInt(hex.slice(offset, offset + 2), 16) / 255;
    return value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
  });
  return channels[0] * 0.2126 + channels[1] * 0.7152 + channels[2] * 0.0722;
}

describe('Android 使用已应用共享主题派生材质', () => {
  afterEach(() => {
    vi.doUnmock('@tauri-apps/plugin-os');
    vi.resetModules();
    document.documentElement.removeAttribute('style');
    delete document.documentElement.dataset.platform;
  });
  it('全部共享方案与极端自定义强调色保持主要文字和主操作对比', () => {
    for (const scheme of THEME_SCHEMES) {
      for (const accent of ['#5b7c99', '#7aaf8f', '#ffffff', '#000000', '#ffee00', '#112233']) {
        const colors = scheme.variables;
        const tokens = androidMaterialTokens({
          background: colors['--bg-base'],
          surface: colors['--bg-elevated'],
          inset: colors['--bg-inset'],
          foreground: colors['--text-primary'],
          secondary: colors['--text-secondary'],
          accent,
        });
        expect(tokens['--bg-base']).toBe(colors['--bg-base']);
        expect(tokens['--text-primary']).toBe(colors['--text-primary']);
        expect(tokens['--accent-primary']).toBe(accent);
        for (const [foreground, background] of [
          ['--text-primary', '--bg-base'],
          ['--md-on-primary', '--accent-primary'],
          ['--md-on-primary-container', '--md-primary-container'],
          ['--md-primary-ink', '--bg-base'],
          ['--md-primary-ink', '--bg-elevated'],
        ]) {
          const a = luminance(tokens[foreground]);
          const b = luminance(tokens[background]);
          expect((Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05)).toBeGreaterThanOrEqual(4.5);
        }
        Object.values(tokens).forEach((color) => expect(color).toMatch(/^#[\da-f]{6}$/));
      }
    }
  });
  it('原生与画布只得到不透明数值色，透明输入层按真实底色合成', () => {
    expect(materialHex('#abc')).toBe('#aabbcc');
    expect(materialHex('rgba(255, 0, 0, .5)', '#000000')).toBe('#800000');
    expect(materialHex('rgb(15, 30, 45)')).toBe('#0f1e2d');
  });
  it('同一深浅模式更换底色或强调色会交付新快照，不覆盖共享变量', () => {
    const root = document.documentElement;
    root.style.setProperty('--bg-base', '#112233');
    root.style.setProperty('--bg-elevated', '#223344');
    root.style.setProperty('--accent-primary', '#abcdef');
    const notify = vi.fn();
    const dispose = subscribeAndroidMaterial(notify);
    try {
      applyAndroidMaterial();
      const before = getAndroidMaterialSnapshot();
      expect(before.colors['--accent-primary']).toBe('#abcdef');
      expect(root.style.getPropertyValue('--bg-base')).toBe('#112233');
      expect(root.style.getPropertyValue('--text-primary')).toBe('');
      root.style.setProperty('--bg-elevated', '#445566');
      root.style.setProperty('--accent-primary', '#fedcba');
      applyAndroidMaterial();
      const after = getAndroidMaterialSnapshot();
      expect(after).not.toBe(before);
      expect(after.colors['--android-liquid-base']).toBe('#445566');
      expect(after.colors['--accent-primary']).toBe('#fedcba');
      expect(root.style.getPropertyValue('--md-primary-container')).toBe(
        after.colors['--md-primary-container'],
      );
      expect(notify).toHaveBeenCalledTimes(2);
      applyAndroidMaterial();
      expect(getAndroidMaterialSnapshot()).toBe(after);
      expect(notify).toHaveBeenCalledTimes(2);
    } finally {
      dispose();
    }
  });
  it.each(['android', 'macos', 'windows', 'ios'])(
    '平台 %s 在首屏前设置样式标记，只有 Android 启用新外观',
    async (platform) => {
      vi.resetModules();
      vi.doMock('@tauri-apps/plugin-os', () => ({ platform: () => platform }));
      const module = await import('./platform');
      await module.initPlatform();
      expect(document.documentElement.dataset.platform).toBe(platform);
      expect(module.isAndroidSync()).toBe(platform === 'android');
    },
  );
});
