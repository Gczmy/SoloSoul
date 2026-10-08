import { expect, test } from '@playwright/test';
import { login, setupTauriMock } from './fixtures/auth';

// 验证生产前端对原生过渡几何和最终通知的处理；不代表系统全屏动画验收。
test.use({ viewport: { width: 1200, height: 800 }, isMobile: false, hasTouch: false });

for (const theme of ['light', 'dark']) {
  test(`macOS ${theme}: 全屏过渡不压缩AppBar，最终通知恢复原生避让`, async ({ page }) => {
    await setupTauriMock(page);
    await page.addInitScript((theme) => {
      const layout = { platform: 'macos', titlebarHeight: 52, trafficLightsRight: 79 };
      let measurements = 0;
      Object.assign(window, {
        __MOCK_PLATFORM__: 'macos',
        __FULLSCREEN_LAYOUT__: layout,
        __FULLSCREEN_MEASUREMENTS__: () => measurements,
        __E2E_MOCKS__: {
          ui_get_preferences: () => ({ theme, hasSeenOnboarding: true, language: 'en-US' }),
          user_data_get_preferences: () => ({ theme, autoLockTimeoutMinutes: 0 }),
          set_titlebar_color: () => ({
            ...layout,
            material: 'liquid-glass',
            reduceMotion: false,
            highContrast: false,
          }),
          get_window_layout: () => {
            measurements += 1;
            return { ...layout };
          },
          vault_check_directory: () => true,
        },
      });
      localStorage.setItem('i18nextLng', 'en-US');
    }, theme);
    await login(page);
    const header = page.locator('header').first();
    const root = page.locator('html');
    await expect(header).toHaveCSS('height', '52px');
    const content = page.locator('[data-shell-content]');
    const originalTop = (await content.boundingBox())!.y;
    for (const height of [0, 32, 52, 0, 32, 52]) {
      await page.evaluate(async (height) => {
        const target = window as typeof window & {
          __FULLSCREEN_LAYOUT__: { titlebarHeight: number; trafficLightsRight: number };
          __TAURI_INTERNALS__: { invoke: (name: string, args: unknown) => Promise<unknown> };
        };
        target.__FULLSCREEN_LAYOUT__.titlebarHeight = height;
        target.__FULLSCREEN_LAYOUT__.trafficLightsRight = height === 0 ? 0 : 79;
        if (height === 32) window.dispatchEvent(new Event('resize'));
        else
          await target.__TAURI_INTERNALS__.invoke('plugin:event|emit', {
            event: 'native-window-layout-changed',
            payload: {},
          });
      }, height);
      // 每一阶段先确认新的原生测量已应用，避免把启动时52pt误算成恢复成功。
      await expect(root).toHaveCSS('--native-titlebar-height', `${height}px`);
      await expect(header).toHaveCSS('height', '52px');
      expect((await content.boundingBox())!.y).toBe(originalTop);
      await expect(
        page.getByRole('button', { name: 'Collapse sidebar', exact: true }),
      ).toBeVisible();
    }
    await page.getByRole('button', { name: 'Collapse sidebar', exact: true }).click();
    await page.getByRole('button', { name: 'Expand sidebar', exact: true }).click();
    await expect(header).toHaveCSS('height', '52px');
  });
}
