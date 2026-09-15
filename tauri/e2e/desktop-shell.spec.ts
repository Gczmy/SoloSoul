import { expect, test } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { login } from './fixtures/auth';

for (const position of ['left', 'right'] as const) {
  test(`${position} 侧栏展开、折叠、切页及快捷卡片位置`, async ({ page }) => {
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
    const sidebar = page.locator('#desktop-navigation');
    await expect(sidebar).toHaveAttribute('data-expanded', 'true');
    await expect(sidebar).toHaveCSS('width', '232px');
    await expect(
      sidebar.getByRole('button', { name: 'Settings', exact: true }).locator('span'),
    ).toBeVisible();
    await page.getByRole('button', { name: 'Collapse sidebar' }).click();
    await expect(sidebar).toHaveCSS('width', '96px');
    for (const name of ['Home', 'Settings', 'Add Page', 'Tools']) {
      const button = sidebar.getByRole('button', { name, exact: true });
      const label = button.locator('span').last();
      await expect(label).toBeVisible();
      const iconBounds = (await button.locator('svg').boundingBox())!;
      const labelBounds = (await label.boundingBox())!;
      expect(labelBounds.y).toBeGreaterThanOrEqual(iconBounds.y + iconBounds.height);
      expect(labelBounds.x + labelBounds.width).toBeLessThanOrEqual(
        (await button.boundingBox())!.x + (await button.boundingBox())!.width,
      );
    }
    await sidebar.getByRole('button', { name: 'Settings', exact: true }).click();
    await expect(page).toHaveURL(/\/settings$/);
    await expect(sidebar).toHaveAttribute('data-expanded', 'false');
    await page.getByRole('button', { name: 'Expand sidebar' }).click();
    await expect(sidebar).toHaveCSS('width', '232px');
    expect(await page.evaluate(() => localStorage.getItem('solosoul_sidebar_expanded'))).toBe(
      'true',
    );
    const navBounds = await sidebar.boundingBox();
    const headerBounds = await page.locator('header').first().boundingBox();
    expect(navBounds).not.toBeNull();
    expect(headerBounds).not.toBeNull();
    if (position === 'left')
      expect(headerBounds!.x).toBeGreaterThanOrEqual(navBounds!.x + navBounds!.width);
    else expect(headerBounds!.x + headerBounds!.width).toBeLessThanOrEqual(navBounds!.x + 1);
    await sidebar.getByRole('button', { name: 'Plugins', exact: true }).click();
    const panel = page.getByRole('dialog', { name: 'Plugins', exact: true });
    await expect(panel).toBeVisible();
    const panelBounds = await panel.boundingBox();
    if (position === 'left') expect(panelBounds!.x).toBeGreaterThanOrEqual(navBounds!.width);
    else expect(panelBounds!.x + panelBounds!.width).toBeLessThanOrEqual(navBounds!.x);
    await page.keyboard.press('Escape');
    await expect(panel).toHaveCount(0);
    await page.screenshot({ path: `test-results/desktop-${position}.png` });
  });
}

test('窄视口继续使用底部导航，桌面展开状态不挤占内容', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.addInitScript({ path: 'e2e/fixtures/tauriMock.js' });
  await login(page);
  await expect(page.locator('#desktop-navigation')).toHaveCount(0);
  const overflow = await page.evaluate(
    () => document.documentElement.scrollWidth > window.innerWidth,
  );
  expect(overflow).toBe(false);
});

for (const position of ['top', 'bottom'] as const) {
  test(`${position} 导航保持横向布局`, async ({ page }) => {
    await page.addInitScript({
      content:
        readFileSync('e2e/fixtures/tauriMock.js', 'utf8') +
        `
      window.__E2E_MOCKS__ = {
        vault_check_directory: () => true,
        ocr_get_model_status: () => ({ installed: true, bundled: true }),
        sync_list_conflicts: () => [],
      };
      const originalInvoke = window.__TAURI_INTERNALS__.invoke;
      window.__TAURI_INTERNALS__.invoke = async (cmd, args) => {
        const result = await originalInvoke(cmd, args);
        if (cmd === 'user_data_get_preferences') result.sidebarPosition = '${position}';
        return result;
      };
    `,
    });
    await login(page);
    await expect(page.locator('#desktop-navigation')).toHaveCount(0);
    await expect(page.locator('header')).toHaveCount(2);
    const rectangles = await page.locator('header').evaluateAll((elements) =>
      elements.map((el) => {
        const r = el.getBoundingClientRect();
        return { top: r.top, bottom: r.bottom, left: r.left, width: r.width };
      }),
    );
    expect(rectangles.every((r) => r.left === 0 && r.width === 1280)).toBe(true);
    expect(
      rectangles[0].bottom <= rectangles[1].top || rectangles[1].bottom <= rectangles[0].top,
    ).toBe(true);
  });
}

test('深色主题和减少动态效果在最小桌面窗口保持可用', async ({ page }) => {
  await page.setViewportSize({ width: 800, height: 600 });
  await page.emulateMedia({ colorScheme: 'dark', reducedMotion: 'reduce' });
  await page.addInitScript({
    content:
      readFileSync('e2e/fixtures/tauriMock.js', 'utf8') +
      `
    window.__MOCK_PLATFORM__ = 'windows';
    window.__E2E_MOCKS__ = {
      vault_check_directory: () => true,
      ocr_get_model_status: () => ({ installed: true, bundled: true }),
      sync_list_conflicts: () => [],
      set_titlebar_color: () => ({
        material: 'solid', platform: 'windows', reduceMotion: true, highContrast: false,
      }),
    };
    localStorage.setItem('i18nextLng', 'en-US');
    const originalInvoke = window.__TAURI_INTERNALS__.invoke;
    window.__TAURI_INTERNALS__.invoke = async (cmd, args) => {
      if (cmd === 'get_system_theme') return 'dark';
      const result = await originalInvoke(cmd, args);
      if (cmd === 'ui_get_preferences' || cmd === 'user_data_get_preferences') {
        result.theme = 'dark';
      }
      return result;
    };
  `,
  });
  await page.goto('/login');
  const root = page.locator('html');
  await expect(root).toHaveAttribute('data-theme', 'dark');
  await expect(page.locator('#startup-screen')).toHaveCount(0);
  const submit = page.locator('button[type="submit"]');
  await expect(submit).toBeInViewport();
  await page.screenshot({ path: 'test-results/desktop-login-dark.png' });
  await page.locator('input[type="text"]').fill('any-password');
  await submit.click();
  await page.waitForURL('/');
  const sidebar = page.locator('#desktop-navigation');
  await sidebar.getByRole('button', { name: 'Settings', exact: true }).click();
  await expect(root).toHaveAttribute('data-theme', 'dark');
  await expect(root).toHaveAttribute('data-reduce-motion', 'true');
  await expect(page.getByRole('button', { name: 'Collapse sidebar' })).toBeInViewport();
  await expect(sidebar.getByRole('button', { name: 'Settings', exact: true })).toBeInViewport();
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBe(800);
  const navBounds = await sidebar.boundingBox();
  const contentBounds = await page.locator('main').boundingBox();
  expect(contentBounds!.x).toBeGreaterThanOrEqual(navBounds!.x + navBounds!.width);
  expect(contentBounds!.width).toBeGreaterThan(500);
  await page.getByRole('button', { name: 'Collapse sidebar' }).click();
  await expect(sidebar).toHaveCSS('width', '96px');
  await expect(
    sidebar.getByRole('button', { name: 'Settings', exact: true }).locator('span'),
  ).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBe(800);
  await page.screenshot({ path: 'test-results/desktop-minimum-dark.png' });
});
