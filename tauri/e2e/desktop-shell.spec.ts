import { expect, test, type Page } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { DESKTOP_SIDEBAR_COLLAPSED_WIDTH } from '../src/lib/constants';
import { login } from './fixtures/auth';

async function expectMobileNavigation(page: Page) {
  await expect(page.locator('#desktop-navigation')).toHaveCount(0);
  const bottomNav = page.getByTestId('mobile-bottom-nav');
  await expect(bottomNav).toBeVisible();
  await expect(bottomNav).toHaveCSS('position', 'fixed');
  await expect(bottomNav.locator('button[aria-current="page"]')).toHaveCount(1);
  const bounds = await bottomNav.boundingBox();
  expect(bounds).not.toBeNull();
  expect(bounds!.x).toBe(0);
  expect(bounds!.width).toBe(page.viewportSize()!.width);
}

for (const position of ['left', 'right', 'top', 'bottom'] as const) {
  test(`旧 ${position} 配置统一左侧栏，展开、折叠、切页及快捷卡片位置正常`, async ({
    page,
  }, testInfo) => {
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
    if (testInfo.project.name === 'mobile') {
      await expectMobileNavigation(page);
      const bottomNav = page.getByTestId('mobile-bottom-nav');
      const toggle = bottomNav.locator('button[aria-expanded]');
      await expect(toggle).toHaveAttribute('aria-expanded', 'false');
      await toggle.click();
      await expect(toggle).toHaveAttribute('aria-expanded', 'true');
      await toggle.click();
      await expect(toggle).toHaveAttribute('aria-expanded', 'false');
      await bottomNav.getByRole('button', { name: 'Settings', exact: true }).click();
      await expect(page).toHaveURL(/\/settings$/);
      return;
    }
    await expect(sidebar).toHaveAttribute('data-position', 'left');
    await expect(sidebar).toHaveAttribute('data-expanded', 'false');
    await expect(sidebar).toHaveCSS('width', `${DESKTOP_SIDEBAR_COLLAPSED_WIDTH}px`);
    await expect(
      sidebar.getByRole('button', { name: 'Settings', exact: true }).locator('span'),
    ).toBeHidden();
    await page.getByRole('button', { name: 'Hide pages and objects' }).click();
    await expect(page.locator('[data-page-pane-body]')).not.toHaveAttribute('data-pane-animating');
    await expect(sidebar).toHaveCSS('width', `${DESKTOP_SIDEBAR_COLLAPSED_WIDTH}px`);
    for (const name of ['Home', 'Settings', 'Add Page', 'Tools']) {
      const button = sidebar.getByRole('button', { name, exact: true });
      const label = button.locator('span').last();
      await expect(label).toBeHidden();
      const iconBounds = (await button.locator('svg').boundingBox())!;
      const buttonBounds = (await button.boundingBox())!;
      expect(buttonBounds.width).toBeGreaterThanOrEqual(44);
      expect(buttonBounds.height).toBeGreaterThanOrEqual(44);
      expect(
        Math.abs(iconBounds.x + iconBounds.width / 2 - buttonBounds.x - buttonBounds.width / 2),
      ).toBeLessThan(1);
      expect(
        Math.abs(iconBounds.y + iconBounds.height / 2 - buttonBounds.y - buttonBounds.height / 2),
      ).toBeLessThan(1);
    }
    await sidebar.getByRole('button', { name: 'Settings', exact: true }).click();
    await expect(page).toHaveURL(/\/settings$/);
    await expect(sidebar).toHaveAttribute('data-expanded', 'false');
    await page.getByRole('button', { name: 'Show pages and objects' }).click();
    await expect(page.locator('[data-page-pane-body]')).not.toHaveAttribute('data-pane-animating');
    await expect(sidebar).toHaveCSS('width', `${DESKTOP_SIDEBAR_COLLAPSED_WIDTH}px`);
    expect(await page.evaluate(() => localStorage.getItem('solosoul_page_pane_open'))).toBe('true');
    const navBounds = await sidebar.boundingBox();
    const headerBounds = await page.locator('header').first().boundingBox();
    expect(navBounds).not.toBeNull();
    expect(headerBounds).not.toBeNull();
    expect(navBounds!.x).toBe(0);
    expect(headerBounds!.x).toBe(0);
    await sidebar.getByRole('button', { name: 'Tools', exact: true }).hover();
    await sidebar.getByRole('button', { name: 'Plugins', exact: true }).click();
    const panel = page.getByRole('dialog', { name: 'Plugins', exact: true });
    await expect(panel).toBeVisible();
    const panelBounds = await panel.boundingBox();
    expect(panelBounds!.x).toBeGreaterThanOrEqual(navBounds!.width);
    await page.keyboard.press('Escape');
    await expect(panel).toHaveCount(0);
    await page.getByText('Theme & Appearance', { exact: true }).click();
    await expect(page).toHaveURL(/\/settings\/appearance$/);
    await expect(page.getByRole('heading', { name: 'Sidebar Position', exact: true })).toHaveCount(
      0,
    );
    await expect(sidebar).toHaveAttribute('data-position', 'left');
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
  await page.locator('input[type="password"]').fill('any-password');
  await submit.click();
  await page.waitForURL('/');
  const sidebar = page.locator('#desktop-navigation');
  await sidebar.getByRole('button', { name: 'Settings', exact: true }).click();
  await expect(root).toHaveAttribute('data-theme', 'dark');
  await expect(root).toHaveAttribute('data-reduce-motion', 'true');
  await expect(page.getByRole('button', { name: 'Hide pages and objects' })).toBeInViewport();
  await expect(sidebar.getByRole('button', { name: 'Settings', exact: true })).toBeInViewport();
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBe(800);
  const navBounds = await sidebar.boundingBox();
  const contentBounds = await page.locator('main').boundingBox();
  expect(contentBounds!.x).toBeGreaterThanOrEqual(navBounds!.x + navBounds!.width);
  expect(contentBounds!.width).toBeGreaterThan(400);
  await page.getByRole('button', { name: 'Hide pages and objects' }).click();
  await expect(page.locator('[data-page-pane-body]')).not.toHaveAttribute('data-pane-animating');
  await expect(sidebar).toHaveCSS('width', `${DESKTOP_SIDEBAR_COLLAPSED_WIDTH}px`);
  await expect(
    sidebar.getByRole('button', { name: 'Settings', exact: true }).locator('span'),
  ).toBeHidden();
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBe(800);
  await page.screenshot({ path: 'test-results/desktop-minimum-dark.png' });
});
