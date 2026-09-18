import { expect, test, type Page } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { login } from './fixtures/auth';

const fileName = `SoloSoul_${'long_filename_'.repeat(24)}.apk`;
const notes = [
  '# Release notes',
  '',
  '**下载**',
  '',
  '| 平台 | 文件 |',
  '|------|------|',
  '| macOS | `SoloSoul_2.13.1_arm64.dmg` |',
  '| Windows | `SoloSoul_2.13.1_x64-setup.exe` |',
  `| Android | \`${fileName}\` |`,
].join('\n');

async function mockUpdate(page: Page, platform: string, mandatory = false) {
  await page.addInitScript({ content: readFileSync('e2e/fixtures/tauriMock.js', 'utf8') });
  await page.addInitScript(
    ({ notes, platform, mandatory }) => {
      Object.assign(window, {
        __MOCK_PLATFORM__: platform,
        __E2E_MOCKS__: {
          vault_check_directory: () => true,
          ui_get_preferences: () => ({ language: 'en-US', reduceMotion: true }),
          get_app_info: () => ({
            appName: 'SoloSoul',
            version: '2.13.1',
            os: platform,
            arch: 'arm64',
          }),
          desktop_prepare_update: () => ({
            rid: 19,
            current_version: '2.13.1',
            version: '9.0.0',
            body: notes,
            raw_json: {},
          }),
          desktop_check_update: () => ({
            currentVersion: '2.13.1',
            latestVersion: '9.0.0',
            releaseNotes: notes,
            mandatory: mandatory && location.pathname === '/about',
          }),
          android_check_update: () => ({
            currentVersion: '2.13.1',
            latestVersion: '9.0.0',
            releaseNotes: notes,
            checksum: 'a'.repeat(64),
            downloadUrl: 'https://example.com/test.apk',
            mandatory: mandatory && location.pathname === '/about',
          }),
        },
      });
      localStorage.setItem('i18nextLng', 'en-US');
    },
    { notes, platform, mandatory },
  );
}

async function expectTable(page: Page) {
  const table = page.getByRole('table').last();
  await expect(table.getByRole('row')).toHaveCount(4);
  await expect(table.getByRole('cell', { name: fileName })).toBeAttached();
  await table.scrollIntoViewIfNeeded();
  // 等待弹窗进入动画结束后测量边界。
  await expect
    .poll(() =>
      table.evaluate((element) => {
        const area = element.parentElement!;
        const notes = area.parentElement!;
        const dialog = element.closest('[role="dialog"]');
        const bounds = dialog?.getBoundingClientRect();
        return (
          notes.scrollWidth <= notes.clientWidth + 1 &&
          (!bounds || (bounds.left >= 8 && bounds.right <= innerWidth - 8)) &&
          document.documentElement.scrollWidth <= innerWidth
        );
      }),
    )
    .toBe(true);
}

for (const platform of ['windows', 'android']) {
  test(`${platform}：横幅与关于页的表格和长文件名保持在内容区`, async ({ page }) => {
    await page.setViewportSize({ width: platform === 'android' ? 360 : 1100, height: 900 });
    await mockUpdate(page, platform);
    await page.goto('/');
    await expect(page.locator('script[type="module"]').first()).toHaveAttribute(
      'src',
      /^\/assets\/.+\.js$/,
    );
    await page
      .getByRole('button', { name: /view release notes/i })
      .first()
      .click();
    await expectTable(page);
    await page.keyboard.press('Escape');
    await login(page);
    await page.evaluate(() => {
      history.pushState({}, '', '/about');
      dispatchEvent(new PopStateEvent('popstate'));
    });
    await expectTable(page);
  });
  test(`${platform}：强制更新说明的表格保持在弹窗内`, async ({ page }) => {
    await page.setViewportSize({ width: platform === 'android' ? 360 : 1100, height: 900 });
    await mockUpdate(page, platform, true);
    await login(page);
    await page.evaluate(() => {
      history.pushState({}, '', '/about');
      dispatchEvent(new PopStateEvent('popstate'));
    });
    await page
      .getByRole('button', { name: /view release notes/i })
      .last()
      .click();
    await expectTable(page);
  });
}

test('横幅不依赖额外 Markdown 桥接模块，断开旧动态入口仍直接渲染表格', async ({ page }) => {
  await mockUpdate(page, 'windows');
  let requests = 0;
  await page.route('**/assets/ReleaseNotesMarkdown-*.js', async (route) => {
    requests++;
    await route.abort('failed');
  });
  await page.goto('/');
  await page
    .getByRole('button', { name: /view release notes/i })
    .first()
    .click();
  await expectTable(page);
  await expect(page.locator('pre.release-notes-md')).toHaveCount(0);
  expect(requests).toBe(0);
});
