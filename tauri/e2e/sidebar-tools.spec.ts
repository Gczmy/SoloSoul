import { expect, test, type Locator, type Page } from '@playwright/test';
import { login, setupTauriMock } from './fixtures/auth';

async function setupSidebar(
  page: Page,
  position: 'left' | 'right' | 'top' | 'bottom',
  platform = 'macos',
) {
  await setupTauriMock(page);
  await page.addInitScript(
    ({ position, platform }) => {
      const native = {
        platform,
        titlebarHeight: platform === 'macos' ? 52 : 0,
        trafficLightsRight: 79,
      };
      Object.assign(window, {
        __MOCK_PLATFORM__: platform,
        __E2E_MOCKS__: {
          vault_check_directory: () => true,
          ocr_get_model_status: () => ({ installed: true, bundled: true }),
          ocr_list_available_tiers: () => [],
          ocr_get_active_tier: () => 'small',
          sync_list_conflicts: () => [],
          get_window_layout: () => native,
          set_titlebar_color: () => ({
            ...native,
            material: 'liquid-glass',
            highContrast: false,
            reduceMotion: false,
          }),
        },
      });
      const bridge = (window as any).__TAURI_INTERNALS__;
      const invoke = bridge.invoke;
      bridge.invoke = async (cmd: string, args: unknown) => {
        const result = await invoke(cmd, args);
        if (cmd === 'user_data_get_preferences') result.sidebarPosition = position;
        return result;
      };
      localStorage.setItem('i18nextLng', 'en-US');
    },
    { position, platform },
  );
  await login(page);
}

async function buttonAppearance(button: Locator) {
  return button.evaluate((element) => {
    const style = getComputedStyle(element);
    const label = getComputedStyle(element.querySelector('span')!);
    const icon = element.querySelector('svg')!.getBoundingClientRect();
    return {
      color: style.color,
      background: style.backgroundColor,
      height: style.height,
      radius: style.borderRadius,
      fontSize: label.fontSize,
      fontWeight: label.fontWeight,
      iconSize: [icon.width, icon.height],
    };
  });
}

for (const position of ['left', 'right'] as const) {
  test(`${position} 展开侧栏悬停显示全部工具，入口及主导航不移动`, async ({ page }) => {
    await setupSidebar(page, position);
    const nav = page.locator('#desktop-navigation');
    const tools = nav.getByRole('button', { name: 'Tools', exact: true });
    const list = nav.locator('[data-sidebar-tools]');
    await expect(tools).toHaveAttribute('aria-expanded', 'false');
    const toggleBounds = await tools.boundingBox();
    const addPage = nav.getByRole('button', { name: 'Add Page', exact: true });
    const identity = nav.getByRole('button', { name: 'Identity', exact: true });
    expect(await buttonAppearance(addPage)).toEqual(await buttonAppearance(identity));
    await addPage.hover();
    await expect(page.locator('[role="tooltip"]')).toHaveCount(0);
    const addPageBounds = await addPage.boundingBox();
    const identityBounds = await nav
      .getByRole('button', { name: 'Identity', exact: true })
      .boundingBox();
    await tools.hover();
    await expect(tools).toHaveAttribute('aria-expanded', 'true');
    await expect(list).toBeVisible();
    await expect(list).toHaveCSS('background-color', 'rgba(0, 0, 0, 0)');
    await expect(list).toHaveCSS('background-image', 'none');
    await expect(list).toHaveCSS('backdrop-filter', 'none');
    expect(await tools.boundingBox()).toEqual(toggleBounds);
    expect(await addPage.boundingBox()).toEqual(addPageBounds);
    expect(await nav.getByRole('button', { name: 'Identity', exact: true }).boundingBox()).toEqual(
      identityBounds,
    );
    const buttons = list.getByRole('button');
    await expect(buttons).toHaveCount(10);
    for (const button of await buttons.all()) {
      await expect(button).toBeInViewport();
      expect(
        await button.evaluate((element) => {
          const r = element.getBoundingClientRect();
          return element.contains(document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2));
        }),
      ).toBe(true);
    }
    // 原生玻璃下不能透出重叠的导航文字；裁剪只影响绘制，不改变布局。
    expect(
      await identity.evaluate((element) => {
        const zone = element.closest('[class*="primaryZone"]')!;
        const clip = getComputedStyle(zone).clipPath;
        return clip !== 'none' && !clip.endsWith('0px 0px)');
      }),
    ).toBe(true);
    await list.getByRole('button', { name: 'Search', exact: true }).hover();
    await expect(list).toBeVisible();
    await page.screenshot({ path: test.info().outputPath(`tools-${position}.png`) });
    await page.mouse.move(640, 400);
    await expect(tools).toHaveAttribute('aria-expanded', 'false');
    await expect(list).toBeHidden();
    await identity.click();
    await expect(page).toHaveURL(/section=identity/);
  });
}

