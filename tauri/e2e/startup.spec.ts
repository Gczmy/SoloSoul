import { expect, test } from '@playwright/test';

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
