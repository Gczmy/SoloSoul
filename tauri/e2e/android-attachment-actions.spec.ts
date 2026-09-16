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
    const object = {
      id: 'test-object',
      name: 'Travel documents',
      typeId: 'travel',
      createdAt: '2026-09-16',
      updatedAt: '2026-09-16',
      properties: {
        secret: 'FIELD-VALUE',
        __fields: { secret: { name: 'Secret', type: 'text', sensitivityLevel: 'sensitive' } },
      },
      propertyLabels: { secret: 'sensitive' },
    };
    const attachment = {
      id: 'report',
      objectId: object.id,
      fileName: 'Travel report.pdf',
      mimeType: 'application/pdf',
      sizeBytes: 1024,
      createdAt: '2026-09-16',
      vaultPath: '/mock-vault/report.pdf',
    };
    const deleted = {
      ...attachment,
      id: 'deleted',
      fileName: 'Old report.pdf',
      deletedAt: '2026-09-16',
    };
    const tree = (item: typeof attachment) => [
      {
        pageName: 'travel',
        objects: [{ objectId: object.id, objectName: object.name, attachments: [item] }],
      },
    ];
    const calls: { command: string; args: unknown }[] = [];
    const record = (command: string) => (args: unknown) => {
      calls.push({ command, args });
    };
    Object.assign(window, {
      __MOCK_PLATFORM__: 'android',
      __attachmentCalls: calls,
      __E2E_MOCKS__: {
        ui_get_preferences: () => prefs,
        user_data_get_preferences: () => prefs,
        vault_check_directory: () => true,
        object_list: ({ filter }: any) => (filter?.typeId === 'page' ? [] : [object]),
        object_get: () => object,
        attachment_count_stats: () => ({ attachmentCount: 1, photoCount: 0 }),
        attachment_count_batch: () => ({ [object.id]: 1 }),
        attachment_list: ({ showDeleted }: any) => [showDeleted ? deleted : attachment],
        attachment_list_all: () => ({ pages: tree(attachment), trashPages: tree(deleted) }),
        attachment_open: record('preview'),
        attachment_download: record('download'),
        attachment_share: record('share'),
        attachment_soft_delete: record('delete'),
        attachment_restore: record('restore'),
        attachment_permanent_delete: record('purge'),
        'plugin:dialog|save': () => '/mock-download/report.pdf',
        snapshot_list: () => [
          {
            id: 'snap-test',
            timestamp: Date.now(),
            triggeredBy: 'user_edit',
            diffSummary: 'diff_updated',
          },
        ],
        snapshot_get_data: () => object,
        snapshot_count_batch: () => ({ [object.id]: 1 }),
      },
    });
    localStorage.setItem('i18nextLng', 'en-US');
  });
  await login(page);
});

async function openMenu(page: Page, fileName = 'Travel report.pdf') {
  await page.getByRole('button', { name: `Attachment actions: ${fileName}`, exact: true }).click();
  const menu = page.getByRole('dialog', { name: 'More actions', exact: true });
  await expect(menu).toBeVisible();
  await expect(menu.getByText(fileName, { exact: true })).toBeVisible();
  return menu;
}

