import { test, expect } from '@playwright/test';
import { setupTauriMock, login } from './fixtures/auth';

test.beforeEach(async ({ page }) => {
  await setupTauriMock(page);
  await page.addInitScript(() => {
    const defaults = {
      theme: 'light',
      accentColor: 'ocean',
      reduceMotion: false,
      language: 'en-US',
      hasSeenOnboarding: true,
      autoLockTimeoutMinutes: 0,
    };
    const prefs = () => ({
      ...defaults,
      ...JSON.parse(localStorage.getItem('android-test-prefs') || '{}'),
    });
    const objects = [
      {
        id: 'object-a',
        name: 'Passport',
        typeId: 'identity',
        updatedAt: '2026-09-15T09:00:00Z',
        sensitivityLevel: 'critical',
      },
      {
        id: 'object-b',
        name: 'Savings',
        typeId: 'financial',
        updatedAt: '2026-09-14T09:00:00Z',
        sensitivityLevel: 'sensitive',
      },
    ].map((obj) => ({
      ...obj,
      accountId: 'e2e-account',
      createdAt: obj.updatedAt,
      properties: {
        secret: 'PRIVATE-TEST-VALUE',
        __fields: { secret: { name: 'Secret', type: 'text' } },
      },
      propertyLabels: { secret: 'critical' },
    }));
    Object.assign(window, {
      __MOCK_PLATFORM__: 'android',
      __E2E_MOCKS__: {
        ui_get_preferences: prefs,
        user_data_get_preferences: prefs,
        user_data_update_preference: ({ payload }: { payload: { preferences: object } }) => {
          localStorage.setItem(
            'android-test-prefs',
            JSON.stringify({ ...prefs(), ...payload.preferences }),
          );
        },
        object_list: ({ filter }: { filter?: { typeId?: string; parentId?: string } }) =>
          filter?.typeId
            ? objects.filter((obj) => obj.typeId === filter.typeId)
            : filter?.parentId
              ? []
              : objects,
        object_get: ({ objectId }: { objectId: string }) =>
          objects.find((obj) => obj.id === objectId),
        object_create: ({ input }: { input: { id: string; name: string; typeId: string } }) =>
          input,
        vault_check_directory: () => true,
        ocr_get_model_status: () => ({ installed: true, bundled: true }),
        sync_list_conflicts: () => [],
        object_trash_list: () => [],
        attachment_count_stats: () => ({ attachmentCount: 0, photoCount: 0 }),
      },
    });
    localStorage.setItem('i18nextLng', 'en-US');
  });
  await login(page);
});

test('Android navigation stays mounted and icons keep their geometry', async ({ page }) => {
  await expect(page.locator('html')).toHaveAttribute('data-platform', 'android');
  const icons = await page.locator('.android-nav-icon svg').elementHandles();
  const boxes = await Promise.all(icons.map((icon) => icon.boundingBox()));
  for (const path of ['/workspace', '/tools', '/settings', '/']) {
    await page.locator(`.android-navigation a[href="${path}"]`).click();
    await expect(page.locator(`.android-navigation a[href="${path}"]`)).toHaveAttribute(
      'aria-current',
      'page',
    );
    for (const [index, icon] of icons.entries()) {
      expect(await icon.evaluate((node) => node.isConnected)).toBe(true);
      expect(await icon.boundingBox()).toEqual(boxes[index]);
    }
  }
  await expect(page.getByTestId('android-home')).toBeVisible();
});

test('responsive surfaces and navigation rail have no horizontal overflow', async ({ page }) => {
  for (const width of [320, 360, 390, 430, 768, 1024]) {
    await page.setViewportSize({ width, height: 844 });
    await expect(page.locator('.android-navigation')).toBeVisible();
    const dimensions = await page.evaluate(() => ({
      root: document.documentElement.scrollWidth,
      viewport: innerWidth,
      content: document.querySelector('main')!.scrollWidth,
      available: document.querySelector('main')!.clientWidth,
    }));
    expect(dimensions.root).toBeLessThanOrEqual(dimensions.viewport);
    expect(dimensions.content).toBeLessThanOrEqual(dimensions.available);
    const nav = await page.locator('.android-navigation').boundingBox();
    expect(width >= 768 ? nav!.width : nav!.height).toBe(width >= 768 ? 88 : 80);
  }
  await page.setViewportSize({ width: 390, height: 844 });
  await page.screenshot({ path: test.info().outputPath('android-home.png') });
});

