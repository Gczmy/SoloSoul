import { afterEach, describe, expect, it, vi } from 'vitest';
import { ANDROID_PALETTES, androidMaterialTokens } from './androidMaterial';

function luminance(hex: string) {
  const channels = [1, 3, 5].map((offset) => {
    const value = parseInt(hex.slice(offset, offset + 2), 16) / 255;
    return value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
  });
  return channels[0] * 0.2126 + channels[1] * 0.7152 + channels[2] * 0.0722;
}

describe('Android Material 的可读性与平台门控', () => {
  afterEach(() => {
    vi.doUnmock('@tauri-apps/plugin-os');
    vi.resetModules();
    delete document.documentElement.dataset.platform;
  });
  it('三套色板在深浅模式下的主要文字/背景对比度至少为 4.5', () => {
    for (const dark of [false, true])
      for (const palette of ANDROID_PALETTES) {
        const tokens = androidMaterialTokens(dark, palette);
        for (const [foreground, background] of [
          ['--text-primary', '--bg-base'],
          ['--text-secondary', '--bg-elevated'],
          ['--md-on-primary', '--accent-primary'],
          ['--md-on-primary-container', '--md-primary-container'],
        ]) {
          const a = luminance(tokens[foreground]);
          const b = luminance(tokens[background]);
          expect((Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05)).toBeGreaterThanOrEqual(4.5);
        }
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
