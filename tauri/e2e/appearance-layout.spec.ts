import { expect, test } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { login } from './fixtures/auth';

for (const platform of ['windows', 'macos']) {
  for (const width of [800, 1280]) {
    test(`${platform} ${width}px 外观页共享壳尺寸，配色面板不压缩表单`, async ({ page }) => {
      await page.setViewportSize({ width, height: 720 });
      await page.addInitScript({
        content:
          readFileSync('e2e/fixtures/tauriMock.js', 'utf8') +
          `
        window.__MOCK_PLATFORM__ = '${platform}';
        window.__E2E_MOCKS__ = { vault_check_directory: () => true,
          ocr_get_model_status: () => ({ installed: true, bundled: true }), sync_list_conflicts: () => [] };
        localStorage.setItem('i18nextLng', 'en-US');
      `,
      });
      await login(page);
      await page.getByRole('button', { name: 'Settings', exact: true }).click();
      await page.getByText('Theme & Appearance', { exact: true }).click();
      await expect(page).toHaveURL(/\/settings\/appearance$/);
      const more = page.getByRole('button', { name: 'More Appearances' });
      const before = (await more.boundingBox())!;
      await more.click();
      const panel = page.getByRole('region', { name: 'Theme Schemes' });
      await expect(panel).toBeVisible();
      const bounds = (await panel.boundingBox())!;
      expect(bounds.y).toBeGreaterThanOrEqual(48);
      expect(bounds.y + bounds.height).toBeLessThanOrEqual(720);
      const after = (await more.boundingBox())!;
      expect(after.width).toBeGreaterThanOrEqual(width === 800 ? before.width - 1 : 280);
      expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBe(width);
      await panel.getByRole('button', { name: 'Close' }).click();
      await expect(panel).toHaveCount(0);
      await expect(more).toBeInViewport();
    });
  }
}
