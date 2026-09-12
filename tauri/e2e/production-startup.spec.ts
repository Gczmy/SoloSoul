import { expect, test } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { login } from './fixtures/auth';

test('生产包完成启动并正常渲染 Markdown', async ({ page }) => {
  const pageErrors: string[] = [];
  page.on('pageerror', (error) => pageErrors.push(error.message));
  await page.addInitScript({
    content:
      readFileSync('e2e/fixtures/tauriMock.js', 'utf8') +
      `
    localStorage.setItem('i18nextLng', 'en-US');
    window.__E2E_MOCKS__ = {
      vault_check_directory: () => true,
      ocr_get_model_status: () => ({ installed: true, bundled: true }),
      sync_list_conflicts: () => [],
      get_app_info: () => ({ appName: 'SoloSoul', version: '1.0.0', os: 'windows', arch: 'x86_64' }),
      desktop_check_update: () => ({
        currentVersion: '1.0.0',
        latestVersion: location.pathname === '/about' ? '1.0.1' : '1.0.0',
        mandatory: false,
        releaseNotes: '# Release smoke\\n\\n- **Production Markdown renders**',
        publishedAt: null,
      }),
    };
  `,
  });
  await login(page);
  // 明确检查被执行的是打包入口，避免服务器复用把生产回归误跑在开发模式。
  await expect(page.locator('script[type="module"]').first()).toHaveAttribute(
    'src',
    /^\/assets\/.+\.js$/,
  );
  await expect(page.locator('#startup-screen')).toHaveCount(0);
  const sidebar = page.locator('#desktop-navigation');
  await expect(sidebar).toBeVisible();
  await sidebar.getByRole('button', { name: 'Settings', exact: true }).click();
  await expect(page).toHaveURL(/\/settings$/);
  await expect(page.getByRole('heading', { name: 'Settings', exact: true })).toBeVisible();
  await page.getByText('About', { exact: true }).click();
  await expect(page).toHaveURL(/\/about$/);
  await expect(page.getByRole('heading', { name: 'Release smoke', exact: true })).toBeVisible();
  await expect(page.locator('.release-notes-md li strong')).toHaveText(
    'Production Markdown renders',
  );
  expect(pageErrors).toEqual([]);
});
