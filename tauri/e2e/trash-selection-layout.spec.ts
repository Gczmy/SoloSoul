import { expect, test } from '@playwright/test';
import { login, setupTauriMock } from './fixtures/auth';

for (const platform of ['android', 'ios', 'macos', 'windows']) {
  test(`${platform}: trash checkbox and icon share the same row center`, async ({ page }) => {
    await page.setViewportSize({
      width: ['android', 'ios'].includes(platform) ? 390 : 1100,
      height: 844,
    });
    await setupTauriMock(page);
    await page.addInitScript((platform) => {
      const prefs = {
        theme: 'dark',
        language: 'en-US',
        hasSeenOnboarding: true,
        reduceMotion: true,
        autoLockTimeoutMinutes: 0,
      };
      Object.assign(window, {
        __MOCK_PLATFORM__: platform,
        __E2E_MOCKS__: {
          ui_get_preferences: () => prefs,
          user_data_get_preferences: () => prefs,
          vault_check_directory: () => true,
          ocr_get_model_status: () => ({ installed: true, bundled: true }),
          sync_list_conflicts: () => [],
          object_trash_list: () => [
            {
              id: 'trash-selection',
              originalId: 'object-selection',
              itemType: 'object',
              name: 'A long object name for selection layout',
              deletedAt: Date.now(),
              expiresAt: Date.now() + 86400000 * 10,
            },
          ],
        },
      });
      localStorage.setItem('i18nextLng', 'en-US');
    }, platform);
    await login(page);
    await page.evaluate(() => {
      history.pushState(
        { ...history.state, idx: (history.state?.idx ?? 0) + 1 },
        '',
        '/settings/trash',
      );
      dispatchEvent(new PopStateEvent('popstate', { state: history.state }));
    });
    const card = page.locator('.trash-item-card');
    const checkbox = card.getByRole('checkbox');
    await expect(checkbox).toBeVisible();
    const geometry = await card.evaluate((element) => {
      const box = element
        .querySelector('[role="checkbox"],input[type="checkbox"]')!
        .getBoundingClientRect();
      const icon = element.querySelector('[data-trash-item-icon]')!.getBoundingClientRect();
      return { offset: Math.abs(box.y + box.height / 2 - icon.y - icon.height / 2) };
    });
    expect(geometry.offset).toBeLessThan(1);
    await checkbox.click();
    await expect(checkbox).toBeChecked();
    await expect(page.getByText('Content preview', { exact: true })).toHaveCount(0);
    await page.screenshot({ path: test.info().outputPath(`${platform}-trash-selection.png`) });
  });
}
