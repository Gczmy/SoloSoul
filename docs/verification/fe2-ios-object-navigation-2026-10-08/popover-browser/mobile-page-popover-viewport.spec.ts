import { expect, test } from '@playwright/test';
import { login, setupTauriMock } from './fixtures/auth';

test('iOS add-page actions stay above the visual viewport bottom during keyboard resize', async ({
  page,
}) => {
  await page.setViewportSize({ width: 390, height: 640 });
  await setupTauriMock(page);
  await page.addInitScript(() => {
    const prefs = {
      theme: 'light',
      language: 'en-US',
      hasSeenOnboarding: true,
      autoLockTimeoutMinutes: 0,
      backupReminderDays: 0,
    };
    const viewport = Object.assign(new EventTarget(), {
      width: innerWidth,
      height: innerHeight,
      offsetTop: 0,
    });
    Object.defineProperty(window, 'visualViewport', { value: viewport, configurable: true });
    Object.assign(window, {
      __MOCK_PLATFORM__: 'ios',
      __resizeTestViewport: (height: number) => {
        viewport.height = height;
        viewport.dispatchEvent(new Event('resize'));
      },
      __E2E_MOCKS__: {
        ui_get_preferences: () => prefs,
        user_data_get_preferences: () => prefs,
        vault_check_directory: () => true,
      },
    });
    localStorage.setItem('i18nextLng', 'en-US');
  });
  await login(page);
  await page.getByRole('button', { name: 'Add Page', exact: true }).click();
  const card = page.locator('[data-add-page-popover]');
  await expect(card).toBeVisible();
  await card.evaluate(async (node) => {
    await Promise.all(node.getAnimations().map((animation) => animation.finished));
  });
  await page.evaluate(() => {
    (
      window as typeof window & { __resizeTestViewport: (height: number) => void }
    ).__resizeTestViewport(350);
  });
  await expect
    .poll(async () => (await card.boundingBox())!.y + (await card.boundingBox())!.height)
    .toBeLessThanOrEqual(342);
  for (const name of ['Cancel', 'Confirm']) {
    const button = card.getByRole('button', { name, exact: true });
    const rect = (await button.boundingBox())!;
    expect(rect.y + rect.height).toBeLessThanOrEqual(350);
    expect(
      await button.evaluate((node) => {
        const r = node.getBoundingClientRect();
        const hit = document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2);
        return node === hit || node.contains(hit);
      }),
    ).toBe(true);
  }
  await page.screenshot({ path: test.info().outputPath('visual-viewport-popover.png') });
  await card.getByRole('button', { name: 'Cancel', exact: true }).click();
  await expect(card).toHaveCount(0);
});
