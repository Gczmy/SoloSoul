import { expect, test } from '@playwright/test';
import { setupTauriMock } from './fixtures/auth';

for (const platform of ['android', 'windows', 'macos'] as const) {
  for (const theme of ['light', 'dark'] as const) {
    test(`${platform} ${theme} recovery labels name and focus their own inputs`, async ({
      page,
    }) => {
      await page.setViewportSize(
        platform === 'android' ? { width: 390, height: 844 } : { width: 1280, height: 720 },
      );
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
      await page.goto('/login');
      await page.locator('input[type=password]').waitFor();
      await expect(page.locator('html')).toHaveAttribute('data-theme', theme);
      await expect(page.locator('html')).toHaveAttribute(
        platform === 'android' ? 'data-platform' : 'data-desktop-platform',
        platform,
      );
      await page.getByRole('button', { name: 'Restore from another device', exact: true }).click();
      const card = page.locator('[data-ui-card="floating"]');
      await card.getByRole('button', { name: 'Manual', exact: true }).first().click();
      await card.getByRole('button', { name: 'Show optional fingerprint', exact: true }).click();
      await expect(
        card.getByRole('textbox', { name: 'Host fingerprint', exact: true }),
      ).toHaveCount(1);

      const fields = [
        { name: 'Host Address', value: '127.0.0.1:42069' },
        { name: 'PIN', value: '654321' },
        { name: 'Host fingerprint', value: '0'.repeat(64) },
      ];
      const ids: string[] = [];
      for (const field of fields) {
        const input = card.getByRole('textbox', { name: field.name, exact: true });
        await card.getByText(field.name, { exact: true }).click();
        await expect(input).toBeFocused();
        await input.fill(field.value);
        const id = await input.getAttribute('id');
        expect(id).toBeTruthy();
        ids.push(id!);
      }
      expect(new Set(ids).size).toBe(fields.length);
      for (const field of fields) {
        await expect(card.getByRole('textbox', { name: field.name, exact: true })).toHaveValue(
          field.value,
        );
      }
      await page.screenshot({ path: test.info().outputPath('input-label-focus.png') });
      await card.locator('button[aria-label="Close"]').click();
      await expect(card).toHaveCount(0);
      await expect(page.locator('input[type=password]')).toBeVisible();
    });
  }
}
