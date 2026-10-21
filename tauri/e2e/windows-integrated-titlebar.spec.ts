import { test, expect } from '@playwright/test';
import { readFileSync, writeFileSync } from 'node:fs';
import { login } from './fixtures/auth';

for (const motion of ['normal', 'native', 'system'] as const) {
  for (const position of ['left', 'right', 'top', 'bottom'] as const) {
    test(`Windows integrated ${position} titlebar keeps native buttons free${motion === 'normal' ? '' : ` (${motion} reduced motion)`}`, async ({
      page,
    }, testInfo) => {
      test.skip(testInfo.project.name !== 'chromium', 'Windows desktop layout');
      await page.emulateMedia({ reducedMotion: motion === 'system' ? 'reduce' : 'no-preference' });
      await page.addInitScript({
        content:
          readFileSync('e2e/fixtures/tauriMock.js', 'utf8') +
          `
      window.__MOCK_PLATFORM__ = 'windows';
      let frame = false;
      window.__FRAME_CONTROLS__ = [];
      window.__E2E_MOCKS__ = {
        vault_check_directory: () => true,
        ocr_get_model_status: () => ({ installed: true, bundled: true }),
        sync_list_conflicts: () => [],
        set_titlebar_color: () => ({ platform: 'windows', material: 'mica', reduceMotion: ${motion === 'native'}, highContrast: false }),
        set_titlebar_controls: (args) => {
          if (args.windowsFrame !== undefined) frame = args.windowsFrame;
          window.__FRAME_CONTROLS__ = args.regions;
        },
        get_window_layout: () => ({ platform: 'windows', windowsFrame: frame ? {
          enabled: true, headerHeight: 52, dpi: 96, maximized: false,
          captionBounds: [window.innerWidth - 146, 0, 146, 30]
        } : undefined })
      };
      const base = window.__TAURI_INTERNALS__.invoke;
      window.__TAURI_INTERNALS__.invoke = async (command, args) => {
        const result = await base(command, args);
        if (command === 'ui_get_preferences' || command === 'user_data_get_preferences') result.sidebarPosition = '${position}';
        return result;
      };
    `,
      });
      await page.goto('/login');
      await expect(page.locator('[data-windows-titlebar]')).toBeVisible();
      await expect(page.locator('html')).toHaveAttribute('data-windows-frame', 'true');
      await login(page);
      const header = page.locator('[data-appbar]');
      await expect(header).toHaveCSS('height', '52px');
      expect((await header.boundingBox())!.x).toBe(0);
      const toggleBounds = (await header
        .getByRole('button', { name: 'Hide pages and objects' })
        .boundingBox())!;
      expect(toggleBounds).toMatchObject({ x: 10, y: 8, width: 36, height: 36 });
      const headerButtons = await header.locator('button:visible').evaluateAll((buttons) =>
        buttons.map((button) => {
          const r = button.getBoundingClientRect();
          return { top: r.top, bottom: r.bottom, height: r.height };
        }),
      );
      expect(headerButtons.length).toBeGreaterThan(0);
      expect(headerButtons.every((button) => button.top === 8 && button.bottom === 44)).toBe(true);
      const shellColor = await page
        .locator('html')
        .evaluate((el) => getComputedStyle(el).backgroundColor);
      await expect(header).toHaveCSS('background-color', shellColor);
      const controls = header.locator('[data-titlebar-control]');
      const bounds = await controls.evaluateAll((items) =>
        items.map((item) => {
          const r = item.getBoundingClientRect();
          return { top: r.top, bottom: r.bottom, right: r.right, width: r.width };
        }),
      );
      expect(
        bounds
          .filter((r) => r.width > 0)
          .every((r) => r.top >= 0 && r.bottom <= 52 && r.right <= 1280 - 146),
      ).toBe(true);
      const surface = page.locator('[data-shell-surface]');
      await expect(surface).toHaveCSS('border-radius', '12px');
      const content = page.locator('[data-shell-content]');
      await expect(content).toHaveCSS('border-radius', '0px');
      await expect(page.locator('#desktop-navigation')).toHaveAttribute('data-position', 'left');
      {
        const nav = page.locator('#desktop-navigation');
        // Record production geometry reads during the switch, before a later RAF can hide a stale width.
        await nav.evaluate((navigation) => {
          const original = Element.prototype.getBoundingClientRect;
          const mismatches: Array<{ expanded: string | null; width: number }> = [];
          Object.assign(window, { __SIDEBAR_LAYOUT_MISMATCHES__: mismatches });
          Element.prototype.getBoundingClientRect = function () {
            const rect = original.call(this);
            if (this === navigation || navigation.contains(this)) {
              const expanded = navigation.getAttribute('data-expanded');
              const width = original.call(navigation).width;
              const expected = 56;
              if (Math.abs(width - expected) > 0.5) mismatches.push({ expanded, width });
            }
            return rect;
          };
          const traces: unknown[] = [];
          Object.assign(window, { __SIDEBAR_FRAME_TRACES__: traces });
          new MutationObserver(() => {
            const trace: { expanded: string | null; frames: unknown[] } = {
              expanded: navigation.getAttribute('data-expanded'),
              frames: [],
            };
            traces.push(trace);
            const start = performance.now();
            const sample = () => {
              if (navigation.getAttribute('data-expanded') !== trace.expanded) return;
              trace.frames.push({
                elapsed: performance.now() - start,
                geometry: Array.from(
                  navigation.querySelectorAll('button, button > svg, button > span'),
                ).map((element) => {
                  const rect = original.call(element);
                  const style = getComputedStyle(element);
                  return {
                    tag: element.tagName,
                    label: element.closest('button')?.getAttribute('aria-label'),
                    rect: [rect.x, rect.y, rect.width, rect.height],
                    transform: style.transform,
                    transition: style.transition,
                    animations: element.getAnimations().map((animation) => ({
                      type: animation.constructor.name,
                      property: (animation as CSSTransition).transitionProperty,
                    })),
                  };
                }),
              });
              if (performance.now() - start < 350) requestAnimationFrame(sample);
            };
            sample();
          }).observe(document.querySelector('[data-page-pane-body]')!, {
            attributes: true,
            attributeFilter: ['data-pane-open'],
          });
        });
        const toggle = header.getByRole('button', { name: 'Hide pages and objects' });
        const before = await toggle.boundingBox();
        const navBounds = (await nav.boundingBox())!;
        expect(before!.y).toBeGreaterThanOrEqual(0);
        expect(before!.x).toBeGreaterThanOrEqual(navBounds.x);
        expect(before!.x + before!.width).toBeLessThanOrEqual(navBounds.x + navBounds.width);
        await expect(nav.getByRole('button', { name: 'Hide pages and objects' })).toHaveCount(0);
        const track = await page
          .locator('#desktop-navigation')
          .getByRole('button', { name: 'Identity', exact: true })
          .locator('[data-label-track]')
          .elementHandle();
        const stableSurface = await surface.elementHandle();
        await toggle.click();
        await expect(page.locator('#desktop-navigation')).toHaveCSS('width', '56px');
        expect(
          await page.evaluate(
            () =>
              (window as unknown as { __SIDEBAR_LAYOUT_MISMATCHES__: unknown[] })
                .__SIDEBAR_LAYOUT_MISMATCHES__,
          ),
        ).toEqual([]);
        const after = await header
          .getByRole('button', { name: 'Show pages and objects' })
          .boundingBox();
        expect(after!.y).toBe(before!.y);
        expect(after!.x + after!.width / 2).toBe(navBounds.x + navBounds.width / 2);
        await expect(page.locator('#desktop-navigation')).toHaveCSS('background-color', shellColor);
        for (let i = 0; i < 8; i++) {
          await header
            .getByRole('button', {
              name: i % 2 === 0 ? 'Show pages and objects' : 'Hide pages and objects',
            })
            .click();
          await expect(page.locator('#desktop-navigation')).toHaveCSS('width', '56px');
          expect(await track!.evaluate((element) => element.isConnected)).toBe(true);
          expect(await stableSurface!.evaluate((element) => element.isConnected)).toBe(true);
          expect(
            await page.evaluate(
              () =>
                new Promise<boolean>((resolve) =>
                  requestAnimationFrame(() => {
                    const root = document.documentElement;
                    const surface = document.querySelector('[data-shell-surface]')!;
                    resolve(
                      Boolean(root.style.getPropertyValue('--sidebar-width')) &&
                        Boolean(root.style.getPropertyValue('--shell-content-left')) &&
                        getComputedStyle(surface).opacity === '1' &&
                        surface.getBoundingClientRect().width > 0,
                    );
                  }),
                ),
            ),
          ).toBe(true);
          const lastPage = (await nav
            .locator('[class*="navPrimary"] button')
            .last()
            .boundingBox())!;
          const addPage = (await nav
            .getByRole('button', { name: 'Add Page', exact: true })
            .boundingBox())!;
          const gap = addPage.y - lastPage.y - lastPage.height;
          expect(gap).toBeGreaterThanOrEqual(0);
          expect(gap).toBeLessThanOrEqual(8);
          expect(
            await page.evaluate(
              () =>
                (window as unknown as { __SIDEBAR_LAYOUT_MISMATCHES__: unknown[] })
                  .__SIDEBAR_LAYOUT_MISMATCHES__,
            ),
          ).toEqual([]);
        }
        await page.waitForTimeout(400);
        const traces = await page.evaluate(
          () =>
            (
              window as unknown as {
                __SIDEBAR_FRAME_TRACES__: Array<{
                  expanded: string;
                  frames: Array<{ geometry: Array<{ rect: number[] }> }>;
                }>;
              }
            ).__SIDEBAR_FRAME_TRACES__,
        );
        const tracePath = testInfo.outputPath(`sidebar-${position}-frames.json`);
        writeFileSync(tracePath, JSON.stringify(traces, null, 2));
        await testInfo.attach(`sidebar-${position}-frames`, {
          path: tracePath,
          contentType: 'application/json',
        });
        for (const trace of traces) {
          const first = trace.frames[0].geometry.map((node) => node.rect);
          for (const frame of trace.frames.slice(1)) {
            expect(frame.geometry.map((node) => node.rect)).toEqual(first);
          }
        }
      }
      await page.setViewportSize({ width: 800, height: 600 });
      await expect(header).toHaveCSS('height', '52px');
      expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBe(800);
      await expect
        .poll(() =>
          header.locator('button:visible').evaluateAll((buttons) =>
            buttons.every((button) => {
              const r = button.getBoundingClientRect();
              return r.y >= 52 || r.right <= window.innerWidth - 146;
            }),
          ),
        )
        .toBe(true);
      await page.setViewportSize({ width: 1440, height: 900 });
      await page.setViewportSize({ width: 900, height: 600 });
      await expect
        .poll(() =>
          header.locator('button:visible').evaluateAll((buttons) =>
            buttons.every((button) => {
              const r = button.getBoundingClientRect();
              return r.y >= 52 || r.right <= window.innerWidth - 146;
            }),
          ),
        )
        .toBe(true);
      await page.screenshot({ path: `test-results/windows-integrated-${position}.png` });
    });
  }
}
