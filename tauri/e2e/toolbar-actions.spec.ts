import { expect, test, type Page } from '@playwright/test';
import { login, setupTauriMock } from './fixtures/auth';

async function setup(page: Page, platform: string, width: number) {
  await page.setViewportSize({ width, height: 800 });
  await setupTauriMock(page);
  await page.addInitScript((platform) => {
    const preferences = {
      language: 'en-US',
      hasSeenOnboarding: true,
      autoLockTimeoutMinutes: 0,
      backupReminderDays: 0,
    };
    Object.assign(window, {
      __MOCK_PLATFORM__: platform,
      __E2E_MOCKS__: {
        ui_get_preferences: () => preferences,
        user_data_get_preferences: () => preferences,
        vault_check_directory: () => true,
        ocr_get_model_status: () => ({ installed: true, bundled: true }),
        sync_list_conflicts: () => [],
      },
    });
    localStorage.setItem('i18nextLng', 'en-US');
  }, platform);
  await login(page);
}

for (const platform of ['android', 'windows', 'macos']) {
  test.describe(platform, () => {
    test.use({ isMobile: platform === 'android', hasTouch: platform === 'android' });
    for (const width of platform === 'android' ? [320, 390] : [800, 1100]) {
      test(`${width}px: single Guide stays direct through sidebar changes`, async ({ page }) => {
        await setup(page, platform, width);
        const header = page.locator('[data-appbar]');
        const guide = header.getByRole('button', { name: 'Guide', exact: true });
        const more = header.getByRole('button', { name: 'More actions', exact: true });
        const states = platform === 'android' ? [true] : [true, false, true];
        for (const expanded of states) {
          if (platform !== 'android') {
            const toggle = header.locator('button[aria-controls="desktop-page-pane"]');
            if ((await toggle.getAttribute('aria-expanded')) !== String(expanded))
              await toggle.click();
            await expect(toggle).toHaveAttribute('aria-expanded', String(expanded));
          }
          await expect(more).toHaveCount(0);
          await expect(guide).toBeVisible();
          await expect(guide).toHaveAttribute('title', 'Guide');
          await expect(guide).toHaveAttribute('data-ui-icon-intent', 'neutral');
          const box = (await guide.boundingBox())!;
          expect(box.x).toBeGreaterThanOrEqual(0);
          expect(box.x + box.width).toBeLessThanOrEqual(width);
          expect(box.height).toBeGreaterThanOrEqual(platform === 'android' ? 44 : 32);
          await guide.focus();
          await page.keyboard.press('Enter');
          await expect(page.locator('[data-page-guide-overlay] [role="dialog"]')).toBeVisible();
          await page.keyboard.press('Escape');
          await expect(page.locator('[data-page-guide-overlay]')).toHaveCount(0);
          await expect(guide).toBeFocused();
        }
        await page.screenshot({
          path: test.info().outputPath(`${platform}-${width}-direct-guide.png`),
        });
        if (platform === 'android') {
          await expect(
            header.getByRole('button', { name: 'Lock Vault', exact: true }),
          ).toBeVisible();
        } else if (platform === 'windows' && width === 800) {
          await page
            .locator('#desktop-navigation')
            .getByRole('button', { name: 'Identity', exact: true })
            .click();
          await expect(guide).toBeVisible();
          await expect(more).toHaveCount(0);
          const create = header.getByRole('button', { name: '+ New', exact: true });
          await expect(create).toBeInViewport();
          await expect(create).toHaveCSS('font-size', '13px');
          const createBox = (await create.boundingBox())!;
          const headerBox = (await header.boundingBox())!;
          expect(createBox.y + createBox.height).toBeLessThanOrEqual(
            headerBox.y + headerBox.height,
          );
          await create.click();
          await expect(page).toHaveURL(/\/editor\?section=identity/);
        }
        expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBe(width);
      });
    }
    if (platform !== 'android') {
      test('multiple actions retain the keyboard accessible overflow menu', async ({ page }) => {
        await setup(page, platform, 800);
        await page.evaluate(() => {
          history.pushState(null, '', '/llm-chat');
          dispatchEvent(new PopStateEvent('popstate'));
        });
        const header = page.locator('[data-appbar]');
        const more = header.getByRole('button', { name: 'More actions', exact: true });
        const menu = header.locator('#toolbar-secondary-actions');
        const guide = menu.getByRole('button', { name: 'Guide', exact: true });
        await expect(more).toBeVisible();
        await expect(guide).toBeHidden();
        await more.focus();
        await page.keyboard.press('Enter');
        await expect(menu.getByRole('button').first()).toBeFocused();
        await guide.focus();
        await page.keyboard.press('Enter');
        await expect(page.locator('[data-page-guide-overlay] [role="dialog"]')).toBeVisible();
        await page.keyboard.press('Escape');
        await guide.focus();
        await page.keyboard.press('Escape');
        await expect(more).toBeFocused();
        await expect(guide).toBeHidden();
      });
    }
  });
}
