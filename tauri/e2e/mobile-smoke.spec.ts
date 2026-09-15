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
    content: `window.__MOCK_PLATFORM__ = 'android';
      window.__E2E_MOCKS__ = {
        vault_check_directory: () => true,
        object_list: () => [{ id: 'mobile-object', name: 'Mobile note', typeId: 'document', sensitivityLevel: 'internal', properties: {}, createdAt: '2026-09-15', updatedAt: '2026-09-15' }],
        object_get: () => ({ id: 'mobile-object', name: 'Mobile note', typeId: 'document', sensitivityLevel: 'internal', properties: {}, createdAt: '2026-09-15', updatedAt: '2026-09-15' }),
        attachment_count_stats: () => ({ attachmentCount: 0, photoCount: 0 }),
      };`,
  });
  await setupTauriMock(page);
});

test('mobile smoke: login and render home bottom navigation', async ({ page }) => {
  await login(page);

  // 首页应渲染
  await expect(page.locator('text=Welcome back')).toBeVisible();

  // 移动端底部导航应存在（真实 Android 平台使用四入口导航）
  const bottomNav = page.locator('[data-testid="mobile-bottom-nav"], nav');
  await expect(bottomNav).toBeVisible();
});

test('mobile smoke: navigate to settings from home', async ({ page }) => {
  await login(page);

  await page.locator('.android-navigation a[href="/settings"]').click();
  await expect(page).toHaveURL('/settings');
  await expect(page.getByRole('heading', { name: 'Settings', exact: true })).toBeVisible();
});

test('mobile smoke: bottom nav switches between top-level routes', async ({ page }) => {
  await login(page);

  for (const path of ['/workspace', '/tools', '/settings', '/']) {
    await page.locator(`.android-navigation a[href="${path}"]`).click();
    await expect(page).toHaveURL(path);
    await expect(page.locator(`.android-navigation a[href="${path}"]`)).toHaveAttribute(
      'aria-current',
      'page',
    );
  }
});

test('mobile smoke: object detail modal opens and closes', async ({ page }) => {
  await login(page);

  await page.locator('.android-navigation a[href="/workspace"]').click();
  // 通过真实对象入口打开详情。
  await page.waitForSelector('[data-testid="object-card"], [data-testid="workspace-object-card"]', {
    timeout: 10000,
  });
  const firstCard = page
    .locator('[data-testid="object-card"], [data-testid="workspace-object-card"]')
    .first();
  await firstCard.locator('button').first().click();

  // 详情弹窗/页面应出现
  const detail = page.locator(
    '[data-testid="object-detail-modal"], [data-testid="object-detail-page"]',
  );
  await expect(detail).toBeVisible({ timeout: 10000 });

  // 关闭弹窗
  const closeBtn = page
    .locator('[data-testid="object-detail-close"], button[aria-label="Close"]')
    .first();
  if (await closeBtn.isVisible().catch(() => false)) {
    await closeBtn.click();
    await expect(detail).not.toBeVisible();
  }
});

test('mobile smoke: tools keeps the mobile plugin entry reachable', async ({ page }) => {
  await login(page);
  await page.locator('.android-navigation a[href="/tools"]').click();
  await page
    .locator('.android-tool')
    .filter({ hasText: /plugin/i })
    .click();
  await expect(page).toHaveURL('/plugins');
  await expect(page.locator('header[data-appbar] h1')).toContainText(/plugin/i);
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
    expect(
      el.width,
      `Element <${el.tag}> width ${el.width}px is below 44px`,
    ).toBeGreaterThanOrEqual(43);
    expect(
      el.height,
      `Element <${el.tag}> height ${el.height}px is below 44px`,
    ).toBeGreaterThanOrEqual(43);
  }
});
