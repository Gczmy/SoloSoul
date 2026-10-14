import { test, expect } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { login } from './fixtures/auth';

for (const position of ['left', 'right', 'top', 'bottom'] as const) {
  test(`Windows integrated ${position} titlebar keeps native buttons free`, async ({
    page,
  }, testInfo) => {
    test.skip(testInfo.project.name !== 'chromium', 'Windows desktop layout');
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
        set_titlebar_color: () => ({ platform: 'windows', material: 'mica', reduceMotion: false, highContrast: false }),
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
    await expect(header).toHaveCSS('background-color', 'rgba(0, 0, 0, 0)');
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
    if (position === 'left' || position === 'right') {
      const toggle = header.getByRole('button', { name: 'Collapse sidebar' });
      const before = await toggle.boundingBox();
      await toggle.click();
      await expect(page.locator('#desktop-navigation')).toHaveCSS('width', '96px');
      const after = await header.getByRole('button', { name: 'Expand sidebar' }).boundingBox();
      expect(after).toEqual(before);
      await expect(page.locator('#desktop-navigation')).toHaveCSS(
        'background-color',
        'rgba(0, 0, 0, 0)',
      );
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
