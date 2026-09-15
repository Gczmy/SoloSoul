import { expect, test } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { login } from './fixtures/auth';

for (const width of [800, 390]) {
  test(`${width}px 更多操作键盘可达，指南打开后可关闭并保留主操作`, async ({ page }) => {
    await page.setViewportSize({ width, height: 720 });
    await page.addInitScript({
      content:
        readFileSync('e2e/fixtures/tauriMock.js', 'utf8') +
        `
      window.__MOCK_PLATFORM__ = 'windows';
      window.__E2E_MOCKS__ = { vault_check_directory: () => true,
        ocr_get_model_status: () => ({ installed: true, bundled: true }), sync_list_conflicts: () => [] };
      localStorage.setItem('i18nextLng', 'en-US');
    `,
    });
    await login(page);
    const header = page.locator('[data-appbar]');
    const more = header.getByRole('button', { name: 'More actions', exact: true });
    const guide = header.getByRole('button', { name: 'Guide', exact: true });
    await expect(more).toBeVisible();
    await expect(guide).toBeHidden();
    await more.focus();
    await page.keyboard.press('Enter');
    await expect(guide).toBeFocused();
    await page.keyboard.press('Enter');
    await expect(page.getByRole('dialog')).toBeVisible();
    await page.keyboard.press('Escape');
    await expect(page.getByRole('dialog')).toHaveCount(0);
    await guide.focus();
    await page.keyboard.press('Escape');
    await expect(more).toBeFocused();
    await expect(guide).toBeHidden();
    if (width === 800) {
      await page.getByRole('button', { name: 'Identity', exact: true }).click();
      const create = header.getByRole('button', { name: '+ New', exact: true });
      await expect(create).toBeInViewport();
      await expect(create).toHaveCSS('font-size', '14px');
      const box = (await create.boundingBox())!;
      expect(box.y + box.height).toBeLessThanOrEqual(40);
      await create.click();
      await expect(page).toHaveURL(/\/editor\?section=identity/);
    } else {
      const box = (await more.boundingBox())!;
      expect(box.width).toBeGreaterThanOrEqual(44);
      expect(box.height).toBeGreaterThanOrEqual(44);
    }
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBe(width);
  });
}