test('new sheet traps focus, hardware back closes it, page creation preserves route', async ({
  page,
}) => {
  await page.locator('.android-fab').click();
  await expect(page.getByRole('dialog')).toBeVisible();
  await expect(page.locator('#root')).toHaveJSProperty('inert', true);
  await page.keyboard.press('Shift+Tab');
  await expect(page.getByRole('button', { name: 'Close', exact: true })).toBeFocused();
  await page.keyboard.press('Escape');
  await expect(page.getByRole('dialog')).toHaveCount(0);
  await expect(page.locator('.android-fab')).toBeFocused();
  await page.locator('.android-fab').click();
  await page.goBack();
  await expect(page.getByRole('dialog')).toHaveCount(0);
  await expect(page).toHaveURL('/');
  await page.locator('.android-fab').click();
  await page.getByRole('button', { name: /Add Page|New Page/i }).click();
  const input = page.getByRole('textbox').first();
  await expect(input).toBeFocused();
  await input.fill('Reading');
  await page.getByRole('button', { name: 'notebook', exact: true }).click();
  await expect(page.getByRole('dialog')).toBeVisible();
  await page.locator('.android-page-form button[type="submit"]').click();
  await expect(page).toHaveURL(/\/workspace\/custom\//);
  await expect(page.getByRole('dialog')).toHaveCount(0);
  await expect(page.locator('header h1')).toHaveText('Reading');
});

test('all objects, filters, search, details and contextual creation reuse business routes', async ({
  page,
}) => {
  await page.locator('.android-navigation a[href="/workspace"]').click();
  await expect(page.getByTestId('workspace-object-card')).toHaveCount(2);
  const identity = page.locator('.android-chip').filter({ hasText: 'Identity' });
  const width = (await identity.boundingBox())!.width;
  await identity.click();
  await expect(identity).toHaveAttribute('aria-pressed', 'true');
  expect((await identity.boundingBox())!.width).toBe(width);
  await expect(page.getByTestId('workspace-object-card')).toHaveCount(1);
  await page.locator('.android-fab').click();
  await page.getByRole('button', { name: /New object/ }).click();
  await expect(page).toHaveURL('/editor?section=identity');
  await page.locator('.android-navigation a[href="/workspace"]').click();
  await page.getByRole('textbox').fill('Savings');
  await expect(page.getByTestId('workspace-object-card')).toHaveCount(1);
  await page.getByRole('button', { name: /^Savings/ }).click();
  await expect(page.getByTestId('object-detail-modal')).toBeVisible();
  await expect(page.getByText('PRIVATE-TEST-VALUE', { exact: true })).toHaveCount(0);
});

test('appearance persists across reload and a locked vault drops cached names', async ({
  page,
}) => {
  await page.locator('.android-navigation a[href="/settings"]').click();
  await page.getByText('Theme & Appearance', { exact: true }).click();
  await page.getByRole('button', { name: 'Dark', exact: true }).click();
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
  await page.getByRole('button', { name: 'Sage', exact: true }).click();
  await page.getByRole('checkbox').check();
  await expect(page.locator('html')).toHaveAttribute('data-user-reduce-motion', 'true');
  await page.reload();
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
  // 重载重新进入锁定页，解锁后读取已保存的账户偏好。
  await login(page);
  await expect(page.locator('html')).toHaveAttribute('data-accent', 'forest');
  await expect(page.locator('html')).toHaveAttribute('data-user-reduce-motion', 'true');
  await page.getByRole('button', { name: /Lock Vault/i }).click();
  await expect(page.getByTestId('android-home')).toHaveCount(0);
  await expect(page.getByText('Passport', { exact: true })).toHaveCount(0);
  await expect(page.locator('.android-sheet')).toHaveCount(0);
});
