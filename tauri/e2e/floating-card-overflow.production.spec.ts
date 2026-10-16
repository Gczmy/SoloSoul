import { expect, test, type Locator, type Page } from '@playwright/test';
import { login, setupTauriMock } from './fixtures/auth';

type Platform = 'android' | 'windows' | 'macos';

async function prepare(page: Page, platform: Platform, theme: 'light' | 'dark') {
  await setupTauriMock(page);
  await page.addInitScript(
    ({ platform, theme }) => {
      const prefs = {
        theme,
        language: 'en-US',
        hasSeenOnboarding: true,
        autoLockTimeoutMinutes: 0,
      };
      Object.assign(window, {
        __MOCK_PLATFORM__: platform,
        __E2E_MOCKS__: {
          ui_get_preferences: () => prefs,
          user_data_get_preferences: () => prefs,
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
          sync_generate_qr_payload: () =>
            JSON.stringify({ a: '127.0.0.1:42069', f: '0'.repeat(64), n: 'Public test' }),
        },
      });
      Object.defineProperty(navigator, 'mediaDevices', {
        configurable: true,
        value: { enumerateDevices: async () => [] },
      });
      localStorage.setItem('i18nextLng', 'en-US');
    },
    { platform, theme },
  );
}

async function expectFrameWithinViewport(card: Locator) {
  const geometry = await card.evaluate((node) => {
    const rect = node.getBoundingClientRect();
    return {
      top: rect.top,
      bottom: rect.bottom,
      left: rect.left,
      right: rect.right,
      viewportWidth: innerWidth,
      viewportHeight: innerHeight,
      horizontalOverflow: node.scrollWidth - node.clientWidth,
    };
  });
  expect(geometry.top).toBeGreaterThanOrEqual(0);
  expect(geometry.bottom).toBeLessThanOrEqual(geometry.viewportHeight);
  expect(geometry.left).toBeGreaterThanOrEqual(0);
  expect(geometry.right).toBeLessThanOrEqual(geometry.viewportWidth);
  expect(geometry.horizontalOverflow).toBeLessThanOrEqual(1);
}

async function expectMode(page: Page, platform: Platform, theme: 'light' | 'dark') {
  await expect(page.locator('html')).toHaveAttribute('data-theme', theme);
  await expect(page.locator('html')).toHaveAttribute(
    platform === 'android' ? 'data-platform' : 'data-desktop-platform',
    platform,
  );
}

async function reach(card: Locator, control: Locator) {
  await control.scrollIntoViewIfNeeded();
  await expect(control).toBeInViewport({ ratio: 1 });
  await control.click({ trial: true });
  await expectFrameWithinViewport(card);
}

for (const platform of ['android', 'windows', 'macos'] as const) {
  for (const theme of ['light', 'dark'] as const) {
    const viewports = [
      { name: 'compact', width: platform === 'android' ? 320 : 900, height: 480 },
      { name: 'shallow', width: platform === 'android' ? 640 : 1280, height: 240 },
      { name: 'roomy', width: 1280, height: 900 },
    ];
    for (const viewport of viewports) {
      test(`${platform} ${theme} ${viewport.name} recovery form keeps both ends reachable`, async ({
        page,
      }) => {
        await page.setViewportSize(viewport);
        await prepare(page, platform, theme);
        await page.goto('/login');
        await page.locator('input[type=password]').waitFor();
        await expectMode(page, platform, theme);
        await page.getByRole('button', { name: /Restore from another device/ }).click();
        const card = page.locator('[data-ui-card="floating"]');
        await card.getByRole('button', { name: 'Manual', exact: true }).first().click();
        await card.getByRole('button', { name: 'Show optional fingerprint', exact: true }).click();
        await expectFrameWithinViewport(card);
        const fingerprint = card.getByPlaceholder(/abc123/);
        await fingerprint.fill('0'.repeat(64));
        await reach(card, card.getByRole('button', { name: 'Next', exact: true }));
        await expect(fingerprint).toHaveValue('0'.repeat(64));
        const close = card.locator('button[aria-label="Close"]');
        await reach(card, close);
        await page.screenshot({ path: test.info().outputPath('recovery-scrollable.png') });
        await close.click();
        await expect(card).toHaveCount(0);
        await expect(page.locator('input[type=password]')).toBeVisible();
      });

      test(`${platform} ${theme} ${viewport.name} QR scan and display keep dismissal reachable`, async ({
        page,
      }) => {
        await page.setViewportSize(viewport);
        await prepare(page, platform, theme);
        await login(page);
        await expectMode(page, platform, theme);
        if (platform === 'android') {
          await page.locator('.android-navigation a[href="/settings"]').click();
        } else {
          await page.getByRole('button', { name: 'Settings', exact: true }).first().click();
        }
        await page.getByRole('main').getByText('Device Sync', { exact: true }).click();
        await expect(page).toHaveURL('/sync');
        await page.getByRole('button', { name: 'Scan QR', exact: true }).click();
        const card = page.locator('[data-ui-card="floating"]');
        await expect(card).toBeVisible();
        await expectFrameWithinViewport(card);
        await reach(card, card.getByRole('button').last());
        const close = card.locator('button[aria-label="Close"]');
        await reach(card, close);
        await page.screenshot({ path: test.info().outputPath('qr-scan-scrollable.png') });
        await close.click();
        await expect(card).toHaveCount(0);

        await page.getByRole('button', { name: 'Show QR', exact: true }).click();
        await expect(card.locator('svg[height="200"]')).toBeVisible();
        await expectFrameWithinViewport(card);
        const done = card.getByRole('button', { name: 'Close', exact: true }).last();
        await reach(card, done);
        await page.screenshot({ path: test.info().outputPath('qr-display-scrollable.png') });
        await done.click();
        await expect(card).toHaveCount(0);
        await expect(page).toHaveURL('/sync');
      });
    }
  }
}
