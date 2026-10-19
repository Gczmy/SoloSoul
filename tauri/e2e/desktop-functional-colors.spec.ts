import { expect, test } from '@playwright/test';
import { login, setupTauriMock } from './fixtures/auth';
import { getSchemeById } from '../src/lib/themeSchemes';

test.use({ viewport: { width: 1200, height: 800 }, isMobile: false, hasTouch: false });

for (const platform of ['macos', 'windows'] as const) {
  for (const position of ['left', 'right'] as const) {
    for (const schemeId of ['warm-stone', 'soft-cream', 'warm-stone-dark', 'deep-ocean']) {
      test(`${platform} ${schemeId} ${position}: neutral navigation has no vertical separator`, async ({
        page,
      }) => {
        const scheme = getSchemeById(schemeId)!;
        await setupTauriMock(page);
        await page.addInitScript(
          ({ platform, schemeId, mode, position }) => {
            const prefs = {
              theme: mode,
              sidebarPosition: position,
              defaultLightTheme: mode === 'light' ? schemeId : 'warm-stone',
              defaultDarkTheme: mode === 'dark' ? schemeId : 'warm-stone-dark',
              language: 'en-US',
              hasSeenOnboarding: true,
              autoLockTimeoutMinutes: 0,
            };
            Object.assign(window, {
              __MOCK_PLATFORM__: platform,
              __E2E_MOCKS__: {
                ui_get_preferences: () => prefs,
                user_data_get_preferences: () => prefs,
                vault_check_directory: () => true,
                sync_list_conflicts: () => [],
                set_titlebar_color: () => ({
                  platform,
                  material: platform === 'macos' ? 'liquid-glass' : 'mica',
                  highContrast: false,
                  reduceMotion: false,
                  titlebarHeight: 52,
                  trafficLightsRight: 79,
                }),
              },
            });
            localStorage.setItem('i18nextLng', 'en-US');
          },
          { platform, schemeId, mode: scheme.mode, position },
        );
        await login(page);
        const main = page.getByRole('main');
        await expect(main.getByRole('heading').first()).toHaveCSS(
          'color',
          scheme.mode === 'dark' ? 'rgb(255, 255, 255)' : 'rgb(17, 17, 17)',
        );
        await expect(main.locator('p').first()).toHaveCSS(
          'color',
          scheme.mode === 'dark' ? 'rgb(245, 245, 245)' : 'rgb(32, 32, 32)',
        );
        const nav = page.locator('#desktop-navigation');
        await expect(nav).toBeVisible();
        expect((await nav.boundingBox())!.width).toBe(186);
        await expect(nav.getByRole('img', { name: 'SoloSoul', exact: true })).toHaveCount(0);
        const home = nav.getByRole('button', { name: 'Home', exact: true });
        const ink = scheme.mode === 'dark' ? 'rgb(180, 180, 180)' : 'rgb(82, 82, 82)';
        const primaryInk = scheme.mode === 'dark' ? 'rgb(236, 236, 236)' : 'rgb(36, 36, 36)';
        await expect(home).toHaveCSS('color', primaryInk);
        await expect(nav.getByRole('button', { name: 'Identity', exact: true })).toHaveCSS(
          'color',
          ink,
        );
        await expect(page.locator('[data-appbar] h1')).toHaveCSS('color', primaryInk);
        await home.hover();
        await expect(home).toHaveCSS('color', primaryInk);
        await page.mouse.move(600, 700);
        const sample = await nav.evaluate((element) => {
          const canvas = document.createElement('canvas');
          canvas.width = canvas.height = 1;
          const ctx = canvas.getContext('2d')!;
          const rgba = (value: string) => {
            ctx.clearRect(0, 0, 1, 1);
            ctx.fillStyle = value;
            ctx.fillRect(0, 0, 1, 1);
            return Array.from(ctx.getImageData(0, 0, 1, 1).data);
          };
          const root = getComputedStyle(document.documentElement);
          return {
            shell: rgba(getComputedStyle(document.body, '::before').backgroundColor),
            nav: rgba(getComputedStyle(element).backgroundColor),
            base: rgba(root.getPropertyValue('--bg-base')),
            content: rgba(
              getComputedStyle(document.querySelector('[data-shell-surface]')!).backgroundColor,
            ),
            text: rgba(
              getComputedStyle(element.querySelector('button[aria-current="page"]')!).color,
            ),
            leftBorder: parseFloat(getComputedStyle(element).borderLeftWidth),
            rightBorder: parseFloat(getComputedStyle(element).borderRightWidth),
          };
        });
        const luma = (rgb: number[]) =>
          rgb
            .slice(0, 3)
            .map((v) => {
              const n = v / 255;
              return n <= 0.04045 ? n / 12.92 : ((n + 0.055) / 1.055) ** 2.4;
            })
            .reduce((sum, v, i) => sum + v * [0.2126, 0.7152, 0.0722][i], 0);
        const contrast = (a: number[], b: number[]) =>
          (Math.max(luma(a), luma(b)) + 0.05) / (Math.min(luma(a), luma(b)) + 0.05);
        const composite = (paint: number[], base: number[]) =>
          paint.slice(0, 3).map((v, i) => (v * paint[3]) / 255 + base[i] * (1 - paint[3] / 255));
        await test.info().attach('navigation-palette-measurement', {
          body: JSON.stringify(sample),
          contentType: 'application/json',
        });
        if (platform === 'macos') {
          expect(sample.content).toEqual(
            scheme.mode === 'light' ? [255, 255, 255, 255] : sample.base,
          );
          if (scheme.mode === 'dark') {
            // 新设计采用较暗的中性侧栏，保留与正文的可见明度层次。
            const paintedShell = composite(sample.shell, [0, 0, 0]);
            expect(
              paintedShell.reduce(
                (sum, channel, index) => sum + Math.abs(channel - sample.base[index]),
                0,
              ),
            ).toBeGreaterThan(20);
          }
          // WebKit/Chromium对90% alpha的8位像素取整分别可能为229/230。
          expect(Math.abs(sample.shell[3] - 230)).toBeLessThanOrEqual(1);
          for (const underlay of [
            [0, 0, 0],
            [255, 255, 255],
          ]) {
            expect(contrast(sample.text, composite(sample.shell, underlay))).toBeGreaterThanOrEqual(
              4.5,
            );
          }
        } else {
          const painted = composite(sample.nav, sample.base);
          expect(contrast(sample.text, painted)).toBeGreaterThanOrEqual(4.5);
        }
        expect(sample.leftBorder).toBe(0);
        expect(sample.rightBorder).toBe(0);
        await page.screenshot({
          path: test.info().outputPath('desktop-functional-colors.png'),
          animations: 'disabled',
        });
        await page.getByRole('button', { name: 'Collapse sidebar', exact: true }).click();
        await expect(nav).toHaveCSS('width', '96px');
        await expect(nav).toHaveCSS('border-left-width', '0px');
        await expect(nav).toHaveCSS('border-right-width', '0px');
        const collapsedBox = (await nav.boundingBox())!;
        expect(position === 'left' ? collapsedBox.x : collapsedBox.x + collapsedBox.width).toBe(
          position === 'left' ? 0 : 1200,
        );
        await page.getByRole('button', { name: 'Expand sidebar', exact: true }).click();
        await expect(nav).toHaveCSS('width', '186px');
        await page.locator('html').evaluate((root) => {
          root.dataset.nativeMaterial = 'solid';
        });
        const fallback =
          platform === 'macos'
            ? await page
                .locator('body')
                .evaluate((el) => getComputedStyle(el, '::before').backgroundColor)
            : await nav.evaluate((el) => getComputedStyle(el).backgroundColor);
        expect(fallback).not.toContain('rgba');
        await expect(home).toHaveCSS('color', primaryInk);
        await page.emulateMedia({ forcedColors: 'active' });
        await expect(home).toBeVisible();
        await expect(home).not.toHaveCSS('color', 'rgba(0, 0, 0, 0)');
        await expect(nav).toHaveCSS('border-left-width', '0px');
        await expect(nav).toHaveCSS('border-right-width', '0px');
      });
    }
  }
}

