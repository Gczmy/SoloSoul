import { expect, test, type Page } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { getSchemeById, DESKTOP_NAVIGATION_COLORS } from '../src/lib/themeSchemes';
import { login } from './fixtures/auth';

async function mockTheme(
  page: Page,
  schemeId: string,
  material = 'auto',
  systemMode?: 'light' | 'dark',
  platform: 'windows' | 'android' | 'ios' = 'windows',
) {
  // 仅Windows材质场景使用桌面视口，保留Android/iOS项目环境。
  if (platform === 'windows') await page.setViewportSize({ width: 1280, height: 720 });
  const scheme = getSchemeById(schemeId, platform === 'windows' ? 'desktop' : 'shared')!;
  await page.addInitScript({
    content:
      readFileSync('e2e/fixtures/tauriMock.js', 'utf8') +
      `
    window.__MOCK_PLATFORM__ = '${platform}';
    window.__RF201_SYSTEM_IPC_COUNT__ = 0;
    localStorage.setItem('i18nextLng', 'en-US');
    const prefs = ${JSON.stringify({
      theme: systemMode ? 'system' : scheme.mode,
      defaultLightTheme: scheme.mode === 'light' ? schemeId : 'warm-stone',
      defaultDarkTheme: scheme.mode === 'dark' ? schemeId : 'warm-stone-dark',
      startupThemes: {
        [scheme.mode]: {
          background: scheme.variables['--bg-base'],
          foreground: scheme.variables['--text-primary'],
          secondary: scheme.variables['--text-secondary'],
        },
      },
    })};
    localStorage.setItem('solosoul_ui_prefs', JSON.stringify(prefs));
    window.__E2E_MOCKS__ = {
      vault_check_directory: () => true,
      ocr_get_model_status: () => ({ installed: true, bundled: true }),
      sync_list_conflicts: () => [],
      set_titlebar_color: ({ color }) => {
        document.documentElement.dataset.nativeThemeRgb = [color.red, color.green, color.blue].join(',');
        const material = '${material}' === 'auto' ? (color.red < 128 ? 'solid' : 'mica') : '${material}';
        return { material, platform: 'windows', reduceMotion: false, highContrast: false };
      },
    };
    const systemMode = ${JSON.stringify(systemMode ?? null)};
    const originalInvoke = window.__TAURI_INTERNALS__.invoke;
    window.__TAURI_INTERNALS__.invoke = async (cmd, args) => {
      // 通用 fixture 对该命令固定返回 light，因此在外层模拟原生系统结果。
      if (cmd === 'get_system_theme' && '${platform}' !== 'windows') {
        window.__RF201_SYSTEM_IPC_COUNT__ += 1;
        return systemMode ?? 'dark';
      }
      if (cmd === 'get_system_theme' && systemMode !== null) return systemMode;
      if (cmd === 'set_status_bar_style') {
        document.documentElement.dataset.statusBarStyle = args.payload.style;
      }
      const result = await originalInvoke(cmd, args);
      if (cmd === 'ui_get_preferences' || cmd === 'user_data_get_preferences') Object.assign(result, prefs);
      return result;
    };
  `,
  });
}

async function expectThemeBackground(page: Page, schemeId: string) {
  const scheme = getSchemeById(schemeId, 'desktop')!;
  // 原生外壳与导航共用中性色，正文仍使用所选平台色板。
  const native = DESKTOP_NAVIGATION_COLORS[scheme.mode];
  const nativeRgb = [1, 3, 5].map((offset) => parseInt(native.slice(offset, offset + 2), 16));
  const root = page.locator('html');
  await expect(root).toHaveAttribute('data-theme', scheme.mode);
  await expect(page.locator('html')).toHaveAttribute('data-native-theme-rgb', nativeRgb.join(','));
  const transparent =
    scheme.mode === 'light' && (await root.getAttribute('data-native-material')) === 'mica';
  const shellColor = transparent ? 'rgba(0, 0, 0, 0)' : `rgb(${nativeRgb.join(', ')})`;
  for (const selector of ['html', 'body', '#root']) {
    await expect(page.locator(selector)).toHaveCSS('background-color', shellColor);
    await expect(page.locator(selector)).toHaveCSS('background-image', 'none');
  }
  await expect
    .poll(() => root.evaluate((el) => getComputedStyle(el).getPropertyValue('--bg-base').trim()))
    .toBe(scheme.variables['--bg-base']);
}

async function expectWindowsContentSurface(page: Page, schemeId: string) {
  // 页面树与正文共用外层内容面；main 只负责滚动，不能用它判断实际绘制颜色。
  const surface = page.locator('[data-shell-surface][data-page-pane-layout]');
  const color = getSchemeById(schemeId, 'desktop')!.variables['--bg-base'];
  const channels = [1, 3, 5].map((offset) => parseInt(color.slice(offset, offset + 2), 16));
  await expect(surface).toHaveCSS('background-color', `rgb(${channels.join(', ')})`);
  await expect(surface).toHaveCSS('border-top-style', 'solid');
  await expect(surface).toHaveCSS('border-top-width', '1px');
}

