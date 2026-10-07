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
    window.__STARTUP_HANDOFFS__ = [];
    window.addEventListener('solosoul:startup-handoff', () => {
      window.__STARTUP_HANDOFFS__.push({
        rootHasChildren: Boolean(document.getElementById('root')?.hasChildNodes()),
        marked: performance.getEntriesByName('solosoul:startup-dismissed', 'mark').length > 0,
        startup: window.__SOLOSOUL_STARTUP__?.diagnostic(),
      });
    });
    window.__E2E_MOCKS__ = {
      vault_check_directory: () => true,
      ocr_get_model_status: () => ({ installed: true, bundled: true }),
      sync_list_conflicts: () => [],
      get_app_info: () => ({ appName: 'SoloSoul', version: '1.0.0', os: 'windows', arch: 'x86_64' }),
      desktop_prepare_update: () =>
        location.pathname === '/about'
          ? {
              rid: 19,
              currentVersion: '1.0.0',
              version: '1.0.1',
              body: '# Release smoke\\n\\n- **Production Markdown renders**',
              rawJson: {},
            }
          : null,
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
  // 真实生产入口完成 React 提交后的交接，StrictMode 不得重复发出通知。
  expect(await page.evaluate('window.__STARTUP_HANDOFFS__')).toEqual([
    {
      rootHasChildren: true,
      marked: true,
      startup: { schemaVersion: 1, state: 'ready', phase: 'accounts', reason: 'none' },
    },
  ]);
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
