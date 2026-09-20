import { expect, test } from '@playwright/test';
import { login, setupTauriMock } from './fixtures/auth';
import { measureControlContrast } from './fixtures/contrast';

for (const platform of ['android', 'ios', 'macos', 'windows']) {
  for (const theme of ['light', 'dark']) {
    test(`${platform} ${theme}: checkbox touch target, mark and keyboard selection`, async ({
      page,
    }) => {
      await page.setViewportSize({
        width: ['android', 'ios'].includes(platform) ? 390 : 1100,
        height: 844,
      });
      await setupTauriMock(page);
      await page.addInitScript(
        ({ platform, theme }) => {
          const prefs = {
            theme,
            accentColor: 'ocean',
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
                  id: 'check-1',
                  originalId: 'object-1',
                  itemType: 'object',
                  name: 'Keyboard fixture',
                  deletedAt: Date.now(),
                },
              ],
            },
          });
          localStorage.setItem('i18nextLng', 'en-US');
        },
        { platform, theme },
      );
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
      const size = await checkbox.boundingBox();
      const target = platform === 'android' ? 48 : platform === 'ios' ? 44 : 14;
      expect(size!.height).toBeGreaterThanOrEqual(target);
      expect(size!.width).toBeGreaterThanOrEqual(target);
      // 点按触控区边缘也能选择，图形不需要占满整个目标。
      await checkbox.click({ position: { x: 2, y: 2 } });
      await expect(checkbox).toBeChecked();
      await expect(page.getByText('Content preview', { exact: true })).toHaveCount(0);
      const visual = card.locator('[data-checkbox-visual]');
      const expectedVisual = ['android', 'ios'].includes(platform) ? 20 : 14;
      expect((await visual.boundingBox())!.width).toBe(expectedVisual);
      await expect
        .poll(async () => (await measureControlContrast(visual, '[data-ui-card]')).iconContrast)
        .toBeGreaterThanOrEqual(4.5);
      await expect(visual).not.toHaveCSS('background-color', 'rgba(0, 0, 0, 0)');
      await checkbox.focus();
      await checkbox.press('Space');
      await expect(checkbox).not.toBeChecked();
      await checkbox.press('Space');
      await expect(checkbox).toBeChecked();
      await expect(page.getByText('Content preview', { exact: true })).toHaveCount(0);
      await expect(visual).toHaveCSS('outline-style', 'solid');
      await page.screenshot({ path: test.info().outputPath(`${platform}-${theme}-checkbox.png`) });
    });
  }
}
