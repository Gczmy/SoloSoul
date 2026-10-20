import { test, expect } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { login } from './fixtures/auth';

for (const platform of ['android', 'ios']) {
  test(`${platform} wide viewport retains its existing navigation without a desktop directory`, async ({
    page,
  }) => {
    await page.setViewportSize({ width: 1000, height: 800 });
    await page.addInitScript({
      content:
        readFileSync('e2e/fixtures/tauriMock.js', 'utf8') +
        `
      window.__MOCK_PLATFORM__ = '${platform}';
      localStorage.setItem('i18nextLng', 'en-US');
      window.__E2E_MOCKS__ = {
        vault_check_directory: () => true,
        ocr_get_model_status: () => ({ installed: true, bundled: true }),
        sync_list_conflicts: () => []
      };
    `,
    });
    await login(page);
    await expect(page.locator('#desktop-page-pane')).toHaveCount(0);
    if (platform === 'ios') {
      const rail = page.locator('#desktop-navigation');
      await expect(rail).toHaveAttribute('data-expanded', 'true');
      await expect(rail).toHaveCSS('width', '186px');
      await rail.getByRole('button', { name: 'Collapse sidebar', exact: true }).click();
      await expect(rail).toHaveCSS('width', '64px');
    } else {
      await expect(page.locator('[data-android-shell]')).toBeVisible();
      await expect(page.locator('#desktop-navigation')).toHaveCount(0);
    }
  });
}
