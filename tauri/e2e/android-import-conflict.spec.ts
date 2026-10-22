import { expect, test } from '@playwright/test';
import { mountForm, noOverflow, changes } from './fixtures/android-form';
for (const theme of ['light', 'dark'])
  test(`Android ${theme}: full conflict names and distinct strategies`, async ({ page }) => {
    await page.setViewportSize({ width: 320, height: 568 });
    const long = 'PublicImported_ABCDEFGHIJKLMNOPQRSTUVWXYZ'.repeat(5),
      local = 'PublicLocal_ABCDEFGHIJKLMNOPQRSTUVWXYZ'.repeat(5);
    await mountForm(
      page,
      '/src/components/import/AndroidImportConflict.tsx',
      'AndroidImportConflict',
      {
        importedName: long,
        existingName: local,
        kindLabel: 'Local name changed',
        value: 'skipExisting',
      },
      'android',
      theme,
    );
    await noOverflow(page, '[data-form-fixture], [data-form-fixture-ready]');
    await expect(page.locator('p').filter({ hasText: long })).toBeVisible();
    await expect(page.locator('p').filter({ hasText: local })).toBeVisible();
    const trigger = page.getByRole('button', { name: 'Import Strategy', exact: true });
    await trigger.click();
    await page.getByRole('radio', { name: /Overwrite.*Replace all existing data/ }).click();
    expect(await changes(page)).toEqual(['overwrite']);
    await trigger.click();
    await page.getByRole('radio', { name: /Keep both.*Import as a new object/ }).click();
    expect(await changes(page)).toEqual(['overwrite', 'keepBoth']);
  });
