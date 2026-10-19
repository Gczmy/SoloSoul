import { expect, test } from '@playwright/test';
import { login, setupTauriMock } from './fixtures/auth';

for (const mode of ['light', 'dark'] as const) {
  for (const position of ['left', 'right'] as const) {
    test(`Windows ${mode} ${position}: shell shares one background around the content`, async ({
      page,
    }) => {
      await page.setViewportSize({ width: 1200, height: 800 });
      await page.emulateMedia({ colorScheme: mode });
      await setupTauriMock(page);
      await page.addInitScript(
        ({ mode, position }) => {
          const prefs = {
            theme: mode,
            sidebarPosition: position,
            defaultLightTheme: 'warm-stone',
            defaultDarkTheme: 'warm-stone-dark',
            language: 'en-US',
            hasSeenOnboarding: true,
            autoLockTimeoutMinutes: 0,
          };
          Object.assign(window, {
            __MOCK_PLATFORM__: 'windows',
            __E2E_MOCKS__: {
              ui_get_preferences: () => prefs,
              user_data_get_preferences: () => prefs,
              vault_check_directory: () => true,
              sync_list_conflicts: () => [],
              set_titlebar_color: () => ({
                platform: 'windows',
                material: 'mica',
                highContrast: false,
                reduceMotion: false,
              }),
              get_window_layout: () => ({
                platform: 'windows',
                windowsFrame: {
                  enabled: true,
                  headerHeight: 52,
                  dpi: 96,
                  maximized: false,
                  captionBounds: [window.innerWidth - 146, 0, 146, 30],
                },
              }),
            },
          });
          localStorage.setItem('i18nextLng', 'en-US');
        },
        { mode, position },
      );
      await page.goto('/login');
      const root = page.locator('html');
      const transparent = 'rgba(0, 0, 0, 0)';
      await expect(root).toHaveAttribute('data-native-material', 'mica');
      await expect(page.locator('[data-windows-titlebar]')).toHaveCSS(
        'background-color',
        transparent,
      );
      await login(page);
      const nav = page.locator('#desktop-navigation');
      const shell = page.locator('[data-windows-shell][data-navigation]');
      const header = page.locator('[data-appbar]');
      const contentColor = mode === 'light' ? 'rgb(255, 255, 255)' : 'rgb(33, 33, 33)';
      await expect(page.locator('[data-shell-surface]')).toHaveCSS(
        'background-color',
        contentColor,
      );
      for (const surface of [root, page.locator('body'), page.locator('#root'), shell, nav]) {
        await expect(surface).toHaveCSS('background-color', transparent);
      }
      await expect(header).toHaveCSS('background-color', transparent);
      await expect(header).toHaveCSS('background-image', 'none');
      await page.screenshot({
        path: test.info().outputPath('mica-shell.png'),
        animations: 'disabled',
        omitBackground: true,
      });

      // 系统减少透明度时，所有外壳面一并回退，不能只给侧栏铺灰色。
      const cdp = await page.context().newCDPSession(page);
      await cdp.send('Emulation.setEmulatedMedia', {
        features: [{ name: 'prefers-reduced-transparency', value: 'reduce' }],
      });
      expect(
        await page.evaluate(() => matchMedia('(prefers-reduced-transparency: reduce)').matches),
      ).toBe(true);
      const solidColor = mode === 'light' ? 'rgb(243, 243, 243)' : 'rgb(24, 24, 24)';
      for (const surface of [root, page.locator('body'), page.locator('#root'), nav]) {
        await expect(surface).toHaveCSS('background-color', solidColor);
      }
      await cdp.send('Emulation.setEmulatedMedia', { features: [] });
      await cdp.detach();
      await root.evaluate((el) => {
        el.dataset.nativeMaterial = 'solid';
      });
      for (const surface of [root, page.locator('body'), page.locator('#root'), shell, nav]) {
        await expect(surface).toHaveCSS('background-color', solidColor);
      }
      await expect(page.locator('[data-shell-surface]')).toHaveCSS(
        'background-color',
        contentColor,
      );
      await page.emulateMedia({ forcedColors: 'active' });
      const forcedBackground = await root.evaluate((el) => getComputedStyle(el).backgroundColor);
      await expect(nav).toHaveCSS('background-color', forcedBackground);
      await expect(nav.getByRole('button', { name: 'Home', exact: true })).toBeVisible();
    });
  }
}