test('global attachment menus preserve actions, confirmations and the trash menu', async ({
  page,
}) => {
  await page.locator('.android-navigation a[href="/settings"]').click();
  await page.getByText('Global Attachment Manager', { exact: true }).click();
  const menu = await openMenu(page);
  await expect(menu.locator('.android-attachment-menu-actions button')).toHaveText([
    'Preview',
    'Download',
    'Forward',
    'Edit Attachment Attributes',
    'Delete',
  ]);
  await expect(page.locator('#root')).toHaveJSProperty('inert', true);
  for (const viewport of [
    { width: 320, height: 640 },
    { width: 844, height: 390 },
  ]) {
    await page.setViewportSize(viewport);
    await expect
      .poll(() =>
        menu.evaluate((panel) => {
          const rect = panel.getBoundingClientRect();
          return (
            rect.left >= 12 &&
            rect.right <= innerWidth - 12 &&
            rect.top >= 24 &&
            rect.bottom <= innerHeight - 6 &&
            panel.scrollWidth <= panel.clientWidth
          );
        }),
      )
      .toBe(true);
  }
  await page.setViewportSize({ width: 390, height: 844 });
  await page.screenshot({ path: test.info().outputPath('attachment-actions.png') });
  await menu.getByRole('button', { name: 'Preview', exact: true }).click();
  await expect(menu).toHaveCount(0);
  await expect
    .poll(() => page.evaluate(() => (window as any).__attachmentCalls))
    .toContainEqual({
      command: 'preview',
      args: { objectId: 'test-object', attachmentId: 'report' },
    });
  await (await openMenu(page)).getByRole('button', { name: 'Download', exact: true }).click();
  await expect
    .poll(() => page.evaluate(() => (window as any).__attachmentCalls))
    .toContainEqual({
      command: 'download',
      args: { srcPath: '/mock-vault/report.pdf', destPath: '/mock-download/report.pdf' },
    });
  await (await openMenu(page))
    .getByRole('button', { name: 'Edit Attachment Attributes', exact: true })
    .click();
  await expect(menu).toHaveCount(0);
  await expect(page.getByRole('dialog').getByLabel('Name', { exact: true })).toHaveValue(
    'Travel report.pdf',
  );
  await page.getByRole('dialog').getByRole('button', { name: 'Cancel', exact: true }).click();
  for (const action of ['Forward', 'Delete']) {
    await (await openMenu(page)).getByRole('button', { name: action, exact: true }).click();
    await expect(menu).toHaveCount(0);
    await expect(page.getByRole('dialog')).toBeVisible();
    await page.getByRole('dialog').getByRole('button', { name: 'Cancel', exact: true }).click();
  }
  await openMenu(page);
  await page.goBack();
  await expect(menu).toHaveCount(0);
  await expect(page).toHaveURL('/settings/attachments');
  await expect(page.locator('#root')).toHaveJSProperty('inert', false);
  await page.getByRole('button', { name: /^Trash/ }).click();
  const trashMenu = await openMenu(page, 'Old report.pdf');
  await expect(trashMenu.locator('.android-attachment-menu-actions button')).toHaveText([
    'Restore',
    'Delete Permanently',
  ]);
  await trashMenu.getByRole('button', { name: 'Delete Permanently', exact: true }).click();
  await expect(trashMenu).toHaveCount(0);
  await page.getByRole('dialog').getByRole('button', { name: 'Cancel', exact: true }).click();
  await (await openMenu(page, 'Old report.pdf'))
    .getByRole('button', { name: 'Restore', exact: true })
    .click();
  await expect
    .poll(() => page.evaluate(() => (window as any).__attachmentCalls))
    .toContainEqual({
      command: 'restore',
      args: { objectId: 'test-object', attachmentId: 'deleted' },
    });
  expect(
    await page.evaluate(() => (window as any).__attachmentCalls.map((call: any) => call.command)),
  ).toEqual(['preview', 'download', 'restore']);
});

test('object attachment menu stays above its host and closes without dismissing details', async ({
  page,
}) => {
  await page.locator('.android-navigation a[href="/workspace"]').click();
  await page.getByTestId('workspace-object-card').getByRole('button').first().click();
  const detail = page.getByTestId('object-detail-modal');
  await expect(detail).toBeVisible();
  await detail.getByRole('button', { name: 'Attachments (1)', exact: true }).click();
  const menu = await openMenu(page);
  await menu.getByRole('button', { name: 'Close', exact: true }).click();
  await expect(menu).toHaveCount(0);
  await expect(detail).toBeVisible();
  await (await openMenu(page))
    .getByRole('button', { name: 'Edit Attachment Attributes', exact: true })
    .click();
  await expect(page.getByRole('dialog').getByLabel('Name', { exact: true })).toHaveValue(
    'Travel report.pdf',
  );
  await page.getByRole('dialog').getByRole('button', { name: 'Cancel', exact: true }).click();
  await expect(detail).toBeVisible();
  await openMenu(page);
  await page.goBack();
  await expect(menu).toHaveCount(0);
  await expect(detail).toBeVisible();
  await expect(
    page.getByRole('button', { name: 'Attachment actions: Travel report.pdf', exact: true }),
  ).toBeVisible();
});

