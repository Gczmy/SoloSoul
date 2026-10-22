import { expect, test } from '@playwright/test';
import { mountForm, noOverflow } from './fixtures/android-form';
for (const platform of ['android', 'windows', 'macos'])
  test(`${platform}: plugin parameters preserve form state and Android long text fits`, async ({
    page,
  }) => {
    await page.setViewportSize({ width: platform === 'android' ? 320 : 1000, height: 700 });
    const long = 'PublicParameter_ABCDEFGHIJKLMNOPQRSTUVWXYZ'.repeat(5);
    await mountForm(
      page,
      '/src/components/plugin/PluginRunParamsDialog.tsx',
      'PluginRunParamsDialog',
      {
        pluginName: long,
        params: [
          {
            id: 'choice',
            type: 'select',
            label: long,
            description: long,
            required: true,
            options: [{ value: 'one', label: long }],
            defaultValue: null,
          },
          {
            id: 'enabled',
            type: 'boolean',
            label: 'Enabled',
            description: long,
            defaultValue: 'false',
            required: false,
          },
          { id: 'count', type: 'number', label: 'Count', defaultValue: '0', required: false },
        ],
      },
      platform,
    );
    const host = page.locator('[data-form-fixture]');
    await host.getByRole('button', { name: 'Run', exact: true }).click();
    await expect(host.getByText('Required', { exact: true })).toBeVisible();
    if (platform === 'android') {
      await noOverflow(page, '[data-form-fixture] > div > div > div');
      await host.getByRole('button', { name: long, exact: true }).click();
      await noOverflow(page, '.android-sheet');
      await page.evaluate(() => history.back());
      await expect(host.getByRole('button', { name: 'Run', exact: true })).toBeVisible();
      await host.getByRole('button', { name: long, exact: true }).click();
      await page.getByRole('radio', { name: long, exact: true }).click();
    } else await host.getByRole('combobox').selectOption('one');
    await host.getByRole('button', { name: 'Run', exact: true }).click();
    const actions = await page.evaluate(
      () =>
        (window as unknown as { __FORM_ACTIONS__: { name: string; args: unknown[] }[] })
          .__FORM_ACTIONS__,
    );
    expect(actions.find((a) => a.name === 'onSubmit')?.args[0]).toEqual({
      choice: 'one',
      enabled: 'false',
      count: '0',
    });
  });
