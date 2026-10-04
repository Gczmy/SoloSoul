import { expect, test } from '@playwright/test';
import { login, setupTauriMock } from './fixtures/auth';

// 桌面侧栏和鼠标场景显式使用桌面环境；各用例仍可自行测试窄视口。
test.use({ viewport: { width: 1280, height: 720 }, isMobile: false, hasTouch: false });

for (const surface of ['page', 'popover'] as const) {
  test(`${surface} 搜索揭示支持键盘、关键验证、取消与新查询失效`, async ({ page }) => {
    await setupTauriMock(page);
    await page.addInitScript(() => {
      Object.assign(window, {
        __MOCK_PLATFORM__: 'windows',
        __E2E_MOCKS__: {
          vault_check_directory: () => true,
          ocr_get_model_status: () => ({ installed: true, bundled: true }),
          ocr_list_available_tiers: () => [],
          ocr_get_active_tier: () => 'small',
          sync_list_conflicts: () => [],
          verify_password: () => true,
          search_unified: ({ query }: { query: string }) => ({
            items: ['sensitive', 'critical'].map((level) => ({
              objectId: level,
              name: `${level} result`,
              itemType: 'object',
              typeId: 'identity',
              relevance: 1,
              matchType: 'fieldValue',
              matchedField: level,
              matchedValue: `${query}_${level}_SECRET`,
              sensitivityLevels: ['public', level],
            })),
            total: 2,
            hasMore: false,
          }),
        },
      });
      localStorage.setItem('i18nextLng', 'en-US');
    });
    await login(page);
    if (surface === 'page') {
      await page.evaluate(() => {
        history.pushState({}, '', '/search');
        window.dispatchEvent(new PopStateEvent('popstate'));
      });
    } else {
      const nav = page.locator('#desktop-navigation');
      await nav.getByRole('button', { name: 'Tools', exact: true }).hover();
      await nav.getByRole('button', { name: 'Search', exact: true }).click();
    }
    const input = page.getByPlaceholder('Search objects, profiles...');
    const hidden = page.getByRole('button').filter({ hasText: /^••••••••$/ });
    await input.fill('first');
    await expect(hidden).toHaveCount(2);
    expect(await page.locator('body').innerHTML()).not.toContain('first_sensitive_SECRET');
    expect(await page.locator('body').innerHTML()).not.toContain('first_critical_SECRET');
    await hidden.first().focus();
    await page.keyboard.press('Enter');
    await expect(page.getByText('first_sensitive_SECRET')).toBeVisible();
    await expect(page.getByTestId('object-detail-modal')).toHaveCount(0);
    await hidden.first().focus();
    await page.keyboard.press('Space');
    const dialog = page.getByRole('dialog');
    await expect(dialog).toBeVisible();
    await dialog.getByRole('button', { name: 'Cancel', exact: true }).click();
    await expect(dialog).toHaveCount(0);
    await expect(input).toBeVisible();
    expect(await page.locator('body').innerHTML()).not.toContain('first_critical_SECRET');
    await hidden.first().click();
    await dialog
      .getByRole('textbox', { name: 'Enter password', exact: true })
      .fill('test-password');
    await dialog.getByRole('button', { name: 'Confirm', exact: true }).click();
    await expect(dialog).toHaveCount(0);
    await expect(page.getByText('first_critical_SECRET')).toBeVisible();
    await expect(page.getByTestId('object-detail-modal')).toHaveCount(0);
    await input.fill('second');
    await expect(hidden).toHaveCount(2);
    expect(await page.locator('body').innerHTML()).not.toContain('second_critical_SECRET');
    expect(await page.locator('body').innerHTML()).not.toContain('first_critical_SECRET');
  });
}