test('Android top bar removes the brand image and preserves account and lock controls', async ({
  page,
}) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await setupTauriMock(page);
  await page.addInitScript(() => {
    Object.assign(window, {
      __MOCK_PLATFORM__: 'android',
      __E2E_MOCKS__: {
        vault_check_directory: () => true,
        vault_get_directory: () => ({ directoryType: 'local', valid: true }),
        ui_get_preferences: () => ({
          theme: 'light',
          language: 'en-US',
          hasSeenOnboarding: true,
          backupReminderDays: 0,
        }),
        user_data_get_preferences: () => ({
          theme: 'light',
          language: 'en-US',
          hasSeenOnboarding: true,
          backupReminderDays: 0,
        }),
        android_check_update: () => ({
          available: false,
          currentVersion: '2.13.2',
          latestVersion: '2.13.2',
        }),
        sync_list_conflicts: () => [],
      },
    });
    localStorage.setItem('i18nextLng', 'en-US');
  });
  await login(page);
  const header = page.locator('[data-appbar]');
  await expect(header.getByRole('heading', { name: 'SoloSoul', exact: true })).toBeVisible();
  await expect(header.getByRole('img', { name: 'SoloSoul', exact: true })).toHaveCount(0);
  await expect(header.getByRole('button', { name: /Lock vault/i })).toBeVisible();
  await expect(
    header.getByRole('button', { name: 'Account management', exact: true }),
  ).toBeVisible();
  await page.screenshot({ path: test.info().outputPath('mobile-topbar.png') });
});
