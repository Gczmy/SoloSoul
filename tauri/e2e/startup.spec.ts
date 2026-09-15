import { expect, test } from '@playwright/test';
import { readFileSync } from 'node:fs';

test('应用模块尚未加载时已有品牌首帧', async ({ page }) => {
  await page.route('**/src/main.tsx', (route) => route.abort());
  await page.goto('/', { waitUntil: 'domcontentloaded' });
  const startup = page.locator('#startup-screen');
  await expect(startup).toBeVisible();
  await expect(startup.locator('.startup-name')).toHaveText('SoloSoul');
  await expect(startup.locator('img')).toHaveJSProperty('naturalWidth', 1024);
  await expect(page.locator('#root')).toBeEmpty();
});

test('显式浅色偏好优先于系统深色，启动层无需 React 即可应用', async ({ page }) => {
  await page.emulateMedia({ colorScheme: 'dark' });
  await page.addInitScript(() => {
    localStorage.setItem('solosoul_ui_prefs', JSON.stringify({ theme: 'light' }));
  });
  await page.route('**/src/main.tsx', (route) => route.abort());
  await page.goto('/', { waitUntil: 'domcontentloaded' });
  await expect(page.locator('#startup-screen')).toHaveCSS('background-color', 'rgb(250, 250, 248)');
});

test('模块缺失时限时显示重试；迟到的就绪不会撤下错误页', async ({ page }) => {
  await page.clock.install();
  await page.route('**/src/main.tsx', (route) => route.abort());
  await page.goto('/', { waitUntil: 'domcontentloaded' });
  await page.clock.fastForward(8001);
  const startup = page.locator('#startup-screen');
  await expect(startup).toHaveAttribute('data-state', 'error');
  await page.getByRole('button', { name: 'Diagnostics', exact: true }).click();
  await expect(startup.locator('pre')).toContainText('reason: timeout');
  const ready = await page.evaluate(() => window.__SOLOSOUL_STARTUP__?.ready());
  expect(ready).toBe(false);
  await expect(startup).toBeVisible();
  await page.getByRole('button', { name: 'Restart', exact: true }).click();
  await expect(startup).toHaveAttribute('data-state', 'loading');
});

test('首帧使用上次已解析的配色，不把浏览器语言写成用户偏好', async ({ page }) => {
  await page.addInitScript(() => {
    localStorage.removeItem('i18nextLng');
    localStorage.setItem(
      'solosoul_ui_prefs',
      JSON.stringify({
        theme: 'light',
        startupTheme: {
          mode: 'light',
          background: '#f5f4f0',
          foreground: '#303030',
          secondary: '#666666',
        },
      }),
    );
  });
  await page.route('**/src/main.tsx', (route) => route.abort());
  await page.goto('/', { waitUntil: 'domcontentloaded' });
  await expect(page.locator('#startup-screen')).toHaveCSS('background-color', 'rgb(245, 244, 240)');
  expect(await page.evaluate(() => localStorage.getItem('i18nextLng'))).toBeNull();
});

test('偏好 IPC 不返回仍进入登录页，启动层完成交接', async ({ page }) => {
  await page.addInitScript({
    content:
      readFileSync('e2e/fixtures/tauriMock.js', 'utf8') +
      `
    const originalInvoke = window.__TAURI_INTERNALS__.invoke;
    window.__TAURI_INTERNALS__.invoke = function(cmd, args) {
      if (cmd === 'ui_get_preferences') return new Promise(() => {});
      return originalInvoke(cmd, args);
    };
  `,
  });
  await page.goto('/login');
  await expect(page.locator('#startup-screen')).toHaveCount(0, { timeout: 10000 });
  await expect(page.locator('button[type="submit"]')).toBeVisible();
});

test('原生确认玻璃材质后透出背景，系统强制颜色时回到实色', async ({ page }) => {
  await page.addInitScript({
    content:
      readFileSync('e2e/fixtures/tauriMock.js', 'utf8') +
      `
    window.__E2E_MOCKS__ = {
      set_titlebar_color: () => ({ material: 'liquid-glass', platform: 'macos', reduceMotion: false, highContrast: false }),
    };
  `,
  });
  await page.route('**/src/bootstrapApp.tsx', (route) => route.abort());
  await page.goto('/');
  await expect(page.locator('html')).toHaveAttribute('data-native-material', 'liquid-glass');
  await expect(page.locator('#startup-screen')).toHaveCSS('background-color', 'rgba(0, 0, 0, 0)');
  await page.emulateMedia({ forcedColors: 'active' });
  await expect(page.locator('#startup-screen')).not.toHaveCSS(
    'background-color',
    'rgba(0, 0, 0, 0)',
  );
});

for (const material of ['mica', 'acrylic']) {
  test(`Windows 首次显示请求发生在 ${material} 状态和图标就绪之后`, async ({ page }) => {
    await page.addInitScript({
      content:
        readFileSync('e2e/fixtures/tauriMock.js', 'utf8') +
        `
    window.__MOCK_PLATFORM__ = 'windows';
    window.__E2E_MOCKS__ = {
      set_titlebar_color: () => ({ material: '${material}', platform: 'windows', reduceMotion: false, highContrast: false }),
      show_main_window: () => {
        const logo = document.querySelector('.startup-logo');
        document.documentElement.dataset.firstFrameReady = String(
          document.documentElement.dataset.nativeMaterial === '${material}' && logo.complete && logo.naturalWidth > 0
        );
      },
    };
  `,
    });
    await page.route('**/src/bootstrapApp.tsx', (route) => route.abort());
    await page.goto('/');
    await expect(page.locator('html')).toHaveAttribute('data-first-frame-ready', 'true');
    await expect(page.locator('html')).toHaveAttribute('data-desktop-platform', 'windows');
  });
}

test('跟随系统时从双主题缓存选取当前配色', async ({ page }) => {
  await page.emulateMedia({ colorScheme: 'dark' });
  await page.addInitScript(() =>
    localStorage.setItem(
      'solosoul_ui_prefs',
      JSON.stringify({
        theme: 'system',
        startupThemes: {
          light: { background: '#f5f4f0', foreground: '#303030', secondary: '#666666' },
          dark: { background: '#202224', foreground: '#eeeeee', secondary: '#aaaaaa' },
        },
      }),
    ),
  );
  await page.route('**/src/main.tsx', (route) => route.abort());
  await page.goto('/');
  await expect(page.locator('#startup-screen')).toHaveCSS('background-color', 'rgb(32, 34, 36)');
});
