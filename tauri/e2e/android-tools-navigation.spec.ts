import { test, expect, type Page } from '@playwright/test';
import { setupTauriMock, login } from './fixtures/auth';

test.beforeEach(async ({ page }) => {
  await setupTauriMock(page);
  await page.addInitScript(() => {
    const prefs = {
      theme: 'light',
      language: 'en-US',
      hasSeenOnboarding: true,
      autoLockTimeoutMinutes: 0,
    };
    Object.assign(window, {
      __MOCK_PLATFORM__: 'android',
      __E2E_MOCKS__: {
        ui_get_preferences: () => prefs,
        user_data_get_preferences: () => prefs,
        vault_check_directory: () => true,
        object_trash_list: () => [],
        attachment_count_stats: () => ({ attachmentCount: 0, photoCount: 0 }),
        attachment_list_all: () => ({ pages: [], trashPages: [] }),
        export_get_scope_tree: () => [],
        ocr_list_available_tiers: () => [],
        ocr_get_active_tier: () => 'small',
        ocr_get_model_status: () => ({ installed: true, bundled: true }),
        llm_get_config: () => ({ aiFeaturesEnabled: { chat: false }, activeProviderId: '' }),
        llm_get_providers: () => [],
        llm_list_conversations: () => [],
        llm_usage_stats: () => ({ usageCount: 0, totalTokens: 0, byProvider: [] }),
        sync_list_conflicts: () => [],
        sync_list_devices: () => [],
        sync_get_history: () => [],
        guide_load_index: () => ({
          categories: [{ id: 'basics', title: { en: 'Basics', zh: '基础' }, order: 0 }],
          guides: [
            {
              id: 'intro',
              title: { en: 'Introduction', zh: '介绍' },
              category: 'basics',
              order: 0,
              keywords: [],
              files: {},
            },
          ],
        }),
        guide_load_content: () => ({
          id: 'intro',
          title: 'Introduction',
          content: '# Introduction',
        }),
      },
    });
    localStorage.setItem('i18nextLng', 'en-US');
  });
  await login(page);
});

const toolEntries = [
  ['Search', '/search'],
  ['Trash', '/settings/trash'],
  ['Templates', '/settings/templates'],
  ['Attachments', '/settings/attachments'],
  ['Plugins', '/plugins'],
  ['OCR', '/ocr'],
  ['Import / Export', '/settings/export-import'],
  ['Device Sync', '/sync'],
  ['Help', '/help'],
  ['AI Chat', '/llm-chat'],
] as const;

async function back(page: Page) {
  await page.locator('.android-appbar').getByRole('button', { name: 'Back', exact: true }).click();
}

for (const [label, path] of toolEntries) {
  test(`Tools returns from ${path} using toolbar and history`, async ({ page }) => {
    await page.locator('.android-navigation a[href="/tools"]').click();
    const tool = page.locator('.android-tool').filter({ hasText: label });
    await tool.click();
    await expect(page).toHaveURL(path);
    await back(page);
    await expect(page).toHaveURL('/tools');
    await expect(tool).toBeVisible();
    // AppBar 的返回必须消费历史条目，不能压入新的 Tools 后又回到刚离开的页面。
    await page.goBack();
    await expect(page).toHaveURL('/');
    await page.locator('.android-navigation a[href="/tools"]').click();
    await tool.click();
    await expect(page).toHaveURL(path);
    await page.goBack();
    await expect(page).toHaveURL('/tools');
  });
}

test('AI settings and usage return through AI Chat to Tools', async ({ page }) => {
  await page.locator('.android-navigation a[href="/tools"]').click();
  await page.locator('.android-tool').filter({ hasText: 'AI Chat' }).click();
  await page.getByRole('button', { name: 'Configure LLM', exact: true }).last().click();
  await expect(page).toHaveURL('/settings/llm');
  await page.getByRole('button', { name: /Usage Statistics/i }).click();
  await expect(page).toHaveURL('/settings/llm/stats');
  await back(page);
  await expect(page).toHaveURL('/settings/llm');
  await back(page);
  await expect(page).toHaveURL('/llm-chat');
  await back(page);
  await expect(page).toHaveURL('/tools');
});

test('Help detail returns through the index to Tools', async ({ page }) => {
  await page.locator('.android-navigation a[href="/tools"]').click();
  await page.locator('.android-tool').filter({ hasText: 'Help' }).click();
  await page.getByText('Introduction', { exact: true }).click();
  await expect(page).toHaveURL('/help?id=intro');
  await back(page);
  await expect(page).toHaveURL('/help');
  await back(page);
  await expect(page).toHaveURL('/tools');
});

test('Settings entry keeps its own return destination', async ({ page }) => {
  await page.locator('.android-navigation a[href="/settings"]').click();
  await page.getByText('Template Manager', { exact: true }).click();
  await expect(page).toHaveURL('/settings/templates');
  await back(page);
  await expect(page).toHaveURL('/settings');
  await page.goBack();
  await expect(page).toHaveURL('/');
});

for (const name of ['Passport', '用于跨境旅行与长期居留登记的个人证件资料模板']) {
  test(`Template detail aligns icon with name: ${name}`, async ({ page }) => {
    await page.evaluate((templateName) => {
      (window as any).__E2E_MOCKS__.template_list = () => [
        {
          id: 'header-layout',
          name: templateName,
          iconId: 'Shield',
          category: 'identity',
          contractTypeId: 'com.solosoul.official.address-fmt/v1',
          properties: ['public', 'internal', 'sensitive', 'critical'].map((level) => ({
            id: level,
            name: level,
            type: 'text',
            sensitivityLevel: level,
          })),
        },
      ];
    }, name);
    await page.locator('.android-navigation a[href="/tools"]').click();
    await page.locator('.android-tool').filter({ hasText: 'Templates' }).click();
    await page.locator('[data-ui-card][role="button"]').filter({ hasText: name }).click();
    const dialog = page.getByRole('dialog', { name, exact: true });
    await expect(dialog).toBeVisible();
    for (const width of [320, 390, 768]) {
      await page.setViewportSize({ width, height: 844 });
      const heading = dialog.getByRole('heading', { name, exact: true });
      const titleBox = (await heading.boundingBox())!;
      const iconBox = (await heading.locator('..').locator(':scope > svg').boundingBox())!;
      const metaBox = (await dialog.getByText('Identity', { exact: true }).boundingBox())!;
      expect(
        Math.abs(iconBox.y + iconBox.height / 2 - titleBox.y - titleBox.height / 2),
      ).toBeLessThan(1);
      expect(iconBox.x + iconBox.width).toBeLessThan(titleBox.x);
      expect(metaBox.y).toBeGreaterThanOrEqual(titleBox.y + titleBox.height);
      expect(await dialog.evaluate((el) => el.scrollWidth <= el.clientWidth)).toBe(true);
      const close = dialog.getByRole('button', { name: 'Close', exact: true }).first();
      const closeBox = (await close.boundingBox())!;
      expect(closeBox.width).toBeGreaterThanOrEqual(48);
      expect(closeBox.x + closeBox.width).toBeLessThanOrEqual(width);
      if (width === 390)
        await dialog.screenshot({ path: test.info().outputPath('template-detail.png') });
    }
    await dialog.getByRole('button', { name: 'Close', exact: true }).first().click();
    await expect(dialog).toHaveCount(0);
    await back(page);
    await expect(page).toHaveURL('/tools');
  });
}