for (const position of ['left', 'right'] as const) {
  test(`${position} 折叠侧栏使用向上玻璃菜单，缩放和操作卡片不挤动导航`, async ({ page }) => {
    await setupSidebar(page, position);
    const nav = page.locator('#desktop-navigation');
    await nav.getByRole('button', { name: 'Collapse sidebar', exact: true }).click();
    const tools = nav.getByRole('button', { name: 'Tools', exact: true });
    const addPage = nav.getByRole('button', { name: 'Add Page', exact: true });
    const menu = nav.locator('[data-sidebar-tools]');
    const toolBounds = (await tools.boundingBox())!;
    const addPageBounds = await addPage.boundingBox();
    await tools.hover();
    await expect(menu).toBeVisible();
    await expect(menu).toHaveCSS('background-color', 'rgba(0, 0, 0, 0)');
    await expect(menu).toHaveCSS('backdrop-filter', 'none');
    expect(await tools.boundingBox()).toEqual(toolBounds);
    expect(await addPage.boundingBox()).toEqual(addPageBounds);
    const menuBounds = (await menu.boundingBox())!;
    const navBounds = (await nav.boundingBox())!;
    expect(menuBounds.height).toBeGreaterThan(400);
    expect(menuBounds.y + menuBounds.height).toBeLessThanOrEqual(toolBounds.y);
    expect(menuBounds.x).toBeGreaterThan(navBounds.x);
    expect(menuBounds.x + menuBounds.width).toBeLessThan(navBounds.x + navBounds.width);
    await page.screenshot({ path: test.info().outputPath(`compact-tools-${position}.png`) });

    // 保持菜单打开并缩小窗口，确认仍能滚动到最后一项并打开卡片。
    await page.setViewportSize({ width: 800, height: 600 });
    await tools.hover();
    await expect(menu).toBeVisible();
    expect((await menu.boundingBox())!.y).toBeGreaterThanOrEqual(100);
    await menu.getByRole('button', { name: 'AI Chat', exact: true }).click();
    const chat = page.locator('[data-ai-quick-chat="open"]');
    await expect(chat).toBeInViewport();
    const chatBounds = (await chat.boundingBox())!;
    if (position === 'left') expect(chatBounds.x).toBeGreaterThanOrEqual(96);
    else expect(chatBounds.x + chatBounds.width).toBeLessThanOrEqual(800 - 96);
    await chat.hover();
    await expect(tools).toHaveAttribute('aria-expanded', 'true');
    await chat.getByRole('button', { name: 'Close', exact: true }).click();
    await expect(tools).toHaveAttribute('aria-expanded', 'false');
    await expect(menu).toBeHidden();
    await nav.getByRole('button', { name: 'Identity', exact: true }).click();
    await expect(page).toHaveURL(/section=identity/);
  });
}

