import { expect, test } from '@playwright/test';
import { mountForm, noOverflow, changes } from './fixtures/android-form';
for (const theme of ['light', 'dark'])
  test(`Android ${theme}: date/time fits 320px and preserves invalid drafts`, async ({ page }) => {
    await page.setViewportSize({ width: 320, height: 568 });
    await mountForm(
      page,
      '/src/components/forms/DatePicker.tsx',
      'DatePicker',
      { value: '2024-02-30T12:30', includeTime: true },
      'android',
      theme,
    );
    const trigger = page.getByRole('button', { name: 'Select date and time', exact: true });
    await trigger.click();
    await expect(page.getByLabel('Day', { exact: true })).toHaveValue('30');
    await noOverflow(page, '.android-sheet, [data-android-date-scroll]');
    await expect(page.getByRole('gridcell')).toHaveCount(0);
    await page.getByLabel('Choose from calendar', { exact: true }).fill('2024-02-29');
    expect(await changes(page)).toEqual(['2024-02-29T12:30']);
    await page.getByLabel('Year', { exact: true }).fill('12');
    expect((await changes(page)).at(-1)).toBe('12-02-29T12:30');
    await page.evaluate(() => history.back());
    await expect(page.getByRole('dialog')).toHaveCount(0);
    await expect(trigger).toBeFocused();
    await expect(trigger).toContainText('12-02-29T12:30');
    await trigger.click();
    await page.getByRole('button', { name: 'Clear', exact: true }).click();
    await expect(trigger).not.toContainText('12-02-29');
  });
for (const platform of ['windows', 'macos'])
  test(`${platform}: desktop calendar remains inline`, async ({ page }) => {
    await mountForm(
      page,
      '/src/components/forms/DatePicker.tsx',
      'DatePicker',
      { value: '2024-01-01' },
      platform,
    );
    await page.getByLabel('年份输入').click();
    await expect(page.getByRole('gridcell').first()).toBeVisible();
    await expect(page.locator('.android-sheet')).toHaveCount(0);
  });
