import { expect, type Page } from '@playwright/test';
import { login, setupTauriMock } from './auth';

/** Public, isolated component harness under the app's real styles/platform initialization. */
export async function mountForm(
  page: Page,
  module: string,
  name: string,
  props: Record<string, unknown>,
  platform = 'android',
  theme = 'light',
) {
  await setupTauriMock(page);
  await page.addInitScript(
    ({ platform, theme }) => {
      const prefs = {
        theme,
        language: 'en-US',
        hasSeenOnboarding: true,
        autoLockTimeoutMinutes: 0,
        reduceMotion: true,
        backupReminderDays: 0,
      };
      Object.assign(window, {
        __MOCK_PLATFORM__: platform,
        __E2E_MOCKS__: {
          ui_get_preferences: () => prefs,
          user_data_get_preferences: () => prefs,
          object_list: () => [],
          template_list: () => [],
          sync_list_conflicts: () => [],
          vault_check_directory: () => true,
        },
      });
      localStorage.setItem('language', 'en-US');
    },
    { platform, theme },
  );
  await login(page);
  await page.evaluate(
    async ({ module, name, props }) => {
      // Use the same optimized module URLs as the app, including Vite's version query.
      // Importing an unversioned URL creates another React dispatcher in development.
      const source = await (await fetch('/src/bootstrapApp.tsx')).text();
      const reactPath = source.match(/"([^"\n]*\/react\.js[^"\n]*)"/)![1];
      const domPath = source.match(/"([^"\n]*\/react-dom_client\.js[^"\n]*)"/)![1];
      const React = (await import(reactPath)).default;
      const dom = await import(domPath);
      const Component = (await import(module))[name];
      const host = document.createElement('div');
      host.dataset.formFixture = '';
      host.style.cssText =
        'position:fixed;inset:0;z-index:4500;padding:16px;box-sizing:border-box;overflow:auto;background:var(--bg-base);color:var(--text-primary)';
      document.body.append(host);
      const changes: unknown[] = [];
      const actions: unknown[] = [];
      Object.assign(window, { __FORM_CHANGES__: changes, __FORM_ACTIONS__: actions });
      const callbacks = Object.fromEntries(
        [
          'onSetActive',
          'onDeleteProvider',
          'onSaveProvider',
          'onTestConnection',
          'onSubmit',
          'onCancel',
          'onSetObjectConflictStrategy',
        ].map((key) => [
          key,
          (...args: unknown[]) => {
            actions.push({ name: key, args });
            if (key === 'onSaveProvider') return Promise.resolve(true);
            if (key === 'onTestConnection') return Promise.resolve('Public connection response');
          },
        ]),
      );
      function Fixture() {
        const [value, setValue] = React.useState(props.value);
        return React.createElement(
          'div',
          { 'data-form-fixture-ready': '', style: { minWidth: 0, width: '100%' } },
          React.createElement(Component, {
            ...callbacks,
            ...props,
            value,
            onChange: (next: unknown) => {
              changes.push(next);
              setValue(next);
            },
          }),
        );
      }
      (dom.createRoot ?? dom.default.createRoot)(host).render(React.createElement(Fixture));
    },
    { module, name, props },
  );
  await expect(page.locator('[data-form-fixture-ready]')).toBeVisible();
}

export async function noOverflow(page: Page, selector: string) {
  const nodes = await page.locator(selector).evaluateAll((elements) =>
    elements.map((el) => ({
      width: el.clientWidth,
      scroll: el.scrollWidth,
      left: el.getBoundingClientRect().left,
      right: el.getBoundingClientRect().right,
    })),
  );
  expect(nodes.length).toBeGreaterThan(0);
  for (const node of nodes) {
    expect(node.scroll).toBeLessThanOrEqual(node.width + 1);
    expect(node.left).toBeGreaterThanOrEqual(-1);
    expect(node.right).toBeLessThanOrEqual((await page.evaluate(() => innerWidth)) + 1);
  }
}

export function changes(page: Page) {
  return page.evaluate(
    () => (window as unknown as { __FORM_CHANGES__: unknown[] }).__FORM_CHANGES__,
  );
}
