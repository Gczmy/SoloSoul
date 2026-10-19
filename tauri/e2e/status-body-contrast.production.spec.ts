import { test, expect, type Page, type Locator } from '@playwright/test';
import { THEME_SCHEMES, getSchemeById } from '../src/lib/themeSchemes';
import { readFileSync } from 'node:fs';
import { setupTauriMock, login } from './fixtures/auth';
import { measureControlContrast } from './fixtures/contrast';
const messages = JSON.parse(
  readFileSync(new URL('../src/locales/en-US/settings.json', import.meta.url), 'utf8'),
);

async function navigate(page: Page, destination: string) {
  await page.evaluate((path) => {
    history.pushState({ ...history.state, idx: (history.state?.idx ?? 0) + 1 }, '', path);
    dispatchEvent(new PopStateEvent('popstate', { state: history.state }));
  }, destination);
  await expect(page).toHaveURL(destination);
}
for (const platform of ['android', 'windows', 'macos'] as const)
  for (const scheme of THEME_SCHEMES.map(
    (s) => getSchemeById(s.id, platform === 'android' ? 'shared' : 'desktop')!,
  ))
    for (const contrast of ['no-preference', 'more'] as const) {
      const mode = scheme.mode;
      for (const scenario of [
        'up-to-date',
        'update-error',
        'available',
        'download-error',
        'provider-success',
        'provider-error',
      ]) {
        test(`${platform} ${scheme.id} ${contrast} ${scenario} status body stays readable`, async ({
          page,
        }) => {
          await page.setViewportSize({ width: platform === 'android' ? 390 : 1000, height: 844 });
          await page.emulateMedia({ contrast });
          await setupTauriMock(page);
          await page.addInitScript(
            ({ platform, mode, scenario, scheme }) => {
              const prefs = {
                theme: mode,
                defaultLightTheme: mode === 'light' ? scheme.id : 'warm-stone',
                defaultDarkTheme: mode === 'dark' ? scheme.id : 'warm-stone-dark',
                language: 'en-US',
                hasSeenOnboarding: true,
                reduceMotion: true,
              };
              const update = {
                currentVersion: '2.13.2',
                latestVersion: ['available', 'download-error'].includes(scenario)
                  ? '9.9.9'
                  : '2.13.2',
                releaseNotes: 'Fixture notes',
                downloadUrl: 'https://fixture.invalid/public.apk',
                checksum: 'a'.repeat(64),
                checksumWarning: 'Fixture checksum warning',
                mandatory: false,
              };
              const fail = () => {
                throw new Error('Fixture update unavailable');
              };
              Object.assign(window, {
                __MOCK_PLATFORM__: platform,
                __E2E_MOCKS__: {
                  ui_get_preferences: () => prefs,
                  user_data_get_preferences: () => prefs,
                  vault_check_directory: () => true,
                  get_app_info: () => ({
                    appName: 'SoloSoul',
                    version: '2.13.2',
                    os: platform,
                    arch: 'x86_64',
                  }),
                  android_cached_update: () => null,
                  android_is_apk_downloaded: () => false,
                  create_update_download: () => 29,
                  android_download_apk: () => {
                    throw new Error('Fixture download unavailable');
                  },
                  desktop_download_update: () => {
                    throw new Error('Fixture download unavailable');
                  },
                  android_check_update: scenario === 'update-error' ? fail : () => update,
                  desktop_prepare_update:
                    scenario === 'update-error'
                      ? fail
                      : () =>
                          ['available', 'download-error'].includes(scenario)
                            ? {
                                rid: 19,
                                current_version: '2.13.2',
                                version: '9.9.9',
                                body: 'Fixture notes',
                                raw_json: {},
                              }
                            : null,
                  desktop_check_update: scenario === 'update-error' ? fail : () => update,
                  llm_get_providers: () => [
                    {
                      id: 'public-fixture',
                      name: 'Public fixture',
                      baseUrl: 'https://fixture.invalid/v1',
                      model: 'fixture-model',
                      isEnabled: true,
                      isBuiltIn: true,
                      apiKey: 'public-unused',
                      apiType: 'openAI',
                    },
                  ],
                  llm_get_config: () => ({
                    activeProviderId: 'public-fixture',
                    aiFeaturesEnabled: { chat: false },
                    hasAcceptedRisk: true,
                    useLocalEmbedding: false,
                  }),
                  llm_check_embedding_available: () => false,
                  llm_get_embed_models: () => [],
                  llm_test_provider: () => {
                    if (scenario === 'provider-error')
                      throw new Error('Fixture provider unavailable');
                    return 'fixture response';
                  },
                },
              });
              localStorage.setItem('i18nextLng', 'en-US');
            },
            { platform, mode, scenario, scheme: { id: scheme.id } },
          );
          await login(page);
          await expect(page.locator('html')).toHaveAttribute('data-theme', mode);
          await expect
            .poll(() =>
              page.evaluate(() =>
                getComputedStyle(document.documentElement).getPropertyValue('--bg-elevated').trim(),
              ),
            )
            .toBe(scheme.variables['--bg-elevated']);
          const samples: Array<
            { name: string } & Awaited<ReturnType<typeof measureControlContrast>>
          > = [];
          async function measure(name: string, locator: Locator) {
            await expect(locator).toBeVisible();
            await locator.scrollIntoViewIfNeeded();
            const value = await measureControlContrast(locator, '[data-ui-card="default"]');
            samples.push({ name, ...value });
            expect
              .soft(value.foregroundContrast, `${name} body >=4.5:1`)
              .toBeGreaterThanOrEqual(4.5);
            if (contrast === 'more')
              expect
                .soft(value.foreground, `${name} follows high contrast`)
                .toEqual(mode === 'dark' ? [255, 255, 255] : [17, 17, 17]);
          }
          if (scenario.startsWith('provider')) {
            await navigate(page, '/settings/llm');
            await page
              .getByText('Public fixture', { exact: true })
              .locator('..')
              .locator('..')
              .getByRole('button')
              .first()
              .click();
            await page
              .getByRole('button', { name: messages.llm_test_connection, exact: true })
              .click();
            await measure(
              scenario,
              scenario === 'provider-success'
                ? page.getByText('✓ Connected "fixture response"', { exact: true })
                : page.getByText(/^✗ Connection failed /),
            );
          } else {
            await navigate(page, '/about');
            if (scenario === 'up-to-date')
              await measure(scenario, page.getByText(messages.latest_version, { exact: true }));
            if (scenario === 'update-error') {
              await measure(
                'update-error-badge',
                page.getByText(messages.update_check_failed, { exact: true }),
              );
              await measure(
                'update-error-detail',
                page.getByText('Fixture update unavailable', { exact: true }).locator('..'),
              );
            }
            if (scenario === 'available') {
              await measure(
                'update-available-badge',
                page.getByText(messages.update_available.replace('{{version}}', '9.9.9'), {
                  exact: true,
                }),
              );
              if (platform === 'android')
                await measure(
                  'checksum-warning',
                  page.getByText('Fixture checksum warning', { exact: true }).last().locator('..'),
                );
            }
          }
          if (scenario === 'download-error') {
            await page
              .getByRole('main')
              .getByRole('button', { name: messages.update_now, exact: true })
              .click();
            const updateCard = page
              .locator('[data-ui-card="default"]')
              .filter({ hasText: 'v2.13.2' });
            await measure(
              'download-error',
              updateCard.getByText('Error: Fixture download unavailable', { exact: true }),
            );
          }
          await test.info().attach('actual-status-colors', {
            body: JSON.stringify({
              platform,
              mode,
              scheme: scheme.id,
              contrast,
              scenario,
              samples,
            }),
            contentType: 'application/json',
          });
          await test.info().attach('actual-production-route', {
            body: await page.screenshot({ fullPage: false }),
            contentType: 'image/png',
          });
        });
      }
    }