test('添加页面在折叠侧栏复用导航的提示卡片，展开后不重复显示提示', async ({ page }) => {
  await setupSidebar(page, 'left');
  const nav = page.locator('#desktop-navigation');
  const addPage = nav.getByRole('button', { name: 'Add Page', exact: true });
  const identity = nav.getByRole('button', { name: 'Identity', exact: true });
  await nav.getByRole('button', { name: 'Collapse sidebar', exact: true }).click();
  expect(await buttonAppearance(addPage)).toEqual(await buttonAppearance(identity));
  await identity.hover();
  const tooltip = page.locator('[role="tooltip"]');
  await expect(tooltip).toHaveText('Identity');
  const tooltipClass = await tooltip.getAttribute('class');
  await addPage.hover();
  await expect(tooltip).toHaveText('Add Page');
  await expect(tooltip).toHaveAttribute('class', tooltipClass!);
  await nav.getByRole('button', { name: 'Expand sidebar', exact: true }).click();
  await addPage.hover();
  await expect(tooltip).toHaveCount(0);
});

for (const position of ['left', 'right'] as const) {
  for (const collapsed of [false, true]) {
    test(`${position} ${collapsed ? '折叠' : '展开'}侧栏添加页面向上利用空间，缩放后表单与图标仍可操作`, async ({
      page,
    }) => {
      await page.setViewportSize({ width: 800, height: 600 });
      await setupSidebar(page, position);
      const nav = page.locator('#desktop-navigation');
      if (collapsed)
        await nav.getByRole('button', { name: 'Collapse sidebar', exact: true }).click();
      const trigger = nav.getByRole('button', { name: 'Add Page', exact: true });
      const triggerBounds = (await trigger.boundingBox())!;
      await trigger.click();
      const card = page.locator('[data-add-page-popover]');
      const icons = card.locator('[data-icon-picker-scroll]');
      await expect(card).toBeVisible();
      await card.evaluate(async (element) => {
        await Promise.all(element.getAnimations().map((animation) => animation.finished));
      });
      const bounds = (await card.boundingBox())!;
      expect(bounds.y).toBeLessThan(triggerBounds.y - 100);
      expect(bounds.height).toBeGreaterThanOrEqual(450);
      expect(bounds.y).toBeGreaterThanOrEqual(64);
      expect(bounds.y + bounds.height).toBeLessThanOrEqual(584);
      expect((await icons.boundingBox())!.height).toBeGreaterThan(280);
      const confirm = card.getByRole('button', { name: 'Confirm', exact: true });
      await confirm.click();
      await expect(card.getByText('Page name is required', { exact: true })).toBeVisible();
      await expect(confirm).toBeInViewport();
      await page.screenshot({
        path: test.info().outputPath(`add-page-${position}-${collapsed}.png`),
      });

      await page.setViewportSize({ width: 800, height: 480 });
      await expect(card).toHaveCSS('height', '400px');
      const smallBounds = (await card.boundingBox())!;
      expect(smallBounds.y).toBeGreaterThanOrEqual(64);
      expect(smallBounds.y + smallBounds.height).toBeLessThanOrEqual(464);
      expect((await icons.boundingBox())!.height).toBeGreaterThan(180);
      const name = card.getByRole('textbox', { name: 'Page name', exact: true });
      await name.fill('New collection');
      await card
        .getByRole('textbox', { name: 'Page description (optional)', exact: true })
        .fill('Draft');
      const footerBounds = await confirm.boundingBox();
      await icons.getByRole('button').last().click();
      await expect(name).toHaveValue('New collection');
      expect(await confirm.boundingBox()).toEqual(footerBounds);
      await card.getByRole('button', { name: 'Cancel', exact: true }).click();
      await expect(card).toHaveCount(0);
      if (position === 'left' && collapsed) {
        await trigger.click();
        await card
          .getByRole('textbox', { name: 'Page name', exact: true })
          .fill('Small window page');
        await confirm.click();
        await expect(page).toHaveURL(/\/workspace\/custom\//);
        await expect(card).toHaveCount(0);
      }
    });
  }
}

for (const position of ['top', 'bottom'] as const) {
  test(`${position} 横向导航添加页面表单和底部操作保持可见`, async ({ page }) => {
    await page.setViewportSize({ width: 800, height: 600 });
    await setupSidebar(page, position);
    await page.getByRole('button', { name: 'Add Page', exact: true }).click();
    const card = page.locator('[data-add-page-popover]');
    await expect(card.getByRole('textbox', { name: 'Page name', exact: true })).toBeInViewport();
    await expect(card.getByRole('button', { name: 'Confirm', exact: true })).toBeInViewport();
    await card.getByRole('button', { name: 'Cancel', exact: true }).click();
    await expect(card).toHaveCount(0);
  });
}

for (const position of ['left', 'right'] as const) {
  for (const collapsed of [false, true]) {
    test(`${position} ${collapsed ? '折叠' : '展开'}侧栏打开搜索后能再次点击搜索及直接切换卡片`, async ({
      page,
    }) => {
      await setupSidebar(page, position);
      const nav = page.locator('#desktop-navigation');
      if (collapsed)
        await nav.getByRole('button', { name: 'Collapse sidebar', exact: true }).click();
      const tools = nav.getByRole('button', { name: 'Tools', exact: true });
      const search = nav.getByRole('button', { name: 'Search', exact: true });
      const input = page.getByPlaceholder('Search objects, profiles...');
      await tools.hover();
      await search.click();
      await expect(input).toBeFocused();

      // 按实际坐标点击，捕获遮罩吞掉侧栏点击的回归，而非等待遮罩消失。
      const bounds = (await search.boundingBox())!;
      await page.mouse.click(bounds.x + bounds.width / 2, bounds.y + bounds.height / 2);
      await expect(input).toHaveCount(0);
      await expect(tools).toHaveAttribute('aria-expanded', 'true');

      await search.click();
      await expect(input).toBeFocused();
      await nav.getByRole('button', { name: 'Plugins', exact: true }).click({ timeout: 3000 });
      await expect(input).toHaveCount(0);
      await expect(page.getByRole('dialog', { name: 'Plugins', exact: true })).toBeVisible();
      await expect(tools).toHaveAttribute('aria-expanded', 'true');
      await page.keyboard.press('Escape');
      await page.mouse.move(640, 650);
      await expect(tools).toHaveAttribute('aria-expanded', 'false');
    });
  }
}

test('工具菜单随原生玻璃与辅助功能切换材质', async ({ page }) => {
  await setupSidebar(page, 'left');
  const root = page.locator('html');
  const tools = page.getByRole('button', { name: 'Tools', exact: true });
  const menu = page.locator('[data-sidebar-tools]');
  await tools.hover();
  for (const material of ['liquid-glass', 'vibrancy']) {
    await root.evaluate(
      (element, value) => element.setAttribute('data-native-material', value),
      material,
    );
    await expect(menu).toHaveCSS('background-color', 'rgba(0, 0, 0, 0)');
    await expect(menu).toHaveCSS('backdrop-filter', 'none');
  }
  await root.evaluate((element) => element.setAttribute('data-native-material', 'solid'));
  await expect(menu).not.toHaveCSS('background-color', 'rgba(0, 0, 0, 0)');
  await root.evaluate((element) => {
    element.setAttribute('data-native-material', 'liquid-glass');
    element.setAttribute('data-high-contrast', 'true');
  });
  await expect(menu).not.toHaveCSS('background-color', 'rgba(0, 0, 0, 0)');
  await root.evaluate((element) => element.setAttribute('data-high-contrast', 'false'));
  await expect(menu).toHaveCSS('background-color', 'rgba(0, 0, 0, 0)');
  // 浏览器的辅助功能偏好也必须覆盖原生玻璃分支，不能被选择器优先级吞掉。
  const cdp = await page.context().newCDPSession(page);
  await cdp.send('Emulation.setEmulatedMedia', {
    features: [{ name: 'prefers-reduced-transparency', value: 'reduce' }],
  });
  await expect(menu).not.toHaveCSS('background-color', 'rgba(0, 0, 0, 0)');
  await cdp.send('Emulation.setEmulatedMedia', { features: [] });
  await cdp.detach();
  await page.emulateMedia({ forcedColors: 'active' });
  await expect(menu).not.toHaveCSS('background-color', 'rgba(0, 0, 0, 0)');
});

test('小窗口工具列表利用剩余高度，卡片打开期间保持菜单且不遮挡固定操作', async ({ page }) => {
  await page.setViewportSize({ width: 800, height: 600 });
  await setupSidebar(page, 'left');
  const nav = page.locator('#desktop-navigation');
  const tools = nav.getByRole('button', { name: 'Tools', exact: true });
  const list = nav.locator('[data-sidebar-tools]');
  await tools.hover();
  const bounds = (await list.boundingBox())!;
  expect(bounds.height).toBeGreaterThan(300);
  expect(bounds.y).toBeGreaterThanOrEqual(100);
  expect(bounds.y + bounds.height).toBeLessThanOrEqual((await tools.boundingBox())!.y);
  await list.getByRole('button', { name: 'AI Chat', exact: true }).scrollIntoViewIfNeeded();
  await list.getByRole('button', { name: 'AI Chat', exact: true }).click();
  const chat = page.locator('[data-ai-quick-chat="open"]');
  await expect(chat).toBeVisible();
  await expect(chat.getByRole('button', { name: 'Close', exact: true })).toBeVisible();
  await chat.hover();
  await expect(tools).toHaveAttribute('aria-expanded', 'true');
  expect((await chat.boundingBox())!.x).toBeGreaterThanOrEqual(232);
  await chat.getByRole('button', { name: 'Close', exact: true }).click();
  await expect(chat).toHaveCount(0);
  await expect(tools).toHaveAttribute('aria-expanded', 'false');
  await tools.hover();
  await expect(list.getByRole('button', { name: 'AI Chat', exact: true })).toBeInViewport();
  await nav.getByRole('button', { name: 'Settings', exact: true }).click();
  await expect(page).toHaveURL(/\/settings$/);
  await expect(tools).toHaveAttribute('aria-expanded', 'false');
});

test('工具入口支持点击、键盘和 Escape，关闭后菜单退出 Tab 顺序', async ({ page, browserName }) => {
  // macOS WebKit 默认使用 Option+Tab 访问按钮，与系统键盘导航偏好保持一致。
  const tabKey = browserName === 'webkit' ? 'Alt+Tab' : 'Tab';
  await setupSidebar(page, 'left', 'windows');
  const nav = page.locator('#desktop-navigation');
  const tools = nav.getByRole('button', { name: 'Tools', exact: true });
  const list = nav.locator('[data-sidebar-tools]');
  await tools.click();
  await page.mouse.move(640, 400);
  await expect(tools).toHaveAttribute('aria-expanded', 'true');
  await page.mouse.click(640, 400);
  await expect(tools).toHaveAttribute('aria-expanded', 'false');
  await tools.focus();
  await page.keyboard.press(tabKey);
  await expect(nav.getByRole('button', { name: /Lock Vault/i })).toBeFocused();
  await tools.focus();
  await page.keyboard.press('Enter');
  await page.keyboard.press(tabKey);
  await expect(list.getByRole('button', { name: 'Search', exact: true })).toBeFocused();
  await page.keyboard.press('Escape');
  await expect(tools).toBeFocused();
  await expect(list).toBeHidden();
});

// 暂停真实 CSS transition 采样中间帧，验证展开/收起方向，而非只检查最终 class。
async function sampleMenuSweep(menu: Locator) {
  return menu.evaluate(async (element) => {
    const zone = element.closest('nav')!.querySelector('[class*="primaryZone"]')!;
    const animation = element
      .getAnimations()
      .find((a) => a instanceof CSSTransition && a.transitionProperty === 'clip-path');
    const zoneAnimation = zone
      .getAnimations()
      .find((a) => a instanceof CSSTransition && a.transitionProperty === 'clip-path');
    if (!animation || !zoneAnimation) throw new Error('Menu and navigation must animate together');
    animation.pause();
    zoneAnimation.pause();
    const duration = Number(animation.effect!.getTiming().duration);
    const samples = [];
    for (const progress of [0.15, 0.5, 0.85]) {
      animation.currentTime = duration * progress;
      zoneAnimation.currentTime = duration * progress;
      await new Promise(requestAnimationFrame);
      const rect = element.getBoundingClientRect();
      const top = parseFloat(getComputedStyle(element).clipPath.slice(6));
      const zoneBottom = parseFloat(getComputedStyle(zone).clipPath.split(' ')[2]);
      samples.push({
        top,
        revealed: rect.height * (1 - top / 100),
        zoneBottom,
        bottom: rect.bottom,
      });
    }
    animation.finish();
    zoneAnimation.finish();
    return samples;
  });
}

for (const platform of ['macos', 'windows']) {
  for (const collapsed of [false, true]) {
    test(`${platform} ${collapsed ? '折叠' : '展开'}侧栏工具沿底边向上展开、向下收起且不缩放按钮`, async ({
      page,
    }) => {
      await setupSidebar(page, 'left', platform);
      const nav = page.locator('#desktop-navigation');
      if (collapsed)
        await nav.getByRole('button', { name: 'Collapse sidebar', exact: true }).click();
      const tools = nav.getByRole('button', { name: 'Tools', exact: true });
      const menu = nav.locator('[data-sidebar-tools]');
      const toggleBounds = await tools.boundingBox();
      await tools.hover();
      const buttonBounds = await menu.locator('button').first().boundingBox();
      const opening = await sampleMenuSweep(menu);
      expect(opening[0].top).toBeGreaterThan(opening[1].top);
      expect(opening[1].top).toBeGreaterThan(opening[2].top);
      expect(await menu.locator('button').first().boundingBox()).toEqual(buttonBounds);
      await expect(menu).toHaveCSS('opacity', '1');
      await page.mouse.move(640, 400);
      await expect(menu).toHaveJSProperty('inert', true);
      const closing = await sampleMenuSweep(menu);
      expect(closing[0].top).toBeLessThan(closing[1].top);
      expect(closing[1].top).toBeLessThan(closing[2].top);
      for (const sample of [...opening, ...closing]) {
        expect(sample.bottom).toBe(opening[0].bottom);
        expect(Math.abs(sample.zoneBottom - sample.revealed)).toBeLessThan(1);
      }
      await expect(menu).toBeHidden();
      expect(await tools.boundingBox()).toEqual(toggleBounds);
      // 快速反向悬停后仍能完成展开，卡片保持最终锚点位置。
      await tools.hover();
      await page.mouse.move(640, 400);
      await tools.hover();
      await menu.getByRole('button', { name: 'Search', exact: true }).click();
      await expect(page.getByPlaceholder('Search objects, profiles...')).toBeFocused();
    });
  }
}

test('系统和应用减少动态效果时，工具菜单立即展开/收起', async ({ page }) => {
  await setupSidebar(page, 'left');
  const tools = page.getByRole('button', { name: 'Tools', exact: true });
  const menu = page.locator('[data-sidebar-tools]');
  for (const source of ['media', 'native', 'app']) {
    await page.emulateMedia({ reducedMotion: source === 'media' ? 'reduce' : 'no-preference' });
    await page.locator('html').evaluate((root, source) => {
      root.setAttribute('data-reduce-motion', String(source === 'native'));
      root.setAttribute('data-user-reduce-motion', String(source === 'app'));
    }, source);
    await tools.hover();
    await expect(menu).toBeVisible();
    expect(
      await menu.evaluate((element) => parseFloat(getComputedStyle(element).transitionDuration)),
    ).toBeLessThan(0.001);
    await expect(menu).toHaveCSS('clip-path', /^inset\(0px -24px(?: 0px)?\)$/);
    await page.mouse.move(640, 400);
    await expect(menu).toBeHidden();
  }
});