// 只验证浏览器 CSS 与模拟 IPC 参数一致，不代表真实 Windows/Android 材质验收。
for (const { systemMode, mediaMode, schemeId } of [
  { systemMode: 'dark', mediaMode: 'light', schemeId: 'deep-ocean' },
  { systemMode: 'light', mediaMode: 'dark', schemeId: 'soft-cream' },
] as const) {
  test(`system 主题在模拟 IPC=${systemMode}、媒体查询=${mediaMode} 时保持一致`, async ({
    page,
  }) => {
    await page.emulateMedia({ colorScheme: mediaMode });
    await mockTheme(page, schemeId, 'auto', systemMode);
    await page.goto('/login');
    await expect(page.locator('#startup-screen')).toHaveCount(0);
    expect(
      await page.evaluate(() => window.matchMedia('(prefers-color-scheme: dark)').matches),
    ).toBe(mediaMode === 'dark');
    await expect(page.locator('html')).toHaveAttribute('data-theme', systemMode);
    await expect
      .poll(() =>
        page
          .locator('html')
          .evaluate((root) => getComputedStyle(root).getPropertyValue('--bg-base').trim()),
      )
      .toBe(getSchemeById(schemeId, 'desktop')!.variables['--bg-base']);
    await expect(page.locator('html')).toHaveAttribute(
      'data-native-material',
      systemMode === 'dark' ? 'solid' : 'mica',
    );
    await expectThemeBackground(page, schemeId);
    await expect(page.locator('html')).toHaveAttribute('data-status-bar-style', systemMode);
    await expect(page.locator('[class*="loginWrapper"]')).toHaveCSS(
      'background-color',
      'rgba(0, 0, 0, 0)',
    );
    const card = page.locator('[data-login-card]');
    await expect(card).toBeVisible();
    await expect(card).not.toHaveCSS('background-color', 'rgba(0, 0, 0, 0)');
  });
}

for (const scheme of ['warm-stone-dark', 'deep-ocean', 'forest-night', 'soft-cream']) {
  test(`Windows ${scheme} 登录外壳按模式选择材质并保持中性`, async ({ page }) => {
    await mockTheme(page, scheme);
    await page.goto('/login');
    await expect(page.locator('#startup-screen')).toHaveCount(0);
    await expect(page.locator('html')).toHaveAttribute(
      'data-native-material',
      getSchemeById(scheme)!.mode === 'dark' ? 'solid' : 'mica',
    );
    await expectThemeBackground(page, scheme);
    await expect(page.locator('[class*="loginWrapper"]')).toHaveCSS(
      'background-color',
      'rgba(0, 0, 0, 0)',
    );
    await expect(page.locator('[class*="loginCard"]')).not.toHaveCSS(
      'background-color',
      'rgba(0, 0, 0, 0)',
    );
    await page.screenshot({ path: `test-results/login-${scheme}.png` });
    await login(page);
    await expectWindowsContentSurface(page, scheme);
    await expect(page.locator('[data-appbar]')).toHaveCSS(
      'background-color',
      await page.locator('html').evaluate((el) => getComputedStyle(el).backgroundColor),
    );
    await expect(page.locator('[data-appbar]')).toHaveCSS('background-image', 'none');
  });
}

test('同一深色模式切换配色后，实色外壳保持中性灰，正文保留所选色板', async ({ page }) => {
  await mockTheme(page, 'warm-stone-dark');
  await login(page);
  await page
    .locator('#desktop-navigation')
    .getByRole('button', { name: 'Settings', exact: true })
    .click();
  await page.getByText('Theme & Appearance', { exact: true }).click();
  await page.getByRole('button', { name: 'More Appearances', exact: true }).click();
  for (const [name, scheme] of [
    ['Deep Ocean', 'deep-ocean'],
    ['Forest Night', 'forest-night'],
  ]) {
    await page.getByRole('button', { name, exact: true }).click();
    await expectThemeBackground(page, scheme);
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
    await expect(page.locator('#desktop-navigation')).toHaveCSS(
      'background-color',
      'rgb(24, 24, 24)',
    );
    await expectWindowsContentSurface(page, scheme);
    await expect(page.locator('[data-appbar]')).toHaveCSS('background-color', 'rgb(24, 24, 24)');
    await expect(page.locator('[data-appbar]')).toHaveCSS('background-image', 'none');
  }
});

test('无 Mica 时采用中性外壳，正文配色不受首帧缓存锁定', async ({ page }) => {
  await mockTheme(page, 'warm-stone-dark', 'solid');
  await login(page);
  await page
    .locator('#desktop-navigation')
    .getByRole('button', { name: 'Settings', exact: true })
    .click();
  await page.getByText('Theme & Appearance', { exact: true }).click();
  await page.getByRole('button', { name: 'More Appearances', exact: true }).click();
  await page.getByRole('button', { name: 'Deep Ocean', exact: true }).click();
  await expectThemeBackground(page, 'deep-ocean');
});

