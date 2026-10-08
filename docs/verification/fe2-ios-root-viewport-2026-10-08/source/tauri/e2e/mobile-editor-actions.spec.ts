import { expect, test } from '@playwright/test';
import { login, setupTauriMock } from './fixtures/auth';

for (const platform of ['ios', 'android'] as const) {
  for (const width of [320, 390]) {
    test(`${platform} ${width}px: editor save and cancel remain above navigation`, async ({
      page,
    }) => {
      await page.setViewportSize({ width, height: 640 });
      await setupTauriMock(page);
      await page.addInitScript((mockPlatform) => {
        if (mockPlatform === 'ios') {
          const viewport = Object.assign(new EventTarget(), { height: 640, offsetTop: 0 });
          Object.defineProperty(window, 'visualViewport', { configurable: true, value: viewport });
        }
        const prefs = {
          theme: 'light',
          language: 'en-US',
          hasSeenOnboarding: true,
          autoLockTimeoutMinutes: 0,
          backupReminderDays: 0,
        };
        Object.assign(window, {
          __MOCK_PLATFORM__: mockPlatform,
          __E2E_MOCKS__: {
            ui_get_preferences: () => prefs,
            user_data_get_preferences: () => prefs,
            vault_check_directory: () => true,
            template_list: () => [
              {
                id: 'public-identity',
                accountId: 'e2e-account',
                name: 'Public identity',
                category: 'identity',
                createdAt: '2026-10-08T00:00:00Z',
                properties: Array.from({ length: 8 }, (_, i) => ({
                  id: `field-${i}`,
                  name: `Public field ${i}`,
                  type: 'text',
                  sensitivityLevel: 'public',
                })),
              },
            ],
            object_field_suggestions: () => [],
          },
        });
        localStorage.setItem('i18nextLng', 'en-US');
      }, platform);
      await login(page);
      await page
        .getByRole('button', { name: /^Identity/ })
        .first()
        .click();
      if (platform === 'android') {
        await page.locator('.android-fab').click();
        await page.getByRole('button', { name: /^New object/ }).click();
      } else {
        await page.getByRole('button', { name: '+', exact: true }).click();
      }
      await expect(page).toHaveURL('/editor?section=identity');
      await expect(page.getByLabel('Object Name', { exact: true })).toBeVisible();
      await page.evaluate(() => {
        document.documentElement.style.fontSize = '20px';
      });
      if (platform === 'ios') {
        await page.getByLabel('Object Name', { exact: true }).fill('Keyboard regression');
        await page.evaluate(() => {
          const viewport = window.visualViewport!;
          Object.defineProperty(viewport, 'height', { configurable: true, value: 350 });
          viewport.dispatchEvent(new Event('resize'));
        });
      }
      const cancel = page.getByRole('button', { name: 'Cancel', exact: true });
      const save = page.getByRole('button', { name: 'Save', exact: true });
      await cancel.scrollIntoViewIfNeeded();
      const nav = page.locator(
        platform === 'ios' ? '[data-testid="mobile-bottom-nav"]' : '.android-navigation',
      );
      const navRect = (await nav.boundingBox())!;
      if (platform === 'ios') {
        const header = (await page.getByRole('banner').boundingBox())!;
        expect(header.y).toBe(0);
        expect(navRect.y + navRect.height).toBeLessThanOrEqual(350);
        expect(await page.evaluate(() => window.scrollY)).toBe(0);
      }
      await page.screenshot({ path: test.info().outputPath('editor-actions.png') });
      for (const button of [save, cancel]) {
        const rect = (await button.boundingBox())!;
        expect(rect.height).toBeGreaterThanOrEqual(44);
        expect(rect.y).toBeGreaterThanOrEqual(0);
        expect(rect.y + rect.height).toBeLessThanOrEqual(navRect.y);
        expect(
          await button.evaluate((node) => {
            const r = node.getBoundingClientRect();
            const hit = document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2);
            return node === hit || node.contains(hit);
          }),
        ).toBe(true);
      }
      await cancel.click();
      await expect(page).toHaveURL('/workspace?section=identity');
    });
  }
}
