import { expect, test } from '@playwright/test';
import { login, setupTauriMock } from './fixtures/auth';
import { noOverflow } from './fixtures/android-form';

for (const platform of ['android', 'windows', 'macos'])
  test(`${platform}: backup names fit and reminder writes its selected value`, async ({ page }) => {
    await page.setViewportSize({ width: platform === 'android' ? 320 : 1100, height: 700 });
    await setupTauriMock(page);
    const long = 'PublicBackup_ABCDEFGHIJKLMNOPQRSTUVWXYZ'.repeat(6);
    await page.addInitScript(
      ({ platform, long }) => {
        const prefs = {
          theme: 'light',
          language: 'en-US',
          hasSeenOnboarding: true,
          autoLockTimeoutMinutes: 0,
          reduceMotion: true,
          backupReminderDays: 0,
        };
        const writes: unknown[] = [];
        Object.assign(window, {
          __MOCK_PLATFORM__: platform,
          __BACKUP_WRITES__: writes,
          __E2E_MOCKS__: {
            ui_get_preferences: () => prefs,
            user_data_get_preferences: () => prefs,
            vault_check_directory: () => true,
            template_list: () => [],
            object_list: () => [],
            sync_list_conflicts: () => [],
            backup_list: () => [
              {
                id: 'public',
                name: long,
                created_at: '2026-10-22T10:00:00Z',
                size_bytes: 12345,
                object_count: 4,
              },
            ],
            user_data_update_preference: (args: unknown) => {
              writes.push(args);
            },
            ui_update_preference: () => {},
          },
        });
        localStorage.setItem('language', 'en-US');
      },
      { platform, long },
    );
    await login(page);
    await page.evaluate(() => {
      history.pushState({ idx: (history.state?.idx ?? 0) + 1 }, '', '/settings/backup');
      dispatchEvent(new PopStateEvent('popstate', { state: history.state }));
    });
    await expect(page.getByText(long, { exact: true })).toBeVisible();
    if (platform === 'android') {
      await noOverflow(page, '[data-ui-card]');
      await page.getByRole('button', { name: 'Reminder Frequency', exact: true }).click();
      await page.getByRole('radio', { name: '3 days', exact: true }).click();
    } else await page.locator('select').selectOption('3');
    await expect
      .poll(() =>
        page.evaluate(
          () => (window as unknown as { __BACKUP_WRITES__: unknown[] }).__BACKUP_WRITES__.length,
        ),
      )
      .toBe(1);
    const writes = await page.evaluate(
      () => (window as unknown as { __BACKUP_WRITES__: unknown[] }).__BACKUP_WRITES__,
    );
    expect(writes[0]).toMatchObject({ payload: { preferences: { backupReminderDays: 3 } } });
  });
