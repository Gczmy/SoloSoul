import { expect, test } from '@playwright/test';
import { THEME_SCHEMES, getSchemeById } from '../src/lib/themeSchemes';
import { login, setupTauriMock } from './fixtures/auth';
import { measureControlContrast } from './fixtures/contrast';

const qrFailure = 'Synchronization is disabled. Enable it first.';

for (const platform of ['android', 'windows', 'macos'] as const) {
  for (const scheme of THEME_SCHEMES.map(
    (s) => getSchemeById(s.id, platform === 'android' ? 'shared' : 'desktop')!,
  )) {
    for (const contrast of ['no-preference', 'more'] as const) {
      test(`${platform} ${scheme.id} ${contrast} recovery errors stay readable`, async ({
        page,
      }) => {
        await page.setViewportSize({ width: platform === 'android' ? 390 : 1000, height: 844 });
        await page.emulateMedia({ contrast });
        await setupTauriMock(page);
        await page.addInitScript(
          ({ platform, mode, schemeId, qrFailure }) => {
            const prefs = {
              theme: mode,
              defaultLightTheme: mode === 'light' ? schemeId : 'warm-stone',
              defaultDarkTheme: mode === 'dark' ? schemeId : 'warm-stone-dark',
              language: 'en-US',
              hasSeenOnboarding: true,
              autoLockTimeoutMinutes: 0,
            };
            Object.assign(window, {
              __MOCK_PLATFORM__: platform,
              __RF1179_CAMERA_REQUESTS__: 0,
              __E2E_MOCKS__: {
                ui_get_preferences: () => prefs,
                user_data_get_preferences: () => prefs,
                vault_check_directory: () => true,
                ocr_get_model_status: () => ({ installed: true, bundled: true }),
                sync_get_status: () => ({
                  isDiscovering: false,
                  syncEnabled: true,
                  autoSyncEnabled: false,
                  localFingerprint: '0'.repeat(64),
                  connectedPeers: [],
                }),
                sync_listen_addr: () => '127.0.0.1:42069',
                sync_list_conflicts: () => [],
                mdns_discover: () => [],
                sync_generate_qr_payload: () => {
                  throw { code: 'SYNC_NOT_ENABLED', safeDetails: null, retryable: false };
                },
              },
            });
            Reflect.deleteProperty(window, 'BarcodeDetector');
            Object.defineProperty(navigator, 'mediaDevices', {
              configurable: true,
              value: {
                enumerateDevices: async () => [
                  {
                    deviceId: 'public-camera',
                    kind: 'videoinput',
                    label: 'Public camera',
                    groupId: 'public',
                  },
                ],
                getUserMedia: async () => {
                  (
                    window as unknown as { __RF1179_CAMERA_REQUESTS__: number }
                  ).__RF1179_CAMERA_REQUESTS__ += 1;
                  throw new DOMException('Camera permission denied', 'NotAllowedError');
                },
              },
            });
            localStorage.setItem('i18nextLng', 'en-US');
          },
          { platform, mode: scheme.mode, schemeId: scheme.id, qrFailure },
        );

        await login(page);
        await expect(page.locator('html')).toHaveAttribute('data-theme', scheme.mode);
        await expect(page.locator('html')).toHaveAttribute(
          platform === 'android' ? 'data-platform' : 'data-desktop-platform',
          platform,
        );
        await expect
          .poll(() =>
            page.evaluate(() =>
              getComputedStyle(document.documentElement).getPropertyValue('--bg-elevated').trim(),
            ),
          )
          .toBe(scheme.variables['--bg-elevated']);
        expect(await page.evaluate(() => matchMedia('(prefers-contrast: more)').matches)).toBe(
          contrast === 'more',
        );
        if (platform === 'android')
          await page.locator('.android-navigation a[href="/settings"]').click();
        else await page.getByRole('button', { name: 'Settings', exact: true }).first().click();
        await page.getByRole('main').getByText('Device Sync', { exact: true }).click();
        await expect(page).toHaveURL('/sync');
        const card = page.locator('[data-ui-card="floating"]');

        await page.getByRole('button', { name: 'Show QR', exact: true }).click();
        const status = card.getByText(qrFailure, { exact: false });
        await expect(status).toBeVisible();
        await status.scrollIntoViewIfNeeded();
        const statusColors = await measureControlContrast(status, '[data-ui-card="floating"]');
        expect
          .soft(statusColors.foregroundContrast, 'QR failure body contrast')
          .toBeGreaterThanOrEqual(4.5);
        await test.info().attach('qr-error-contrast', {
          body: JSON.stringify(statusColors),
          contentType: 'application/json',
        });
        await page.screenshot({ path: test.info().outputPath('qr-generation-error.png') });
        await card.locator('button[aria-label="Close"]').click();
        await expect(card).toHaveCount(0);

        await page.getByRole('button', { name: 'Scan QR', exact: true }).click();
        const denied = card.getByText(/^Camera access is blocked\./);
        await expect(denied).toBeVisible();
        await denied.scrollIntoViewIfNeeded();
        const deniedColors = await measureControlContrast(denied, '[data-ui-card="floating"]');
        expect
          .soft(deniedColors.foregroundContrast, 'Camera failure body contrast')
          .toBeGreaterThanOrEqual(4.5);
        const samples = [statusColors, deniedColors];
        for (const sample of samples) {
          if (contrast === 'more') {
            const primary = scheme.mode === 'dark' ? [255, 255, 255] : [17, 17, 17];
            expect
              .soft(sample.foreground, 'high contrast error body follows neutral foreground')
              .toEqual(primary);
          } else {
            expect
              .soft(sample.foreground[0], 'semantic error remains red')
              .toBeGreaterThan(sample.foreground[1]);
          }
        }
        await test.info().attach('camera-error-contrast', {
          body: JSON.stringify(deniedColors),
          contentType: 'application/json',
        });
        expect(
          await page.evaluate(
            () =>
              (window as unknown as { __RF1179_CAMERA_REQUESTS__: number })
                .__RF1179_CAMERA_REQUESTS__,
          ),
        ).toBeGreaterThan(0);
        await page.screenshot({ path: test.info().outputPath('camera-permission-error.png') });
        await card
          .getByRole('button', { name: 'Use discovery or manual input', exact: true })
          .click();
        await expect(card).toHaveCount(0);
        await expect(page).toHaveURL('/sync');
      });
    }
  }
}
