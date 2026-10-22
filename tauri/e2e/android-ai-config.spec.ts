import { expect, test } from '@playwright/test';
import { mountForm, noOverflow } from './fixtures/android-form';
for (const platform of ['android', 'windows', 'macos'])
  test(`${platform}: provider configuration preserves values and Android long text fits`, async ({
    page,
  }) => {
    await page.setViewportSize({ width: platform === 'android' ? 320 : 1000, height: 700 });
    const long = 'PublicProvider_ABCDEFGHIJKLMNOPQRSTUVWXYZ'.repeat(5);
    await mountForm(
      page,
      '/src/components/llm-config/ProviderManagerPanel.tsx',
      'ProviderManagerPanel',
      {
        providers: [
          {
            id: 'public',
            name: long,
            model: long,
            baseUrl: 'https://example.invalid',
            apiKey: 'public-fixture',
            apiType: 'openAI',
            isEnabled: true,
            isBuiltIn: false,
          },
        ],
        activeId: 'public',
        loading: false,
        accountId: 'public',
      },
      platform,
    );
    const host = page.locator('[data-form-fixture]');
    if (platform === 'android') await noOverflow(page, '[data-form-fixture], [data-ui-card]');
    await host.getByRole('button', { name: new RegExp('Configure') }).click();
    if (platform === 'android') {
      await noOverflow(page, '[data-ui-card]');
      await host.getByRole('radio', { name: 'Anthropic', exact: true }).click();
    } else await host.getByRole('combobox').selectOption('anthropic');
    await host.getByRole('button', { name: 'Save', exact: true }).click();
    const actions = await page.evaluate(
      () =>
        (window as unknown as { __FORM_ACTIONS__: { name: string; args: unknown[] }[] })
          .__FORM_ACTIONS__,
    );
    expect(actions.find((a) => a.name === 'onSaveProvider')?.args[0]).toMatchObject({
      id: 'public',
      name: long,
      model: long,
      apiKey: 'public-fixture',
      apiType: 'anthropic',
    });
  });