test('object detail keeps primary actions visible and groups secondary actions safely', async ({
  page,
}) => {
  await page.locator('.android-navigation a[href="/workspace"]').click();
  const openDetail = async () => {
    await page.getByTestId('workspace-object-card').getByRole('button').first().click();
    await expect(page.getByTestId('object-detail-modal')).toBeVisible();
  };
  await openDetail();
  const detail = page.getByTestId('object-detail-modal');
  const footer = detail.locator('.android-object-detail-footer');
  const more = detail.getByRole('button', { name: 'Actions for Travel documents', exact: true });
  const menu = page.getByRole('dialog', { name: 'More actions', exact: true });
  await expect(footer.getByRole('button')).toHaveCount(3);
  await expect(footer.getByRole('button', { name: 'Attachments (1)', exact: true })).toBeVisible();
  await expect(footer.getByText('Edit', { exact: true })).toBeVisible();
  for (const viewport of [
    { width: 320, height: 640 },
    { width: 390, height: 844 },
    { width: 844, height: 390 },
  ]) {
    await page.setViewportSize(viewport);
    await expect
      .poll(() =>
        footer.evaluate((element) => {
          const rect = element.getBoundingClientRect();
          return (
            rect.left >= 0 &&
            rect.right <= innerWidth &&
            element.scrollWidth <= element.clientWidth &&
            [...element.querySelectorAll('button')].every((button) => {
              const bounds = button.getBoundingClientRect();
              return (
                bounds.width >= 48 &&
                bounds.height >= 48 &&
                bounds.left >= rect.left &&
                bounds.right <= rect.right &&
                button.scrollWidth <= button.clientWidth
              );
            })
          );
        }),
      )
      .toBe(true);
  }
  await page.setViewportSize({ width: 390, height: 844 });
  await page.screenshot({
    path: test.info().outputPath('object-detail-footer.png'),
    animations: 'disabled',
  });
  await more.click();
  await expect(menu.locator('.android-object-menu-actions button')).toHaveText([
    'History',
    'Guide',
    'Delete',
  ]);
  await expect(menu.getByText('Travel documents', { exact: true })).toBeVisible();
  await page.screenshot({
    path: test.info().outputPath('object-detail-menu.png'),
    animations: 'disabled',
  });
  await page.goBack();
  await expect(menu).toHaveCount(0);
  await expect(detail).toBeVisible();
  await expect(more).toBeFocused();

  await more.click();
  await menu.getByRole('button', { name: 'Guide', exact: true }).click();
  const guide = page.getByRole('dialog', { name: 'Object Detail Card', exact: true });
  await expect(menu).toHaveCount(0);
  await expect(guide).toBeVisible();
  await expect(guide.getByText(/Open the … menu for History/)).toBeVisible();
  await expect(page.locator('#root')).toHaveJSProperty('inert', true);
  await page.goBack();
  await expect(guide).toHaveCount(0);
  await expect(detail).toBeVisible();
  await expect(page.locator('#root')).toHaveJSProperty('inert', false);

  await more.click();
  await menu.getByRole('button', { name: 'History', exact: true }).click();
  await expect(menu).toHaveCount(0);
  await expect(page.getByText(/^Version #1 ·/)).toBeVisible();
  await page.getByRole('button', { name: 'Close', exact: true }).last().click();
  await expect(detail).toBeVisible();

  await more.click();
  await menu.getByRole('button', { name: 'Delete', exact: true }).click();
  await expect(menu).toHaveCount(0);
  const confirmation = page.getByRole('dialog').filter({
    has: page.getByRole('heading', { name: 'Delete Object', exact: true }),
  });
  await expect(confirmation).toBeVisible();
  await confirmation.getByRole('button', { name: 'Cancel', exact: true }).click();
  await expect(page.getByTestId('workspace-object-card')).toHaveCount(1);
  await openDetail();
  await footer.getByRole('button', { name: 'Edit', exact: true }).click();
  await expect(page).toHaveURL('/editor/test-object');
});

test('object attachment count refreshes after deleting an attachment', async ({ page }) => {
  await page.evaluate(() => {
    const mocks = (window as any).__E2E_MOCKS__;
    const list = mocks.attachment_list;
    const remove = mocks.attachment_soft_delete;
    let deleted = false;
    mocks.attachment_count_batch = () => ({ 'test-object': deleted ? 0 : 1 });
    mocks.attachment_list = (args: any) => (deleted && !args.showDeleted ? [] : list(args));
    mocks.attachment_soft_delete = (args: any) => {
      remove(args);
      deleted = true;
    };
  });
  await page.locator('.android-navigation a[href="/workspace"]').click();
  await page.getByTestId('workspace-object-card').getByRole('button').first().click();
  const detail = page.getByTestId('object-detail-modal');
  await detail.getByRole('button', { name: 'Attachments (1)', exact: true }).click();
  await (await openMenu(page)).getByRole('button', { name: 'Delete', exact: true }).click();
  const confirmation = page.getByRole('dialog').filter({ hasText: 'Travel report.pdf' });
  await confirmation.getByRole('button', { name: 'Delete', exact: true }).click();
  await expect(detail.getByRole('button', { name: 'Attachments (0)', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Close', exact: true }).last().click();
  await expect(detail).toBeVisible();
  await expect(detail.locator('.android-object-attachment-count')).toHaveText('0');
});
