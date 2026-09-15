import { expect, test } from '@playwright/test';
import { login, setupTauriMock } from './fixtures/auth';

test.beforeEach(async ({ page }) => {
  await setupTauriMock(page);
  await page.addInitScript(() => {
    Object.assign(window, {
      __MOCK_PLATFORM__: 'macos',
      __E2E_MOCKS__: {
        vault_check_directory: () => true,
        ocr_get_model_status: () => ({ installed: true, bundled: true }),
        sync_list_conflicts: () => [],
        object_trash_list: () => [],
        attachment_count_stats: () => ({ attachmentCount: 0, photoCount: 0 }),
        set_titlebar_color: () => ({
          material: 'liquid-glass',
          platform: 'macos',
          reduceMotion: false,
          highContrast: false,
          titlebarHeight: 52,
          trafficLightsRight: 79,
        }),
        get_window_layout: () => ({
          platform: 'macos',
          titlebarHeight: 52,
          trafficLightsRight: 79,
        }),
      },
    });
    localStorage.setItem('i18nextLng', 'en-US');
  });
  await login(page);
});

for (const [name, path] of [
  ['Identity', '/workspace?section=identity'],
  ['Settings', '/settings'],
  ['Search', '/search'],
  ['Trash', '/settings/trash'],
  ['Attachments', '/settings/attachments'],
  ['OCR', '/ocr'],
  ['Device Sync', '/sync'],
  ['Help', '/help'],
  ['AI Chat', '/llm-chat'],
]) {
  test(`${name} 返回首页不卸载窗口布局`, async ({ page }) => {
    const main = await page.locator('main').elementHandle();
    const header = await page.locator('header[data-appbar]').elementHandle();
    const shell = await page.locator('[data-navigation]').elementHandle();

    // 走真实首页入口及真实页面的返回处理，防止只测模拟路由而遗漏 /home 重定向。
    await page
      .locator('main [role="button"]')
      .filter({ has: page.getByRole('heading', { name, exact: true }) })
      .click();
    await expect(page).toHaveURL(`http://localhost:1420${path}`);
    await page
      .locator('header[data-appbar]')
      .getByRole('button', { name: 'Back', exact: true })
      .click();
    await expect(page).toHaveURL('http://localhost:1420/');
    await expect(page.locator('main').getByRole('heading', { name: /Welcome back/ })).toBeVisible();

    // 最终首页可见并不足以证明没有闪烁：错误路径会先卸载整壳，再重定向挂载首页。
    for (const element of [main, header, shell]) {
      expect(await element!.evaluate((node) => node.isConnected)).toBe(true);
      await element!.dispose();
    }
    await expect(page.locator('main')).toHaveJSProperty('scrollTop', 0);
    await expect(page.locator('header[data-appbar] h1')).toHaveText('Home');
  });
}
