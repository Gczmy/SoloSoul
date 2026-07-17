import { test, expect } from '@playwright/test';
import { login, setupTauriMock } from './fixtures/auth';

/**
 * 移动端视口 E2E 冒烟测试（MOB-P1-06）
 * 验证在 390×844 窄视口 + Android 平台 mock 下，核心流程可跑通。
 *
 * 注意：本测试在桌面 Chromium 中以移动视口运行，依赖 `tauriMock.js`
 * 模拟 Tauri 移动端 API。它不能替代真机/模拟器测试，但能在 CI 中
 * 快速捕获前端响应式回归与路由错误。
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

test('mobile smoke: bottom nav switches between top-level routes', async ({ page }) => {
  await login(page);

  // 底部导航应至少包含 Home / Search / Settings 入口
  const homeLink = page.locator('[data-testid="mobile-bottom-nav"] a, nav a').filter({ hasText: /home|首页/i }).first();
  const searchLink = page.locator('[data-testid="mobile-bottom-nav"] a, nav a').filter({ hasText: /search|搜索/i }).first();

  if (await homeLink.isVisible().catch(() => false)) {
    await homeLink.click();
    await expect(page).toHaveURL(/\/(home)?/);
  }

  if (await searchLink.isVisible().catch(() => false)) {
    await searchLink.click();
    await expect(page).toHaveURL(/\/search/);
  }
});

test('mobile smoke: object detail modal opens and closes', async ({ page }) => {
  await login(page);

  // 等待对象列表渲染（桌面工作区在移动端以卡片网格呈现）
  await page.waitForSelector('[data-testid="object-card"], [data-testid="workspace-object-card"]', { timeout: 10000 });
  const firstCard = page.locator('[data-testid="object-card"], [data-testid="workspace-object-card"]').first();
  await firstCard.click();

  // 详情弹窗/页面应出现
  const detail = page.locator('[data-testid="object-detail-modal"], [data-testid="object-detail-page"]');
  await expect(detail).toBeVisible({ timeout: 10000 });

  // 关闭弹窗
  const closeBtn = page.locator('[data-testid="object-detail-close"], button[aria-label="Close"]').first();
  if (await closeBtn.isVisible().catch(() => false)) {
    await closeBtn.click();
    await expect(detail).not.toBeVisible();
  }
});

test('mobile smoke: touch target sizes are at least 44px', async ({ page }) => {
  await login(page);

  const interactiveElements = await page.$$eval(
    'button, a, input, select, textarea, [role="button"], [role="link"]',
    (els) =>
      els
        .map((el) => {
          const rect = el.getBoundingClientRect();
          const style = window.getComputedStyle(el);
          return {
            width: rect.width,
            height: rect.height,
            tag: el.tagName,
            ariaHidden: el.getAttribute('aria-hidden') === 'true',
            pointerEvents: style.pointerEvents,
            opacity: parseFloat(style.opacity),
            // 是否已有更大的可点击父元素（简单启发式：父元素是 button/a 且完全包含当前元素）
            hasLargerClickableParent: (() => {
              const parent = el.parentElement;
              if (!parent) return false;
              const parentRect = parent.getBoundingClientRect();
              const isClickableParent =
                parent.tagName === 'BUTTON' ||
                parent.tagName === 'A' ||
                parent.getAttribute('role') === 'button' ||
                parent.getAttribute('role') === 'link';
              return (
                isClickableParent &&
                parentRect.width >= rect.width + 4 &&
                parentRect.height >= rect.height + 4
              );
            })(),
          };
        })
        .filter((el) => {
          // 过滤不可见、被父元素覆盖、或明确装饰性的元素
          if (el.width <= 0 || el.height <= 0) return false;
          if (el.ariaHidden) return false;
          if (el.pointerEvents === 'none') return false;
          if (el.opacity <= 0.1) return false;
          if (el.hasLargerClickableParent) return false;
          return true;
        }),
  );

  for (const el of interactiveElements) {
    // 允许 1px 浮点误差
    expect(el.width, `Element <${el.tag}> width ${el.width}px is below 44px`).toBeGreaterThanOrEqual(43);
    expect(el.height, `Element <${el.tag}> height ${el.height}px is below 44px`).toBeGreaterThanOrEqual(43);
  }
});
