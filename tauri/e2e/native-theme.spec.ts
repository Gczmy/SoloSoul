import { expect, test, type Page } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { getSchemeById } from '../src/lib/themeSchemes';
import { login } from './fixtures/auth';

async function mockTheme(page: Page, schemeId: string, material = 'mica') {
  const scheme = getSchemeById(schemeId)!;
  await page.addInitScript({
    content:
      readFileSync('e2e/fixtures/tauriMock.js', 'utf8') +
      `
    window.__MOCK_PLATFORM__ = 'windows';
    localStorage.setItem('i18nextLng', 'en-US');
    const prefs = ${JSON.stringify({
      theme: scheme.mode,
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
        document.documentElement.dataset.captionRgb = [color.red, color.green, color.blue].join(',');
        return { material: '${material}', platform: 'windows', reduceMotion: false, highContrast: false };
      },
    };
    const originalInvoke = window.__TAURI_INTERNALS__.invoke;
    window.__TAURI_INTERNALS__.invoke = async (cmd, args) => {
      const result = await originalInvoke(cmd, args);
      if (cmd === 'ui_get_preferences' || cmd === 'user_data_get_preferences') Object.assign(result, prefs);
      return result;
    };
  `,
  });
}

async function expectThemeBackground(page: Page, schemeId: string, translucent: boolean) {
  const hex = getSchemeById(schemeId)!.variables['--bg-base'];
  const expected = [1, 3, 5].map((offset) => parseInt(hex.slice(offset, offset + 2), 16));
  await expect(page.locator('html')).toHaveAttribute('data-caption-rgb', expected.join(','));
  // 通过浏览器实际解析后的颜色比较，兼容 rgb()/color(srgb) 的不同序列化形式。
  const actual = await page.locator('html').evaluate((root) => {
    const canvas = document.createElement('canvas');
    canvas.width = canvas.height = 1;
    const context = canvas.getContext('2d')!;
    context.fillStyle = getComputedStyle(root).backgroundColor;
    context.fillRect(0, 0, 1, 1);
    return Array.from(context.getImageData(0, 0, 1, 1).data);
  });
  expected.forEach((component, index) =>
    expect(Math.abs(actual[index] - component)).toBeLessThanOrEqual(1),
  );
  if (translucent) {
    expect(actual[3]).toBeGreaterThan(210);
    expect(actual[3]).toBeLessThan(255);
    await expect(page.locator('body')).toHaveCSS('background-color', 'rgba(0, 0, 0, 0)');
    await expect(page.locator('#root')).toHaveCSS('background-color', 'rgba(0, 0, 0, 0)');
  } else {
    expect(actual[3]).toBe(255);
  }
}

for (const scheme of ['warm-stone-dark', 'deep-ocean', 'forest-night', 'soft-cream']) {
  test(`Windows ${scheme} 登录背景取自主题且只着色一层`, async ({ page }) => {
    await mockTheme(page, scheme);
    await page.goto('/login');
    await expect(page.locator('#startup-screen')).toHaveCount(0);
    await expectThemeBackground(page, scheme, true);
    await page.screenshot({ path: `test-results/login-${scheme}.png` });
  });
}

test('同一深色模式切换配色时，窗口背景与原生标题栏一起更新', async ({ page }) => {
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
    await expectThemeBackground(page, scheme, true);
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
  }
});

test('无 Mica 时采用当前主题实色，不受首帧缓存颜色锁定', async ({ page }) => {
  await mockTheme(page, 'warm-stone-dark', 'solid');
  await login(page);
  await page
    .locator('#desktop-navigation')
    .getByRole('button', { name: 'Settings', exact: true })
    .click();
  await page.getByText('Theme & Appearance', { exact: true }).click();
  await page.getByRole('button', { name: 'More Appearances', exact: true }).click();
  await page.getByRole('button', { name: 'Deep Ocean', exact: true }).click();
  await expectThemeBackground(page, 'deep-ocean', false);
});
