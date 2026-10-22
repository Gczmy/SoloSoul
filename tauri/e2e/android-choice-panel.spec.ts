import { expect, test } from '@playwright/test';
import { mountForm, noOverflow, changes } from './fixtures/android-form';

for (const theme of ['light', 'dark']) {
  test(`choice panel ${theme}: long labels scroll independently, back cancels and focus returns`, async ({
    page,
  }) => {
    await page.setViewportSize({ width: 320, height: 568 });
    const choices = Array.from({ length: 30 }, (_, i) => ({
      value: String(i),
      label: `PublicLongOption${i}_` + 'ABCDEFGHIJKLMNOPQRSTUVWXYZ'.repeat(5),
    }));
    await mountForm(
      page,
      '/src/components/android/AndroidChoiceField.tsx',
      'AndroidChoiceField',
      { title: 'Public choices', choices, multiple: true, value: ['0'] },
      'android',
      theme,
    );
    const trigger = page.getByRole('button', { name: 'Public choices' });
    await trigger.click();
    await noOverflow(page, '.android-sheet, [data-android-choice-scroll], [role="checkbox"]');
    await page.getByRole('checkbox').nth(1).click();
    const scroll = page.locator('[data-android-choice-scroll]');
    await scroll.evaluate((el) => {
      el.scrollTop = el.scrollHeight;
    });
    await expect(page.getByRole('checkbox').last()).toBeInViewport();
    await expect(page.getByRole('button', { name: 'Confirm', exact: true })).toBeInViewport();
    await page.evaluate(() => history.back());
    await expect(page.getByRole('dialog')).toHaveCount(0);
    await expect(trigger).toBeFocused();
    expect(await changes(page)).toEqual([]);
    await trigger.click();
    await page.getByRole('checkbox').nth(1).click();
    await page.getByRole('button', { name: 'Confirm', exact: true }).click();
    expect(await changes(page)).toEqual([['0', '1']]);
  });
}
