import { expect, test } from '@playwright/test';
import { setupTauriMock, login } from './fixtures/auth';

for (const platform of ['android', 'ios', 'macos', 'windows']) {
  test(`${platform}: protected trash values align with reveal controls`, async ({ page }) => {
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
        autoLockTimeoutMinutes: 0,
      };
      const item = {
        id: 'trash-layout',
        originalId: 'object-layout',
        itemType: 'object',
        name: 'Layout fixture',
        deletedAt: Date.now(),
      };
      Object.assign(window, {
        __MOCK_PLATFORM__: platform,
        __E2E_MOCKS__: {
          ui_get_preferences: () => prefs,
          user_data_get_preferences: () => prefs,
          vault_check_directory: () => true,
          ocr_get_model_status: () => ({ installed: true, bundled: true }),
          sync_list_conflicts: () => [],
          object_trash_list: () => [item],
          trash_get_detail: () => ({
            ...item,
            deletedBy: 'user',
            originalLocation: 'identity',
            previewProperties: ['public', 'internal', 'sensitive', 'critical'].map(
              (sensitivityLevel) => ({
                key: sensitivityLevel,
                value: 'Short test value',
                type: 'text',
                sensitivityLevel,
              }),
            ),
            attachments: [],
            deletedAttachments: [],
            snapshots: [],
            childItems: [],
          }),
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
    await page.getByText('Layout fixture', { exact: true }).click();
    for (const level of ['internal', 'sensitive', 'critical']) {
      const button = page.getByRole('button', { name: new RegExp(`^${level}:`) });
      await expect(button).toBeVisible();
      const geometry = await button.evaluate((control) => {
        const text = control.parentElement!.querySelector('[data-field-value-text]')!;
        const a = text.getBoundingClientRect();
        const b = control.getBoundingClientRect();
        return {
          offset: Math.abs(a.y + a.height / 2 - b.y - b.height / 2),
          height: b.height,
          basis: getComputedStyle(control.parentElement!).flexBasis,
        };
      });
      expect(geometry.offset).toBeLessThan(1);
      expect(geometry.basis).toBe('0%');
      if (platform === 'android') expect(geometry.height).toBeGreaterThanOrEqual(48);
    }
    await expect(page.getByRole('button', { name: /^public:/ })).toHaveCount(0);
    await page.getByRole('button', { name: /^sensitive:/ }).click();
    await expect(
      page.locator('[data-field-value-text]').filter({ hasText: 'Short test value' }),
    ).toHaveCount(2);
    await page.screenshot({ path: test.info().outputPath(`${platform}-trash-values.png`) });
  });
}
