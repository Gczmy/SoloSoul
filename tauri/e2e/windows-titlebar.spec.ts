import { expect, test } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { login } from './fixtures/auth';

for (const position of ['left', 'right', 'top', 'bottom']) {
  test(`Windows ${position} 兼容顶栏紧凑且外壳同色`, async ({ page }) => {
    await page.setViewportSize({ width: 800, height: 600 });
    await page.addInitScript({
      content:
        readFileSync('e2e/fixtures/tauriMock.js', 'utf8') +
        `
      window.__MOCK_PLATFORM__ = 'windows';
      const layout = { platform: 'windows', titlebarHeight: 0 };
      window.__E2E_MOCKS__ = {
        vault_check_directory: () => true, ocr_get_model_status: () => ({ installed: true, bundled: true }),
        sync_list_conflicts: () => [], get_window_layout: () => layout,
        set_titlebar_color: () => ({ ...layout, material: 'mica', reduceMotion: false, highContrast: false }),
      };
      localStorage.setItem('i18nextLng', 'en-US');
      const invoke = window.__TAURI_INTERNALS__.invoke;
      window.__TAURI_INTERNALS__.invoke = async (cmd, args) => {
        const result = await invoke(cmd, args);
        if (cmd === 'user_data_get_preferences') result.sidebarPosition = '${position}';
        return result;
      };
    `,
    });
    await login(page);
    const header = page.locator('[data-appbar]');
    await expect(header).toHaveCSS('height', '40px');
    expect((await header.boundingBox())!.y).toBe(0);
    const guide = header.getByRole('button', { name: 'Guide', exact: true });
    await expect(guide).toBeVisible();
    const guideBounds = (await guide.boundingBox())!;
    expect(guideBounds.x + guideBounds.width).toBeLessThanOrEqual(800);
    const navigation =
      position === 'left' || position === 'right'
        ? page.locator('#desktop-navigation')
        : page.locator('header:not([data-appbar])');
    expect(await navigation.evaluate((el) => getComputedStyle(el).backgroundColor)).toBe(
      await header.evaluate((el) => getComputedStyle(el).backgroundColor),
    );
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBe(800);
    await page.screenshot({ path: `test-results/windows-titlebar-${position}.png` });
  });
}
