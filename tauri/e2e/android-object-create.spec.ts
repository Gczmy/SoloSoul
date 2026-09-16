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
      androidGlass: 'local',
    };
    const now = '2026-09-16T00:00:00Z';
    const objects: Record<string, any>[] = [
      {
        id: 'reading-page',
        name: 'Reading',
        typeId: 'page',
        properties: {},
        iconName: 'notebook',
        createdAt: now,
        updatedAt: now,
      },
      {
        id: 'deleted-page',
        name: 'Deleted page',
        typeId: 'page',
        properties: {},
        isDeleted: true,
        createdAt: now,
        updatedAt: now,
      },
    ];
    const templates = [
      { id: 'identity-template', name: 'Identity record', category: 'identity' },
      { id: 'travel-template', name: 'Travel record', category: 'travel' },
      { id: 'reading-template', name: 'Book record', category: 'reading-page' },
    ].map((tpl) => ({
      ...tpl,
      accountId: 'e2e-account',
      createdAt: now,
      updatedAt: now,
      properties: [{ id: 'notes', name: 'Notes', type: 'text', sensitivityLevel: 'internal' }],
    }));
    Object.assign(window, {
      __MOCK_PLATFORM__: 'android',
      __createdObjects: [],
      __E2E_MOCKS__: {
        ui_get_preferences: () => prefs,
        user_data_get_preferences: () => prefs,
        user_data_update_preference: ({ payload }: any) =>
          Object.assign(prefs, payload.preferences),
        template_list: () => templates,
        object_field_suggestions: () => [],
        object_list: ({ filter }: any) =>
          objects.filter(
            (obj) =>
              (filter?.includeDeleted || !obj.isDeleted) &&
              (!filter?.typeId || obj.typeId === filter.typeId) &&
              (!filter?.parentId || obj.parentId === filter.parentId),
          ),
        object_get: ({ objectId }: any) => objects.find((obj) => obj.id === objectId),
        object_create: ({ input }: any) => {
          (window as any).__createdObjects.push(input);
          const object = {
            ...input,
            id: `created-${objects.length}`,
            createdAt: now,
            updatedAt: now,
            sensitivityLevel: 'internal',
          };
          objects.push(object);
          return object;
        },
        vault_check_directory: () => true,
        object_trash_list: () => [],
        sync_list_conflicts: () => [],
        attachment_count_stats: () => ({ attachmentCount: 0, photoCount: 0 }),
      },
    });
    localStorage.setItem('i18nextLng', 'en-US');
  });
  await login(page);
  await expect(page.locator('.android-category').filter({ hasText: 'Reading' })).toBeVisible();
});

async function openNewObject(page: Page) {
  await page.locator('.android-fab').click();
  await page.getByRole('button', { name: /^New object/ }).click();
}

test('Home chooses a destination before templates; saving returns home and the object is in its page', async ({
  page,
}) => {
  await openNewObject(page);
  const picker = page.getByTestId('object-destination-picker');
  await expect(picker).toBeVisible();
  await expect(page.getByRole('button', { name: 'Book record', exact: true })).toHaveCount(0);
  await expect(picker.getByRole('button', { name: 'Deleted page' })).toHaveCount(0);
  await page.screenshot({ path: test.info().outputPath('choose-page.png') });
  await picker.getByRole('button', { name: 'Reading', exact: true }).click();
  await expect(page).toHaveURL('/editor?parentId=reading-page');
  await expect(page.getByTestId('object-save-destination')).toHaveText('Save to: Reading');
  await expect(page.getByRole('button', { name: 'Book record', exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Travel record', exact: true })).toHaveCount(0);
  await page.getByLabel('Object Name', { exact: true }).fill('My book');
  await page.screenshot({ path: test.info().outputPath('save-to-reading.png') });
  await page.getByRole('button', { name: 'Save', exact: true }).click();
  await expect(page).toHaveURL('/');
  expect(await page.evaluate(() => (window as any).__createdObjects)).toEqual([
    expect.objectContaining({
      name: 'My book',
      typeId: 'reading-page',
      parentId: 'reading-page',
      templateId: 'reading-template',
    }),
  ]);
  await page
    .locator('.android-category')
    .filter({ hasText: 'Reading' })
    .getByRole('button')
    .first()
    .click();
  await expect(page.getByTestId('workspace-object-card')).toContainText('My book');
  await openNewObject(page);
  await expect(picker).toHaveCount(0);
  await expect(page.getByTestId('object-save-destination')).toHaveText('Save to: Reading');
});

test('built-in destination filters templates and has no custom parent; contextual creation skips picker', async ({
  page,
}) => {
  await openNewObject(page);
  await page
    .getByTestId('object-destination-picker')
    .getByRole('button', { name: 'Travel', exact: true })
    .click();
  await expect(page.getByTestId('object-save-destination')).toHaveText('Save to: Travel');
  await expect(page.getByRole('button', { name: 'Identity record', exact: true })).toHaveCount(0);
  await page.getByLabel('Object Name', { exact: true }).fill('Trip');
  await page.getByRole('button', { name: 'Save', exact: true }).click();
  await expect(page).toHaveURL('/');
  const [created] = await page.evaluate(() => (window as any).__createdObjects);
  expect(created).toMatchObject({ typeId: 'travel', templateId: 'travel-template' });
  expect(created.parentId).toBeUndefined();
  await page
    .locator('.android-category')
    .filter({ hasText: 'Travel' })
    .getByRole('button')
    .first()
    .click();
  await expect(page.getByTestId('workspace-object-card')).toContainText('Trip');
  await openNewObject(page);
  await expect(page).toHaveURL('/editor?section=travel');
  await expect(page.getByTestId('object-destination-picker')).toHaveCount(0);
});

test('empty destination stays scoped and Back returns to the source', async ({ page }) => {
  await page.locator('.android-navigation a[href="/tools"]').click();
  await openNewObject(page);
  await page
    .getByTestId('object-destination-picker')
    .getByRole('button', { name: 'Professional', exact: true })
    .click();
  await expect(page.getByTestId('object-save-destination')).toHaveText('Save to: Professional');
  await expect(page.getByText('No templates for this page.', { exact: false })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Save', exact: true })).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Book record', exact: true })).toHaveCount(0);
  await page.getByRole('button', { name: 'Back', exact: true }).click();
  await expect(page).toHaveURL('/tools');
});

test('native enhanced create menu also opens the shared destination picker', async ({ page }) => {
  await page.locator('.android-navigation a[href="/settings"]').click();
  await page.getByText('Theme & Appearance', { exact: true }).click();
  await page.getByRole('button', { name: 'Enhanced glass', exact: true }).click();
  await page.locator('.android-navigation a[href="/"]').click();
  await page.evaluate(() => {
    const mocks = (window as any).__E2E_MOCKS__;
    mocks.android_glass_capabilities = () => ({ windowBlur: true });
    mocks.android_show_glass_menu = ({ payload }: any) => ({
      requestId: payload.requestId,
      action: 'object',
    });
  });
  await page.locator('.android-fab').click();
  await expect(page.getByTestId('object-destination-picker')).toBeVisible();
  await page
    .getByTestId('object-destination-picker')
    .getByRole('button', { name: 'Identity', exact: true })
    .click();
  await expect(page).toHaveURL('/editor?section=identity');
  await expect(page.getByTestId('object-save-destination')).toHaveText('Save to: Identity');
});
