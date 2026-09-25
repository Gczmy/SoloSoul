import { expect, test } from '@playwright/test';
import { login, setupTauriMock } from './fixtures/auth';

test('搜索页仅显示最新查询，清空后迟到结果不恢复', async ({ page }, testInfo) => {
  await setupTauriMock(page);
  await page.addInitScript(
    ({ mobile }) => {
      const pending: Record<string, (value: unknown) => void> = {};
      Object.assign(window, {
        __MOCK_PLATFORM__: mobile ? 'android' : 'windows',
        __SEARCH_PENDING__: pending,
        __E2E_MOCKS__: {
          vault_check_directory: () => true,
          search_unified: ({ query }: { query: string }) =>
            new Promise((resolve) => {
              pending[query] = resolve;
            }),
        },
      });
      localStorage.setItem('i18nextLng', 'en-US');
    },
    { mobile: testInfo.project.name === 'mobile' },
  );
  await login(page);
  await page.evaluate(() => {
    history.pushState({}, '', '/search');
    window.dispatchEvent(new PopStateEvent('popstate'));
  });
  const input = page.getByPlaceholder('Search objects, profiles...');
  const waitForQuery = (query: string) =>
    page.waitForFunction(
      (q) =>
        q in
        (window as unknown as { __SEARCH_PENDING__: Record<string, unknown> }).__SEARCH_PENDING__,
      query,
    );
  const complete = (query: string, name: string) =>
    page.evaluate(
      ({ query, name }) => {
        const pending = (
          window as unknown as { __SEARCH_PENDING__: Record<string, (value: unknown) => void> }
        ).__SEARCH_PENDING__;
        pending[query]({
          items: [{ objectId: query, name, typeId: 'identity', itemType: 'object', relevance: 1 }],
          total: 1,
          hasMore: false,
        });
      },
      { query, name },
    );
  await input.fill('aaa');
  await waitForQuery('aaa');
  await input.fill('bbb');
  await waitForQuery('bbb');
  await complete('bbb', 'Latest result');
  await expect(page.getByText('Latest result', { exact: true })).toBeVisible();
  await complete('aaa', 'Stale result');
  await expect(page.getByText('Stale result', { exact: true })).toHaveCount(0);
  await input.fill('ccc');
  await waitForQuery('ccc');
  await page.getByRole('button', { name: 'Clear', exact: true }).click();
  await complete('ccc', 'Cleared result');
  await expect(input).toHaveValue('');
  await expect(page.getByText('Cleared result', { exact: true })).toHaveCount(0);
  await expect(page.getByText('Latest result', { exact: true })).toHaveCount(0);
});
