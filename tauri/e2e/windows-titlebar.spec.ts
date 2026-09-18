import { expect, test } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { login } from './fixtures/auth';

for (const position of ['left', 'right', 'top', 'bottom']) {
  test(`Windows Mica ${position} 操作栏与正文同色、导航透出材质且正文独立滚动`, async ({
    page,
  }) => {
    await page.setViewportSize({ width: 800, height: 600 });
    await page.addInitScript({
      content:
        readFileSync('e2e/fixtures/tauriMock.js', 'utf8') +
        `
      window.__MOCK_PLATFORM__ = 'windows';
      const layout = { platform: 'windows', titlebarHeight: 0 };
      window.__E2E_MOCKS__ = {
        vault_check_directory: () => true, ocr_get_model_status: () => ({ installed: true, bundled: true }),
        sync_list_conflicts: () => [], get_window_layout: () => layout,
        set_titlebar_color: () => ({ ...layout, material: 'mica', reduceMotion: false, highContrast: false }),
      };
      localStorage.setItem('i18nextLng', 'en-US');
      const invoke = window.__TAURI_INTERNALS__.invoke;
      window.__TAURI_INTERNALS__.invoke = async (cmd, args) => {
        const result = await invoke(cmd, args);
        if (cmd === 'user_data_get_preferences') result.sidebarPosition = '${position}';
        return result;
      };
    `,
    });
    await login(page);
    const header = page.locator('[data-appbar]');
    await expect(header).toHaveCSS('height', '40px');
    expect((await header.boundingBox())!.y).toBe(0);
    const guide = header.getByRole('button', { name: 'Guide', exact: true });
    const more = header.getByRole('button', { name: 'More actions', exact: true });
    if (await more.isVisible()) await more.click();
    await expect(guide).toBeVisible();
    const guideBounds = (await guide.boundingBox())!;
    expect(guideBounds.x + guideBounds.width).toBeLessThanOrEqual(800);
    const navigation =
      position === 'left' || position === 'right'
        ? page.locator('#desktop-navigation')
        : page.locator('header:not([data-appbar])');
    await expect(navigation).toHaveCSS('background-color', 'rgba(0, 0, 0, 0)');
    await expect(header).toHaveCSS('border-top-left-radius', '12px');
    await expect(header).toHaveCSS('border-top-right-radius', '12px');
    const content = page.locator('main');
    const bounds = (await content.boundingBox())!;
    expect(bounds.y).toBe(position === 'top' ? 88 : 40);
    expect(bounds.y + bounds.height).toBe(position === 'bottom' ? 544 : 592);
    await expect(content).toHaveCSS('border-top-left-radius', position === 'top' ? '12px' : '0px');
    await expect(content).toHaveCSS('border-top-right-radius', position === 'top' ? '12px' : '0px');
    const surface = await content.evaluate((el) => getComputedStyle(el).backgroundColor);
    expect(surface).not.toBe('rgba(0, 0, 0, 0)');
    await expect(header).toHaveCSS('background-color', surface);
    await expect(header).toHaveCSS('border-bottom-color', 'rgba(0, 0, 0, 0)');
    await expect(header).toHaveCSS('box-shadow', 'none');
    await expect(content).toHaveCSS('box-shadow', 'none');
    const headerBounds = (await header.boundingBox())!;
    expect(headerBounds.x).toBe(bounds.x);
    expect(headerBounds.width).toBe(bounds.width);
    // 正文滚动后仍在独立的裁剪区域内，不会穿到操作栏下面。
    await content.evaluate((el) => {
      const filler = document.createElement('div');
      filler.dataset.scrollProbe = 'true';
      filler.style.height = '2000px';
      el.appendChild(filler);
      el.scrollTop = 500;
    });
    expect(await content.evaluate((el) => el.scrollTop)).toBeGreaterThan(0);
    expect((await content.boundingBox())!.y).toBe(bounds.y);
    await content.evaluate((el) => {
      el.querySelector('[data-scroll-probe]')?.remove();
      el.scrollTop = 0;
    });
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBe(800);
    await page.screenshot({ path: `test-results/windows-titlebar-${position}.png` });
  });
}
