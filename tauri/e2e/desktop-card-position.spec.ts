import { expect, test } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { login } from './fixtures/auth';

type CardFrames = {
  running: boolean;
  settled: boolean;
  frames: {
    top: number;
    inset: number;
    rect?: { x: number; y: number; width: number; height: number };
  }[];
};

for (const side of ['left', 'right'] as const) {
  test(`${side} 展开侧栏的快捷卡片首帧及 AI 懒加载保持原位`, async ({ page }) => {
    await page.addInitScript({
      content:
        readFileSync('e2e/fixtures/tauriMock.js', 'utf8') +
        `
        window.__MOCK_PLATFORM__ = 'macos';
        window.__E2E_MOCKS__ = {
          vault_check_directory: () => true,
          ocr_get_model_status: () => ({ installed: true, bundled: true }),
          sync_list_conflicts: () => [],
        };
        localStorage.setItem('i18nextLng', 'en-US');
        const originalInvoke = window.__TAURI_INTERNALS__.invoke;
        window.__TAURI_INTERNALS__.invoke = async (cmd, args) => {
          const result = await originalInvoke(cmd, args);
          if (cmd === 'user_data_get_preferences') result.sidebarPosition = '${side}';
          return result;
        };
      `,
    });

    let releaseChat!: () => void;
    const chatChunkReady = new Promise<void>((resolve) => {
      releaseChat = resolve;
    });
    await page.route('**/AiQuickChatPopover.tsx*', async (route) => {
      await chatChunkReady;
      await route.continue();
    });
    await login(page);
    const sidebar = page.locator('#desktop-navigation');
    await expect(sidebar).toHaveCSS('width', '232px');

    for (const card of [
      {
        button: '[data-ai-button] button',
        selector: '[data-testid="quick-chat-loading"], [data-ai-quick-chat]',
        height: 520,
      },
      { button: '[data-ocr-button] button', selector: '[data-ocr-quick-scan]', height: 560 },
      {
        button: '[data-plugin-button] button',
        selector: '[role="dialog"][aria-label="Plugins"]',
        height: 560,
      },
    ]) {
      const button = sidebar.locator(card.button);
      await button.scrollIntoViewIfNeeded();
      const buttonBounds = (await button.boundingBox())!;
      const expectedTop = Math.min(Math.max(buttonBounds.y, 8), 720 - card.height - 8);

      // 从点击前逐帧采样，避免只检查动画结束后的最终位置而漏掉闪跳。
      await page.evaluate(
        ({ selector, side }) => {
          const state: CardFrames = { running: true, settled: false, frames: [] };
          Object.assign(window, { __cardFrames: state });
          const sample = () => {
            for (const el of document.querySelectorAll(selector)) {
              const style = getComputedStyle(el);
              if (style.visibility === 'hidden' || style.position !== 'fixed') continue;
              const { x, y, width, height } = el.getBoundingClientRect();
              state.frames.push({
                top: parseFloat(style.top),
                inset: parseFloat(style[side]),
                rect: state.settled ? { x, y, width, height } : undefined,
              });
            }
            if (state.running) requestAnimationFrame(sample);
          };
          requestAnimationFrame(sample);
        },
        { selector: card.selector, side },
      );

      await button.click();
      if (card.height === 520) {
        const loading = page.getByTestId('quick-chat-loading');
        await expect(loading).toBeVisible();
        const loadingBounds = (await loading.boundingBox())!;
        const inset =
          side === 'left' ? loadingBounds.x : 1280 - loadingBounds.x - loadingBounds.width;
        // 首次 chunk 延迟时，占位也必须位于展开侧栏外侧。
        expect(inset).toBeGreaterThanOrEqual(232);
        // 等首次入场结束，再加载正文；记录正文替换期间的每帧边界，捕获动画重播。
        await page.locator('[data-ai-quick-chat]').evaluate(async (el) => {
          await Promise.all(el.getAnimations().map((animation) => animation.finished));
          (window as unknown as { __cardFrames: CardFrames }).__cardFrames.settled = true;
        });
        releaseChat();
        await expect(loading).toHaveCount(0);
        await expect(page.locator('[data-ai-quick-chat]')).toBeVisible();
      } else {
        await expect(page.locator(card.selector)).toBeVisible();
      }

      const frames = await page.evaluate(async () => {
        const state = (
          window as unknown as {
            __cardFrames: CardFrames;
          }
        ).__cardFrames;
        // 跨过入场动画及 Suspense 内容替换，再检查所有实际绘制帧的定位。
        await new Promise((resolve) => setTimeout(resolve, 350));
        state.running = false;
        return state.frames;
      });
      expect(frames.length).toBeGreaterThan(0);
      for (const frame of frames) {
        expect(frame.top).toBeCloseTo(expectedTop, 0);
        expect(frame.inset).toBe(236);
        if (frame.rect) {
          expect(frame.rect.x).toBeCloseTo(side === 'left' ? 236 : 1280 - 236 - 380, 1);
          expect(frame.rect.y).toBeCloseTo(expectedTop, 1);
          expect(frame.rect.width).toBeCloseTo(380, 1);
          expect(frame.rect.height).toBeCloseTo(520, 1);
        }
      }
      await page.keyboard.press('Escape');
      await expect(page.locator(card.selector)).toHaveCount(0);
    }

    // 关闭后改变窗口尺寸并折叠侧栏，重新打开不能沿用上次的纵向坐标。
    await page.setViewportSize({ width: 1000, height: 900 });
    await sidebar.getByRole('button', { name: 'Collapse sidebar' }).click();
    await sidebar.getByRole('button', { name: 'Tools', exact: true }).hover();
    const aiButton = sidebar.locator('[data-ai-button] button');
    await aiButton.scrollIntoViewIfNeeded();
    await aiButton.click();
    const chat = page.locator('[data-ai-quick-chat]');
    await expect(chat).toHaveCSS(side, '52px');
    const anchor = (await aiButton.boundingBox())!;
    await expect(chat).toHaveCSS('top', `${Math.min(Math.max(anchor.y, 8), 372)}px`);
    await page.setViewportSize({ width: 1000, height: 600 });
    await expect(chat).toHaveCSS('top', '72px');
    await expect(chat).toHaveCSS('height', '520px');
    await page.keyboard.press('Escape');
    await expect(chat).toHaveCount(0);
  });
}

