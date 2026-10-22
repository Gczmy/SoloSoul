import { expect, test } from '@playwright/test';
import { login, setupTauriMock } from './fixtures/auth';

for (const theme of ['light', 'dark'])
  test(`Android ${theme}: create menu rows use the same surfaces and text colors`, async ({
    page,
  }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await setupTauriMock(page);
    await page.addInitScript((theme) => {
      const prefs = {
        theme,
        language: 'en-US',
        hasSeenOnboarding: true,
        autoLockTimeoutMinutes: 0,
        reduceMotion: true,
        backupReminderDays: 0,
        androidGlass: 'frosted',
      };
      Object.assign(window, {
        __MOCK_PLATFORM__: 'android',
        __E2E_MOCKS__: {
          ui_get_preferences: () => prefs,
          user_data_get_preferences: () => prefs,
          vault_check_directory: () => true,
          object_list: () => [],
          template_list: () => [],
          sync_list_conflicts: () => [],
        },
      });
      localStorage.setItem('language', 'en-US');
    }, theme);
    await login(page);
    await page.locator('.android-fab').click();
    const rows = page.locator('.android-create-options > button');
    await expect(rows).toHaveCount(3);
    const colors = await rows.evaluateAll((nodes) =>
      nodes.map((node) => ({
        background: getComputedStyle(node).backgroundColor,
        color: getComputedStyle(node).color,
      })),
    );
    expect(colors[0]).toEqual(colors[1]);
    expect(colors[0]).toEqual(colors[2]);
    await rows.first().click();
    await expect(page).toHaveURL(/\/editor/);
  });
