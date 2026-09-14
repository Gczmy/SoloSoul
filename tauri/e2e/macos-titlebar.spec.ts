import { expect, test, type Page } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { login } from './fixtures/auth';

async function mockMacOS(page: Page, position = 'left') {
  await page.addInitScript({
    content:
      readFileSync('e2e/fixtures/tauriMock.js', 'utf8') +
      `
    window.__MOCK_PLATFORM__ = 'macos';
    window.__NATIVE_TITLEBAR_HEIGHT__ = 32;
    window.__E2E_MOCKS__ = {
      vault_check_directory: () => true,
      ocr_get_model_status: () => ({ installed: true, bundled: true }),
      sync_list_conflicts: () => [],
      set_titlebar_color: () => ({ material: 'liquid-glass', platform: 'macos',
        reduceMotion: false, highContrast: false, titlebarHeight: window.__NATIVE_TITLEBAR_HEIGHT__ }),
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
}

for (const position of ['left', 'right', 'top', 'bottom']) {
  test(`macOS ${position} 导航背景铺至顶部，控件避开交通灯`, async ({ page }) => {
    await mockMacOS(page, position);
    await login(page);
    await expect(page.locator('html')).toHaveCSS('--native-titlebar-height', '32px');
    const header = page.locator('header[data-tauri-drag-region="deep"]');
    const headerBox = (await header.boundingBox())!;
    expect(headerBox.y).toBe(position === 'top' ? 80 : 0);
    const title = (await header.locator('h1').boundingBox())!;
    expect(title.y).toBeGreaterThanOrEqual(32);
    const mainBox = (await page.locator('main').boundingBox())!;
    expect(mainBox.y).toBe(position === 'top' ? 80 : 32);
    if (position === 'left' || position === 'right') {
      // 同一背景面横跨原生标题栏和正文，侧栏顶部仍透明。
      const surfaces = await header.evaluate((element) => ({
        top: getComputedStyle(
          document.elementFromPoint(element.getBoundingClientRect().x + 120, 8)!,
        ).backgroundColor,
        main: getComputedStyle(document.querySelector('main')!.parentElement!).backgroundColor,
        sidebar: getComputedStyle(document.querySelector('#desktop-navigation')!).backgroundColor,
      }));
      expect(surfaces.top).toBe(surfaces.main);
      expect(surfaces.top).not.toBe('rgba(0, 0, 0, 0)');
      expect(surfaces.sidebar).toBe('rgba(0, 0, 0, 0)');
      const sidebar = page.locator('#desktop-navigation');
      await expect(sidebar).toHaveCSS('padding-top', '44px');
      await sidebar.getByRole('button', { name: 'Collapse sidebar' }).click();
      await expect(sidebar).toHaveCSS('width', '48px');
      await expect(sidebar).toHaveCSS('padding-top', '44px');
      expect((await header.locator('h1').boundingBox())!.y).toBeGreaterThanOrEqual(32);
      await sidebar.getByRole('button', { name: 'Tools', exact: true }).hover();
      await expect(sidebar.getByRole('button', { name: 'Tools', exact: true })).toHaveAttribute(
        'aria-expanded',
        'true',
      );
      await sidebar.getByRole('button', { name: 'Plugins', exact: true }).click();
      const panel = page.getByRole('dialog', { name: 'Plugins', exact: true });
      await expect(panel).toBeVisible();
      const bounds = (await panel.boundingBox())!;
      expect(bounds.y).toBeGreaterThanOrEqual(40);
      expect(bounds.y + bounds.height).toBeLessThanOrEqual(720);
      await page.keyboard.press('Escape');
    }
    await page.screenshot({ path: `test-results/macos-titlebar-${position}.png` });
  });
}

test('macOS 全屏测量变化后清除并恢复顶部避让', async ({ page }) => {
  await mockMacOS(page);
  await login(page);
  for (const height of [0, 32]) {
    await page.evaluate((value) => {
      Object.assign(window, { __NATIVE_TITLEBAR_HEIGHT__: value });
      window.dispatchEvent(new Event('resize'));
    }, height);
    await expect(page.locator('html')).toHaveCSS('--native-titlebar-height', `${height}px`);
    await expect.poll(async () => (await page.locator('main').boundingBox())!.y).toBe(height);
    expect((await page.locator('header').boundingBox())!.y).toBe(0);
  }
});

test('macOS 小窗口登录卡片保留上下留白及完整圆角', async ({ page }) => {
  await mockMacOS(page);
  await page.setViewportSize({ width: 800, height: 600 });
  await page.goto('/login');
  await expect(page.locator('#startup-screen')).toHaveCount(0);
  const card = page.locator('[class*="loginCard"]');
  await expect(card).toBeVisible();
  const bounds = (await card.boundingBox())!;
  expect(bounds.y).toBeGreaterThanOrEqual(56);
  expect(bounds.y + bounds.height).toBeLessThanOrEqual(576);
  await expect(card).toHaveCSS('border-radius', '20px');
  await expect(page.locator('button[type="submit"]')).toBeInViewport();
});

for (const position of ['left', 'right']) {
  test(`${position} 侧栏快捷卡片覆盖正文和顶部栏，输入区不被遮挡`, async ({ page }) => {
    await mockMacOS(page, position);
    await page.setViewportSize({ width: 800, height: 600 });
    await login(page);
    const navigation = page.locator('#desktop-navigation');
    for (const [name, selector] of [
      ['Search', '[class*="SearchPopover"][class*="card"]'],
      ['Plugins', '[role="dialog"][aria-label="Plugins"]'],
      ['OCR', '[data-ocr-quick-scan="open"]'],
      ['AI Chat', '[data-ai-quick-chat="open"]'],
    ]) {
      await navigation.getByRole('button', { name, exact: true }).click();
      const card =
        name === 'Search'
          ? page.getByPlaceholder('Search objects, profiles...').locator('..').locator('..')
          : page.locator(selector);
      await expect(card).toBeVisible();
      // AI 内容懒加载；待关闭按钮就绪后再验证交互，避免只检查加载占位。
      if (name === 'AI Chat')
        await expect(card.getByRole('button', { name: 'Close', exact: true })).toBeVisible();
      // 几何可见不代表未遮挡：在顶部、正文和下沿采样真实命中元素。
      await expect
        .poll(
          () =>
            card.evaluate((element) => {
              const r = element.getBoundingClientRect();
              return [6, 20, r.height / 2, r.height - 6].every((y) =>
                element.contains(document.elementFromPoint(r.x + r.width / 2, r.y + y)),
              );
            }),
          { message: `${name} 顶部、正文及下沿均应属于卡片自身` },
        )
        .toBe(true);
      if (name === 'Search') {
        const input = page.getByPlaceholder('Search objects, profiles...');
        await input.click();
        await expect(input).toBeFocused();
      }
      await page.keyboard.press('Escape');
      await expect(card).toHaveCount(0);
    }
  });
}
