import { expect, test, type Locator, type Page } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { login } from './fixtures/auth';

async function mockMacOS(page: Page, position = 'left', trafficLightsRight = 79) {
  await page.addInitScript({
    content:
      readFileSync('e2e/fixtures/tauriMock.js', 'utf8') +
      `
    window.__MOCK_PLATFORM__ = 'macos';
    window.__NATIVE_TITLEBAR_HEIGHT__ = 52;
    window.__E2E_MOCKS__ = {
      vault_check_directory: () => true,
      ocr_get_model_status: () => ({ installed: true, bundled: true }),
      sync_list_conflicts: () => [],
      set_titlebar_color: () => ({ material: 'liquid-glass', platform: 'macos',
        reduceMotion: false, highContrast: false, titlebarHeight: window.__NATIVE_TITLEBAR_HEIGHT__,
        trafficLightsRight: window.__NATIVE_TITLEBAR_HEIGHT__ ? ${trafficLightsRight} : 0 }),
      set_titlebar_controls: ({ regions }) => { window.__TITLEBAR_CONTROLS__ = regions; },
      get_window_layout: () => ({ platform: 'macos', titlebarHeight: window.__NATIVE_TITLEBAR_HEIGHT__,
        trafficLightsRight: window.__NATIVE_TITLEBAR_HEIGHT__ ? ${trafficLightsRight} : 0 }),
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

async function expectNativeControl(control: Locator) {
  await expect(control).toBeVisible();
  const bounds = (await control.boundingBox())!;
  expect(bounds.y).toBeGreaterThanOrEqual(0);
  expect(bounds.y + bounds.height).toBeLessThanOrEqual(52);
  await expect
    .poll(() =>
      control.evaluate((element) => {
        const rect = element.getBoundingClientRect();
        const regions =
          (
            window as unknown as {
              __TITLEBAR_CONTROLS__?: { x: number; y: number; width: number; height: number }[];
            }
          ).__TITLEBAR_CONTROLS__ || [];
        return regions.some(
          (r) =>
            rect.x >= r.x &&
            rect.right <= r.x + r.width + 1 &&
            rect.y >= r.y &&
            rect.bottom <= r.y + r.height + 1,
        );
      }),
    )
    .toBe(true);
}

for (const position of ['left', 'right', 'top', 'bottom']) {
  test(`macOS ${position} 导航背景铺至顶部，控件避开交通灯`, async ({ page }) => {
    await mockMacOS(page, position);
    await login(page);
    await expect(page.locator('html')).toHaveCSS('--native-titlebar-height', '52px');
    const header = page.locator('header[data-appbar]');
    const headerBox = (await header.boundingBox())!;
    expect(headerBox.y).toBe(0);
    expect(headerBox.height).toBe(52);
    await expect(header).toHaveAttribute('data-tauri-drag-region', 'false');
    const title = (await header.locator('h1').boundingBox())!;
    expect(title.y).toBeGreaterThanOrEqual(0);
    expect(title.y + title.height).toBeLessThanOrEqual(52);
    expect(title.x).toBeGreaterThanOrEqual(91);
    await expect(header.locator('h1')).toHaveCSS('font-size', '18px');
    const mainBox = (await page.locator('main').boundingBox())!;
    expect(mainBox.y).toBe(position === 'top' ? 48 : 0);
    await expect(page.locator('main')).toHaveCSS('padding-top', '68px');
    const guide = header.getByRole('button', { name: 'Guide', exact: true });
    await expect(guide).toHaveCSS('font-size', '14px');
    await expectNativeControl(guide);
    if (position === 'top' || position === 'bottom') {
      const navigationBar = page.locator('header:not([data-appbar])');
      await expect(navigationBar).toHaveAttribute('data-tauri-drag-region', 'false');
      if (position === 'top') expect((await navigationBar.boundingBox())!.y).toBe(52);
    }
    if (position === 'left' || position === 'right') {
      // 同一背景面横跨原生标题栏和正文，侧栏顶部仍透明。
      const surfaces = await header.evaluate((element) => ({
        top: getComputedStyle(
          document.elementFromPoint(
            element.getBoundingClientRect().x + element.getBoundingClientRect().width / 2,
            2,
          )!,
        ).backgroundColor,
        main: getComputedStyle(document.querySelector('main')!.parentElement!).backgroundColor,
        sidebar: getComputedStyle(document.querySelector('#desktop-navigation')!).backgroundColor,
      }));
      expect(surfaces.top).toBe(surfaces.main);
      expect(surfaces.top).not.toBe('rgba(0, 0, 0, 0)');
      expect(surfaces.sidebar).toBe('rgba(0, 0, 0, 0)');
      const sidebar = page.locator('#desktop-navigation');
      await expect(sidebar).toHaveCSS('padding-top', '64px');
      await sidebar.getByRole('button', { name: 'Collapse sidebar' }).click();
      await expect(sidebar).toHaveCSS('width', '96px');
      await expect(sidebar).toHaveCSS('padding-top', '64px');
      // 交通灯右沿为 79px，折叠后背景边界仍留在整组按钮之外。
      const collapsed = (await sidebar.boundingBox())!;
      if (position === 'left') {
        expect(collapsed.x + collapsed.width).toBeGreaterThanOrEqual(79 + 16);
        expect((await header.boundingBox())!.x).toBe(collapsed.width);
        expect((await page.locator('main').boundingBox())!.x).toBe(collapsed.width);
      }
      expect((await header.locator('h1').boundingBox())!.x).toBeGreaterThanOrEqual(91);
      await sidebar.getByRole('button', { name: 'Tools', exact: true }).hover();
      await expect(sidebar.getByRole('button', { name: 'Tools', exact: true })).toHaveAttribute(
        'aria-expanded',
        'true',
      );
      await sidebar.getByRole('button', { name: 'Plugins', exact: true }).click();
      const panel = page.getByRole('dialog', { name: 'Plugins', exact: true });
      await expect(panel).toBeVisible();
      const bounds = (await panel.boundingBox())!;
      expect(bounds.y).toBeGreaterThanOrEqual(60);
      if (position === 'left') expect(bounds.x).toBeGreaterThanOrEqual(collapsed.width);
      else expect(bounds.x + bounds.width).toBeLessThanOrEqual(collapsed.x);
      expect(bounds.y + bounds.height).toBeLessThanOrEqual(720);
      await page.keyboard.press('Escape');
    }
    await page.screenshot({ path: `test-results/macos-titlebar-${position}.png` });
  });
}

test('macOS 切页、折叠和缩放后更新顶部按钮命中区', async ({ page }) => {
  await mockMacOS(page);
  await login(page);
  const sidebar = page.locator('#desktop-navigation');
  await sidebar.getByRole('button', { name: 'Identity', exact: true }).click();
  const header = page.locator('header[data-appbar]');
  const back = header.getByRole('button', { name: 'Back', exact: true });
  const create = header.getByRole('button', { name: '+ New', exact: true });
  await expectNativeControl(back);
  await expectNativeControl(create);
  await sidebar.getByRole('button', { name: 'Collapse sidebar' }).click();
  await expect(sidebar).toHaveCSS('width', '96px');
  await expectNativeControl(back);
  expect((await back.boundingBox())!.x).toBeGreaterThanOrEqual(91);
  await page.setViewportSize({ width: 800, height: 600 });
  await expectNativeControl(back);
  await expectNativeControl(create);
  await create.click();
  await expect(page).toHaveURL(/\/editor(?:\?|$)/);
  await expectNativeControl(back);
  await back.click();
  await expect(page).toHaveURL(/\/workspace/);
  await expectNativeControl(create);
  await back.click();
  await expect(page).toHaveURL('http://localhost:1420/');
  await expect(back).toHaveCount(0);
  await expectNativeControl(header.getByRole('button', { name: 'Guide', exact: true }));
});

test('macOS 全屏往返保持单行 AppBar，无重复顶部留白', async ({ page }) => {
  await mockMacOS(page);
  await login(page);
  const sidebar = page.locator('#desktop-navigation');
  await sidebar.getByRole('button', { name: 'Collapse sidebar' }).click();
  for (const height of [0, 52]) {
    await page.evaluate((value) => {
      Object.assign(window, { __NATIVE_TITLEBAR_HEIGHT__: value });
      window.dispatchEvent(new Event('resize'));
    }, height);
    await expect(page.locator('html')).toHaveCSS('--native-titlebar-height', `${height}px`);
    await expect.poll(async () => (await page.locator('main').boundingBox())!.y).toBe(0);
    expect((await page.locator('header').boundingBox())!.height).toBe(52);
    expect((await page.locator('header').boundingBox())!.y).toBe(0);
    await expect(sidebar).toHaveCSS('width', '96px');
  }
});

test('macOS 交通灯范围变宽时折叠侧栏和正文同步避让', async ({ page }) => {
  await mockMacOS(page, 'left', 106);
  await login(page);
  const sidebar = page.locator('#desktop-navigation');
  await sidebar.getByRole('button', { name: 'Collapse sidebar' }).click();
  await expect(sidebar).toHaveCSS('width', '122px');
  expect((await page.locator('header[data-appbar]').boundingBox())!.x).toBe(122);
  expect((await page.locator('main').boundingBox())!.x).toBe(122);
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