test('Windows 实色回退仍能区分外壳和正文', async ({ page }) => {
  await mockTheme(page, 'warm-stone-dark', 'solid');
  await login(page);
  const header = page.locator('[data-appbar]');
  const surface = await header.evaluate((el) => getComputedStyle(el).backgroundColor);
  await expect(page.locator('#desktop-navigation')).toHaveCSS('background-color', surface);
  expect(surface).not.toBe('rgba(0, 0, 0, 0)');
  await expectWindowsContentSurface(page, 'warm-stone-dark');
  await expect(page.locator('[data-shell-surface]')).not.toHaveCSS('background-color', surface);
  await expectThemeBackground(page, 'warm-stone-dark');
});

test('Windows 强制颜色模式关闭透明背景并保留正文边界', async ({ page }) => {
  await mockTheme(page, 'warm-stone-dark');
  await login(page);
  await page.emulateMedia({ forcedColors: 'active' });
  for (const selector of ['html', 'body', '#root', 'main', '[data-shell-surface]', '[data-appbar]']) {
    await expect(page.locator(selector)).not.toHaveCSS('background-color', 'rgba(0, 0, 0, 0)');
  }
  const surface = page.locator('[data-shell-surface]');
  await expect(surface).toHaveCSS('border-top-style', 'solid');
  await expect(surface).toHaveCSS('border-top-width', '1px');
  await expect(surface).toHaveCSS('box-shadow', 'none');
});

test('深色切换期间即使旧原生材质仍为 Mica，外壳也保持中性底色', async ({ page }) => {
  await mockTheme(page, 'warm-stone-dark', 'mica');
  await login(page);
  await expect(page.locator('html')).toHaveAttribute('data-native-material', 'mica');
  await expectThemeBackground(page, 'warm-stone-dark');
  await expect(page.locator('[data-appbar]')).toHaveCSS('background-color', 'rgb(24, 24, 24)');
});

// RF-201：验证真实页面订阅 media 事件；模拟 IPC 故意给出相反主题，不能成为移动端来源。
// 浏览器只证明 DOM/事件/参数链路，原生状态栏与系统切色另有设备验收。
for (const platform of ['android', 'ios'] as const) {
  for (const preset of ['system', 'light', 'dark'] as const) {
    test(`RF-201 ${platform} ${preset}：移动端跟随媒体并保留显式偏好`, async ({ page }) => {
      await page.emulateMedia({ colorScheme: 'light' });
      await mockTheme(
        page,
        preset === 'dark' ? 'warm-stone-dark' : 'warm-stone',
        'solid',
        preset === 'system' ? 'dark' : undefined,
        platform,
      );
      await page.goto('/login');
      const root = page.locator('html');
      await expect(page.locator('#startup-screen')).toHaveCount(0);
      await expect(root).toHaveAttribute('data-platform', platform);
      for (const mode of ['light', 'dark', 'light'] as const) {
        await page.emulateMedia({ colorScheme: mode });
        await expect
          .poll(() => page.evaluate(() => matchMedia('(prefers-color-scheme: dark)').matches))
          .toBe(mode === 'dark');
        // 等待两次真实绘制机会，避免显式主题断言抢在 media 事件交付之前。
        await page.evaluate(
          () =>
            new Promise<void>((resolve) =>
              requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
            ),
        );
        const expected = preset === 'system' ? mode : preset;
        await expect(root).toHaveAttribute('data-theme', expected);
        await expect(root).toHaveAttribute('data-status-bar-style', expected);
        expect(await page.evaluate(() => Reflect.get(window, '__RF201_SYSTEM_IPC_COUNT__'))).toBe(
          0,
        );
      }
    });
  }
}

for (const platform of ['android', 'ios'] as const) {
  for (const schemeId of ['warm-stone', 'warm-stone-dark', 'deep-ocean']) {
    test(`${platform} ${schemeId} 主内容正文和说明保持中性高对比`, async ({ page }) => {
      await page.setViewportSize({ width: 390, height: 844 });
      await mockTheme(page, schemeId, 'solid', undefined, platform);
      await login(page);
      const dark = getSchemeById(schemeId)!.mode === 'dark';
      const main = page.getByRole('main');
      await expect(main.getByRole('heading').first()).toHaveCSS(
        'color',
        dark ? 'rgb(255, 255, 255)' : 'rgb(17, 17, 17)',
      );
      await expect(main.locator('p').first()).toHaveCSS(
        'color',
        dark ? 'rgb(245, 245, 245)' : 'rgb(32, 32, 32)',
      );
      await page.screenshot({
        path: test.info().outputPath(`${platform}-${schemeId}-neutral-content.png`),
      });
    });
  }
}
