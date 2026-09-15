import { expect, test } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { login } from './fixtures/auth';

for (const position of ['left', 'right', 'top', 'bottom']) {
  test(`${position} 折叠工具区退出 Tab 顺序，键盘可展开并用 Escape 返回`, async ({ page }) => {
    await page.addInitScript({
      content:
        readFileSync('e2e/fixtures/tauriMock.js', 'utf8') +
        `
      window.__MOCK_PLATFORM__ = 'windows';
      window.__E2E_MOCKS__ = {
        vault_check_directory: () => true,
        ocr_get_model_status: () => ({ installed: true, bundled: true }),
        sync_list_conflicts: () => [],
      };
      localStorage.setItem('i18nextLng', 'en-US');
      const originalInvoke = window.__TAURI_INTERNALS__.invoke;
      window.__TAURI_INTERNALS__.invoke = async (cmd, args) => {
        const result = await originalInvoke(cmd, args);
        if (cmd === 'user_data_get_preferences') result.sidebarPosition = '${position}';
        return result;
      };
    `,
    });
    await login(page);
    if (position === 'left' || position === 'right') {
      await page.getByRole('button', { name: 'Collapse sidebar', exact: true }).click();
    }
    await page.mouse.move(700, 400);
    const toggle = page.getByRole('button', { name: 'Tools', exact: true });
    await expect(toggle).toHaveAttribute('aria-expanded', 'false');
    await toggle.focus();
    await page.keyboard.press('Tab');
    const nextName = await page.locator(':focus').getAttribute('aria-label');
    expect(nextName).toMatch(/Lock/);
    await toggle.focus();
    await page.keyboard.press('Enter');
    await expect(toggle).toHaveAttribute('aria-expanded', 'true');
    await page.keyboard.press('Tab');
    await expect
      .poll(() => page.locator(':focus').evaluate((el) => !!el.closest('[inert]')))
      .toBe(false);
    expect(await page.locator(':focus').getAttribute('aria-label')).not.toMatch(/Lock/);
    await page.keyboard.press('Escape');
    await expect(toggle).toBeFocused();
    await expect(toggle).toHaveAttribute('aria-expanded', 'false');
  });
}
