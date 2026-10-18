import { expect, test } from '@playwright/test';
import { login, setupTauriMock } from './fixtures/auth';
import { measureControlContrast } from './fixtures/contrast';

// 浏览器仅验证原生布尔标记的样式契约；真实系统交付由 Android 原生队列验收。
for (const platform of ['android', 'windows', 'macos'] as const) {
  for (const mode of ['light', 'dark'] as const) {
    test(`${platform} ${mode} Android contrast flag preserves platform scope and restores colors`, async ({
      page,
    }) => {
      await page.setViewportSize({ width: platform === 'android' ? 390 : 1000, height: 844 });
      await page.emulateMedia({ contrast: 'no-preference' });
      await setupTauriMock(page);
      await page.addInitScript(
        ({ platform, mode }) => {
          const prefs = {
            theme: mode,
            defaultLightTheme: 'warm-stone',
            defaultDarkTheme: 'warm-stone-dark',
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
          localStorage.setItem('i18nextLng', 'en-US');
        },
        { platform, mode },
      );
      await login(page);
      const root = page.locator('html');
      await expect(root).toHaveAttribute('data-theme', mode);
      await expect(root).toHaveAttribute(
        platform === 'android' ? 'data-platform' : 'data-desktop-platform',
        platform,
      );
      if (platform === 'android')
        await page.locator('.android-navigation a[href="/settings"]').click();
      else await page.getByRole('button', { name: 'Settings', exact: true }).first().click();
      await page.getByRole('main').getByText('Device Sync', { exact: true }).click();
      await page.getByRole('button', { name: 'Show QR', exact: true }).click();
      const card = page.locator('[data-ui-card="floating"]');
      const error = card.getByText('Synchronization is disabled. Enable it first.', {
        exact: false,
      });
      await expect(error).toBeVisible();
      const normal = await measureControlContrast(error, '[data-ui-card="floating"]');
      for (const enabled of [false, true, false]) {
        await root.evaluate((element, active) => {
          element.dataset.androidHighTextContrast = String(active);
          // 桌面窗框 IPC 在 Android 返回 false，不得盖掉独立的原生标记。
          element.dataset.highContrast = 'false';
        }, enabled);
        const colors = await measureControlContrast(error, '[data-ui-card="floating"]');
        expect(colors.foregroundContrast).toBeGreaterThanOrEqual(4.5);
        expect(colors.foreground).toEqual(
          platform === 'android' && enabled
            ? mode === 'dark'
              ? [255, 255, 255]
              : [17, 17, 17]
            : normal.foreground,
        );
      }
      await card.locator('button[aria-label="Close"]').click();
      await expect(card).toHaveCount(0);
    });
  }
}
