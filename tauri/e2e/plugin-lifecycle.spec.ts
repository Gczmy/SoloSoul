import { test, expect } from '@playwright/test';
import { login, navigateToPlugins, setupTauriMock } from './fixtures/auth';

const PLUGIN_ID = 'com.e2e.hello';
const PLUGIN_NAME = 'Hello E2E';

const marketPlugin = (installed?: boolean) => ({
  pluginId: PLUGIN_ID,
  installedVersion: installed ? '1.0.0' : undefined,
  hasUpdate: false,
  isCompatible: true,
  tier: 'p1',
  category: 'demo',
  registryEntry: {
    id: PLUGIN_ID,
    name: PLUGIN_NAME,
    author: 'E2E',
    description: 'A plugin for end-to-end testing.',
    latestVersion: '1.0.0',
    minCoreVersion: '2.0.0',
    wasmHashSha256: '00'.repeat(32),
    permissions: ['solosoul_show_dialog'],
    categories: ['demo'],
    i18n: {
      en: { name: PLUGIN_NAME, description: 'A plugin for end-to-end testing.' },
      zh: { name: '你好 E2E', description: '用于端到端测试的插件。' },
    },
  },
});

test.beforeEach(async ({ page }) => {
  await setupTauriMock(page);
});

test('renders plugin dashboard and filters by tier', async ({ page }) => {
  page.on('console', (msg) => console.log('[PAGE]', msg.type(), msg.text()));
  page.on('pageerror', (err) => console.log('[PAGE ERROR]', err.message));
  await page.addInitScript({
    content: `
      window.__E2E_MOCKS__ = {
        plugin_list_all: () => [${JSON.stringify(marketPlugin(false))}],
        plugin_list_installed: () => [],
        plugin_audit_log: () => [],
      };
    `,
  });

  await login(page);
  await navigateToPlugins(page);

  await expect(page.locator('text=Hello E2E')).toBeVisible();

  // Default enabled tiers are P0/P1/P2, so P3+ chips are disabled.
  await expect(page.getByRole('button', { name: 'P3' })).toBeDisabled();
  await expect(page.getByRole('button', { name: 'P4' })).toBeDisabled();
});

test('installs a plugin and runs it through dialog response', async ({ page }) => {
  await page.addInitScript({
    content: `
      window.__E2E_INSTALLED__ = false;
      window.__E2E_PLUGIN_ID__ = ${JSON.stringify(PLUGIN_ID)};
      window.__E2E_PLUGIN_NAME__ = ${JSON.stringify(PLUGIN_NAME)};
      window.__E2E_MOCKS__ = {
        plugin_list_all: () => [
          window.__E2E_INSTALLED__
            ? ${JSON.stringify(marketPlugin(true))}
            : ${JSON.stringify(marketPlugin(false))},
        ],
        plugin_list_installed: () =>
          window.__E2E_INSTALLED__
            ? [{
                id: window.__E2E_PLUGIN_ID__,
                name: window.__E2E_PLUGIN_NAME__,
                version: '1.0.0',
                description: 'A plugin for end-to-end testing.',
                author: 'E2E',
                permissions: ['solosoul_show_dialog'],
                requiredCoreVersion: '2.0.0',
                wasmHashSha256: '00'.repeat(32),
                dataTtlSeconds: 300,
                tier: 'p1',
                category: 'demo',
              }]
            : [],
        plugin_install: () => {
          window.__E2E_INSTALLED__ = true;
          return { pluginId: window.__E2E_PLUGIN_ID__, version: '1.0.0' };
        },
        plugin_run: (_args, emit, finish) => {
          setTimeout(() => {
            emit({
              eventType: 'log',
              jsonData: JSON.stringify({ id: '1', level: 'info', message: 'start', timestamp: Date.now() }),
            });
          }, 50);
          setTimeout(() => {
            emit({
              eventType: 'dialog_request',
              requestId: 'dlg-1',
              pluginId: window.__E2E_PLUGIN_ID__,
              pluginName: window.__E2E_PLUGIN_NAME__,
              jsonData: JSON.stringify({
                type: 'input',
                title: 'Your Name',
                message: 'Please enter your name.',
                placeholder: 'Name',
              }),
            });
          }, 100);
          setTimeout(() => {
            emit({
              eventType: 'result',
              jsonData: JSON.stringify({ type: 'text', content: 'done' }),
            });
          }, 600);
          setTimeout(() => {
            emit({ eventType: 'completed', jsonData: JSON.stringify({ exitCode: 0 }) });
            finish({ exitCode: 0, logs: [], results: [{ type: 'text', content: 'done' }], fuelConsumed: 0 });
          }, 1200);
        },
        plugin_consent_response: () => undefined,
        plugin_dialog_response: () => undefined,
        plugin_audit_log: () => [],
      };
    `,
  });

  await login(page);
  await navigateToPlugins(page);

  // Install the plugin.
  await page.getByRole('button', { name: 'Install', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Run', exact: true })).toBeVisible({ timeout: 5000 });
  await expect(page.getByText('Installed').first()).toBeVisible();

  // Run the plugin.
  await page.getByRole('button', { name: 'Run', exact: true }).click();
  await expect(page.locator('text=start')).toBeVisible({ timeout: 5000 });

  // Dialog appears.
  await expect(page.getByRole('heading', { name: 'Your Name', exact: true })).toBeVisible({ timeout: 5000 });
  await page.locator('input[placeholder="Name"]').fill('Playwright');
  await page.locator('button:has-text("Confirm")').click();

  // Result is rendered.
  await expect(page.locator('text=done')).toBeVisible({ timeout: 5000 });
});
