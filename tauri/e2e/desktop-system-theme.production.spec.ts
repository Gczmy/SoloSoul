import { expect, test, type Page } from '@playwright/test';
import { readFileSync } from 'node:fs';

async function emitMode(page: Page, mode: 'light' | 'dark') {
  await page.evaluate(async (mode) => {
    Reflect.set(window, '__RF1193_MODE__', mode);
    const internals = Reflect.get(window, '__TAURI_INTERNALS__');
    await internals.invoke('plugin:event|emit', {
      event: 'system-theme-changed',
      payload: { event: 'system-theme-changed', id: 0, payload: mode },
    });
  }, mode);
}

for (const platform of ['windows', 'macos'] as const) {
  test(`${platform} system follows native notifications across login, home and explicit overrides`, async ({
    page,
  }) => {
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.emulateMedia({ colorScheme: 'light' });
    await page.addInitScript({
      content:
        readFileSync('e2e/fixtures/tauriMock.js', 'utf8') +
        `
      window.__MOCK_PLATFORM__ = '${platform}';
      window.__RF1193_MODE__ = 'dark';
      const prefs = { theme: 'system', hasSeenOnboarding: true, accentColor:'ocean',
        defaultLightTheme:'warm-stone',defaultDarkTheme:'warm-stone-dark' };
      localStorage.setItem('i18nextLng','en-US');
      localStorage.setItem('solosoul_ui_prefs',JSON.stringify(prefs));
      window.__E2E_MOCKS__ = {
        vault_check_directory: () => true,
        ocr_get_model_status: () => ({ installed:true,bundled:true }),
        sync_list_conflicts: () => [],
        set_titlebar_color: ({color}) => {
          document.documentElement.dataset.nativeThemeRgb = [color.red,color.green,color.blue].join(',');
          return {platform:'${platform}',material:'${platform === 'macos' ? 'liquid-glass' : 'mica'}',
            titlebarHeight:52,trafficLightsRight:${platform === 'macos' ? 80 : 0},reduceMotion:false,highContrast:false};
        }
      };
      const invoke=window.__TAURI_INTERNALS__.invoke;
      window.__TAURI_INTERNALS__.invoke = async (cmd,args) => {
        if(cmd==='get_system_theme') return window.__RF1193_MODE__;
        const result=await invoke(cmd,args);
        if(cmd==='ui_get_preferences'||cmd==='user_data_get_preferences') return {...result,...prefs};
        return result;
      };
    `,
    });
    const root = page.locator('html');
    async function expectMode(mode: 'light' | 'dark') {
      await expect(root).toHaveAttribute('data-theme', mode);
      await expect(root).toHaveAttribute(
        'data-native-theme-rgb',
        mode === 'dark' ? '24,24,24' : '243,243,243',
      );
      await expect
        .poll(() => root.evaluate((el) => el.style.getPropertyValue('--startup-background')))
        .toBe(mode === 'dark' ? '#212121' : '#ffffff');
      await expect(root).toHaveCSS('color-scheme', mode);
    }
    await page.goto('/login');
    await expect(page.locator('[data-login-card]')).toBeVisible();
    await expect(page.locator('#startup-screen')).toHaveCount(0);
    await expectMode('dark'); // Native result wins over the deliberately opposite media query.
    await emitMode(page, 'light');
    await expectMode('light');
    await page.locator('input[type="password"]').fill('any-password');
    await page.locator('button[type="submit"]').click();
    await expect(page).toHaveURL('/');
    await expectMode('light');
    for (const mode of ['dark', 'light', 'dark'] as const) await emitMode(page, mode);
    await expectMode('dark');
    await page
      .locator('#desktop-navigation')
      .getByRole('button', { name: 'Settings', exact: true })
      .click();
    await page.getByText('Theme & Appearance', { exact: true }).click();
    await page.getByRole('radio', { name: /^Light/ }).check();
    await expectMode('light');
    await emitMode(page, 'light');
    await emitMode(page, 'dark');
    await expectMode('light');
    await page.getByRole('button', { name: 'More Appearances' }).click();
    await page
      .getByRole('region', { name: 'Theme Schemes' })
      .getByRole('button', { name: 'System', exact: true })
      .click();
    await expectMode('dark');
    await page.screenshot({
      path: test.info().outputPath('system-theme-handoff.png'),
      animations: 'disabled',
    });
  });
}
