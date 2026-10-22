import { expect, test } from '@playwright/test';
import { mountForm, noOverflow, changes } from './fixtures/android-form';
const long = 'PublicLongField_ABCDEFGHIJKLMNOPQRSTUVWXYZ'.repeat(5);
for (const theme of ['light', 'dark']) {
  for (const type of ['select', 'multiselect'])
    test(`Android ${theme} ${type}: long fields fit and selections commit`, async ({ page }) => {
      await page.setViewportSize({ width: 320, height: 568 });
      await mountForm(
        page,
        '/src/components/TemplateFieldInput.tsx',
        'TemplateFieldInput',
        {
          propertyId: 'public-field',
          label: long,
          type,
          value: type === 'select' ? '' : [],
          options: [long, 'Beta', long],
        },
        'android',
        theme,
      );
      await noOverflow(page, '[data-form-fixture], [data-form-fixture-ready]');
      await page.getByRole('button', { name: long }).click();
      await noOverflow(page, '.android-sheet');
      await page
        .getByRole(type === 'select' ? 'radio' : 'checkbox', { name: 'Beta', exact: true })
        .click();
      if (type === 'multiselect')
        await page.getByRole('button', { name: 'Confirm', exact: true }).click();
      expect(await changes(page)).toEqual([type === 'select' ? 'Beta' : ['Beta']]);
    });
  test(`Android ${theme}: dynamic name and controls occupy separate rows`, async ({ page }) => {
    await page.setViewportSize({ width: 320, height: 568 });
    await mountForm(
      page,
      '/src/components/editor/DynamicGroupEditor.tsx',
      'DynamicGroupEditor',
      {
        propertyId: 'group',
        label: long,
        value: [{ id: 'a', name: long, type: 'text', value: 'value' }],
        allowedTypes: ['text', 'number'],
      },
      'android',
      theme,
    );
    await noOverflow(page, '[data-form-fixture], [data-form-fixture-ready]');
    const name = page.getByRole('button', { name: long, exact: true });
    const remove = page.getByRole('button', { name: 'Delete', exact: true });
    const a = (await name.boundingBox())!,
      b = (await remove.boundingBox())!;
    expect(b.y).toBeGreaterThanOrEqual(a.y + a.height);
    await page.getByRole('button', { name: 'Field type' }).click();
    await page.getByRole('radio', { name: 'Number', exact: true }).click();
    expect(await changes(page)).toEqual([[{ id: 'a', name: long, type: 'number', value: 0 }]]);
  });
}
for (const platform of ['windows', 'macos'])
  test(`${platform}: field selectors remain native desktop controls`, async ({ page }) => {
    await mountForm(
      page,
      '/src/components/TemplateFieldInput.tsx',
      'TemplateFieldInput',
      {
        propertyId: 'public',
        label: 'Public',
        type: 'select',
        value: 'Alpha',
        options: ['Alpha', 'Beta'],
      },
      platform,
    );
    await expect(page.getByRole('combobox', { name: 'Public' })).toHaveValue('Alpha');
    await expect(page.locator('.android-sheet')).toHaveCount(0);
  });
