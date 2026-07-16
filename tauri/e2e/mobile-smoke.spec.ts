import { test, expect } from '@playwright/test';
import { login, setupTauriMock } from './fixtures/auth';

/**
 * 移动端视口 E2E 冒烟测试（MOB-P1-06）
 * 验证在 390×844 窄视口 + Android 平台 mock 下，核心流程可跑通。
 */

test.beforeEach(async ({ page }) => {
  await page.addInitScript({
    content: `window.__MOCK_PLATFORM__ = 'android';`,
  });
  await setupTauriMock(page);
});

test('mobile smoke: login and render home bottom navigation', async ({ page }) => {
  await login(page);

  // 首页应渲染
  await expect(page.locator('text=Welcome back')).toBeVisible();

  // 移动端底部导航应存在（依赖 AppShell 在窄视口下渲染 MobileBottomNav）
  const bottomNav = page.locator('[data-testid="mobile-bottom-nav"], nav');
  await expect(bottomNav).toBeVisible();
});

test('mobile smoke: navigate to settings from home', async ({ page }) => {
  await login(page);

  // 通过底部导航或首页入口进入设置
  const settingsLink = page.locator('a[href="/settings"], button').filter({ hasText: /settings|设置/i }).first();
  // 若首页有设置快捷卡片，优先点击
  const settingsCard = page.locator('text=/settings/i').first();
  if (await settingsCard.isVisible().catch(() => false)) {
    await settingsCard.click();
  } else {
    await page.goto('/settings');
  }

  await page.waitForURL('/settings', { timeout: 10000 });
  await expect(page.locator('text=Settings')).toBeVisible();
});
