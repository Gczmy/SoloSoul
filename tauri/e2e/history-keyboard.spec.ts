import { expect, test } from '@playwright/test';
import { login, setupTauriMock } from './fixtures/auth';

test('隐藏历史字段支持 Enter 和 Space，揭示前无原文 DOM', async ({ page }) => {
  await setupTauriMock(page);
  await page.addInitScript(() => {
    Object.assign(window, {
      __E2E_MOCKS__: {
        vault_check_directory: () => true,
        snapshot_list: () => [
          {
            id: 'keys',
            timestamp: Date.now(),
            triggeredBy: 'user_edit',
            diffSummary: 'diff_updated',
          },
        ],
        snapshot_get_data: () => ({
          properties: { enter: 'ENTER_SECRET', space: 'SPACE_SECRET' },
          propertyLabels: { enter: 'sensitive', space: 'sensitive' },
        }),
      },
    });
  });
  await login(page);
  await page.evaluate(async () => {
    const path = '/e2e/fixtures/historyKeyboardHarness.tsx';
    const fixture = await import(/* @vite-ignore */ path);
    fixture.mount();
  });
  const fixture = page.locator('#history-keyboard-fixture');
  const buttons = fixture.getByRole('button').filter({ hasText: '••••••••' });
  await expect(buttons).toHaveCount(2);
  const fieldRow = fixture.locator('[data-field-presentation-row]').first();
  await expect(fieldRow.locator('[data-field-value]')).toHaveAttribute(
    'data-value-layout',
    'inline',
  );
  const centerOffset = await fieldRow.evaluate((row) => {
    const label = row.querySelector('[data-field-label-slot]')!.getBoundingClientRect();
    const value = row.querySelector('[data-field-value]')!.getBoundingClientRect();
    return Math.abs(label.y + label.height / 2 - value.y - value.height / 2);
  });
  expect(centerOffset).toBeLessThan(1);
  expect(await fixture.innerHTML()).not.toContain('ENTER_SECRET');
  expect(await fixture.innerHTML()).not.toContain('SPACE_SECRET');
  await buttons.first().focus();
  await page.keyboard.press('Enter');
  await expect(fixture.getByText('ENTER_SECRET', { exact: true })).toBeVisible();
  await buttons.first().focus();
  await page.keyboard.press('Space');
  await expect(fixture.getByText('SPACE_SECRET', { exact: true })).toBeVisible();
});