for (const side of ['top', 'bottom'] as const) {
  test(`${side} 横向导航的快捷卡片位于按钮内侧`, async ({ page }) => {
    await page.addInitScript({
      content:
        readFileSync('e2e/fixtures/tauriMock.js', 'utf8') +
        `
        window.__E2E_MOCKS__ = {
          vault_check_directory: () => true,
          ocr_get_model_status: () => ({ installed: true, bundled: true }),
          sync_list_conflicts: () => [],
        };
        const originalInvoke = window.__TAURI_INTERNALS__.invoke;
        window.__TAURI_INTERNALS__.invoke = async (cmd, args) => {
          const result = await originalInvoke(cmd, args);
          if (cmd === 'user_data_get_preferences') result.sidebarPosition = '${side}';
          return result;
        };
      `,
    });
    await login(page);
    await page.locator('[class*="horizontalArrowToggle"]').hover();
    const button = page.locator('[data-ai-button] button');
    await button.scrollIntoViewIfNeeded();
    await button.click();
    const chat = page.locator('[data-ai-quick-chat]');
    const anchor = (await button.boundingBox())!;
    await expect(chat).toHaveCSS(
      'top',
      `${side === 'top' ? anchor.y + anchor.height + 8 : anchor.y - 520 - 8}px`,
    );
    await expect(chat).toHaveCSS('right', '12px');
    await expect(chat).toBeInViewport();
  });
}
