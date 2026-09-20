import { expect, test } from '@playwright/test';
import { login, setupTauriMock } from './fixtures/auth';
import { measureControlContrast } from './fixtures/contrast';

for (const platform of ['android', 'ios', 'macos', 'windows']) {
  for (const theme of ['light', 'dark']) {
    test(`${platform} ${theme}: document format choices follow platform controls`, async ({
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
            language: 'en-US',
            accentColor: 'ocean',
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
              export_get_scope_tree: () => [],
              cloud_targets_detect: () => [],
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
          '/settings/export-import',
        );
        dispatchEvent(new PopStateEvent('popstate', { state: history.state }));
      });
      await page.getByRole('button', { name: 'Export as Document', exact: true }).click();
      const choices = page.locator('[data-ui-choice]');
      await expect(choices).toHaveCount(5);
      await expect(page.getByRole('button', { name: 'Word (.docx)', exact: true })).toHaveAttribute(
        'aria-pressed',
        'true',
      );
      for (const label of [
        'PDF (.pdf)',
        'HTML (.html)',
        'Plain Text (.txt)',
        'Markdown (.md)',
        'Word (.docx)',
      ]) {
        const choice = page.getByRole('button', { name: label, exact: true });
        await choice.click();
        await expect(choice).toHaveAttribute('aria-pressed', 'true');
        await expect(page.locator('[data-ui-choice][aria-pressed="true"]')).toHaveCount(1);
        await page.mouse.move(0, 0);
        await expect
          .poll(
            async () => (await measureControlContrast(choice, '[data-ui-card]')).foregroundContrast,
          )
          .toBeGreaterThanOrEqual(4.5);
      }
      const geometry = await choices.evaluateAll((buttons) =>
        buttons.map((button) => {
          const rect = button.getBoundingClientRect();
          const style = getComputedStyle(button);
          return {
            width: rect.width,
            height: rect.height,
            radius: parseFloat(style.borderRadius),
            right: rect.right,
            left: rect.left,
          };
        }),
      );
      for (const button of geometry) {
        expect(button.left).toBeGreaterThanOrEqual(0);
        expect(button.right).toBeLessThanOrEqual(page.viewportSize()!.width);
        if (platform === 'android') {
          expect(button.height).toBeGreaterThanOrEqual(48);
          expect(button.radius).toBe(24);
        }
      }
      await expect(page.getByRole('button', { name: 'Browse', exact: true })).toHaveAttribute(
        'data-ui-button',
        'secondary',
      );
      await expect(
        page.getByRole('button', { name: 'Export as document (0)', exact: true }),
      ).toBeDisabled();
      await page.screenshot({
        path: test.info().outputPath(`${platform}-${theme}-export.png`),
        fullPage: true,
      });
    });
  }
}
