import { expect, test, type Page } from '@playwright/test';
import { setupTauriMock, login } from './fixtures/auth';
import { pluginManifest } from '../src/test/pluginFixtures';

const longName = 'PublicLongFieldName_ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789'.repeat(3);
const longPage = 'PublicPage_ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789'.repeat(3);
const longContract = 'PublicContract_ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789'.repeat(3);

async function setup(page: Page, platform = 'android', theme = 'light', language = 'en-US') {
  await setupTauriMock(page);
  const plugin = pluginManifest({
    id: 'public-plugin',
    name: 'Public Plugin',
    contracts: [
      {
        typeId: 'public.contract',
        version: 1,
        displayName: longContract,
        strictContractGate: false,
        typeIdAliases: [],
        roles: [{ roleId: 'name', label: longName, required: false, defaultPropertyId: 'f0' }],
      },
    ],
  });
  await page.addInitScript(
    ({ platform, theme, language, longName, longPage, plugin }) => {
      const templates = [
        {
          id: 'public-template',
          accountId: 'e2e-account',
          name: 'Public Template',
          category: 'identity',
          iconId: 'document',
          createdAt: '2026-10-22',
          contractTypeId: 'public.contract',
          properties: Array.from({ length: 18 }, (_, index) => ({
            id: `f${index}`,
            name: index === 0 ? longName : `Field ${index}`,
            type: index === 1 ? 'select' : 'text',
            sensitivityLevel: 'internal',
            options: index === 1 ? ['Alpha', 'Beta'] : undefined,
            contractField: index === 0,
            deprecatedAt: index === 2 ? '2026-10-01' : undefined,
          })),
        },
      ];
      const customPages = [
        {
          id: 'custom-active',
          name: longPage,
          iconId: 'document',
          createdAt: '2026-10-22',
          sortOrder: 0,
        },
        {
          id: 'custom-deleted',
          name: 'Deleted Public Page',
          iconId: 'document',
          createdAt: '2026-10-22',
          sortOrder: 1,
          deletedAt: '2026-10-22',
        },
      ];
      const prefs = {
        theme,
        language,
        hasSeenOnboarding: true,
        autoLockTimeoutMinutes: 0,
        reduceMotion: true,
        backupReminderDays: 0,
        customPages,
      };
      const writes: unknown[] = [];
      Object.assign(window, {
        __MOCK_PLATFORM__: platform,
        __TEMPLATE_WRITES__: writes,
        __E2E_MOCKS__: {
          ui_get_preferences: () => prefs,
          user_data_get_preferences: () => prefs,
          vault_check_directory: () => true,
          template_list: () => templates,
          template_check_field_usage: ({ fieldKey }: { fieldKey: string }) => ({
            active: fieldKey === 'f0' ? 3 : 0,
            softDeleted: 0,
          }),
          template_update: (args: object) => {
            writes.push(args);
          },
          template_create: (args: object) => {
            writes.push(args);
            return 'created-template';
          },
          plugin_list_installed: () => [plugin],
          sync_list_conflicts: () => [],
          object_list: ({ filter }: { filter?: { typeId?: string } }) =>
            filter?.typeId === 'page'
              ? customPages.map((page) => ({
                  id: page.id,
                  accountId: 'e2e-account',
                  name: page.name,
                  typeId: 'page',
                  iconName: page.iconId,
                  properties: { sortOrder: page.sortOrder },
                  createdAt: page.createdAt,
                  updatedAt: page.deletedAt ?? page.createdAt,
                  isDeleted: Boolean(page.deletedAt),
                  sensitivityLevel: 'internal',
                }))
              : [],
        },
      });
      localStorage.setItem('i18nextLng', language);
    },
    { platform, theme, language, longName, longPage, plugin },
  );
  await login(page);
  await page.evaluate(() => {
    history.pushState(
      { ...history.state, idx: (history.state?.idx ?? 0) + 1 },
      '',
      '/settings/templates',
    );
    dispatchEvent(new PopStateEvent('popstate', { state: history.state }));
  });
  await expect(page.locator('[data-ui-card]').filter({ hasText: 'Public Template' })).toBeVisible();
}

async function openEdit(page: Page) {
  await page
    .locator('[data-ui-card]')
    .filter({ hasText: 'Public Template' })
    .getByRole('button', { name: /^(Edit|编辑)$/ })
    .click();
  await expect(page.locator('[data-android-template-editor]')).toBeVisible();
}

async function writeCount(page: Page) {
  return page.evaluate(
    () => (window as unknown as { __TEMPLATE_WRITES__: unknown[] }).__TEMPLATE_WRITES__.length,
  );
}

async function noHorizontalOverflow(page: Page, selector: string) {
  const geometry = await page.locator(selector).evaluate((element) => ({
    width: element.clientWidth,
    scrollWidth: element.scrollWidth,
    left: element.getBoundingClientRect().left,
    right: element.getBoundingClientRect().right,
    viewport: innerWidth,
    controls: [...element.querySelectorAll<HTMLElement>('button,input,textarea')].map((node) => ({
      left: node.getBoundingClientRect().left,
      right: node.getBoundingClientRect().right,
    })),
  }));
  expect(geometry.scrollWidth).toBeLessThanOrEqual(geometry.width + 1);
  for (const control of geometry.controls) {
    expect(control.left).toBeGreaterThanOrEqual(geometry.left - 1);
    expect(control.right).toBeLessThanOrEqual(geometry.right + 1);
    expect(control.right).toBeLessThanOrEqual(geometry.viewport + 1);
  }
}

for (const theme of ['light', 'dark']) {
  for (const viewport of [
    { width: 320, height: 568 },
    { width: 390, height: 844 },
    { width: 640, height: 320 },
  ]) {
    test(`Android ${theme} ${viewport.width}x${viewport.height}: full screen and independent picker scroll`, async ({
      page,
    }) => {
      await page.setViewportSize(viewport);
      await setup(page, 'android', theme, viewport.width === 320 ? 'zh-CN' : 'en-US');
      await openEdit(page);
      const editor = page.locator('[data-android-template-editor]');
      const bounds = (await editor.boundingBox())!;
      expect(bounds).toEqual({ x: 0, y: 0, ...viewport });
      await noHorizontalOverflow(page, '[data-android-template-editor]');
      await page.screenshot({
        path: test.info().outputPath('editor-top.png'),
        animations: 'disabled',
      });
      const header = editor.locator('header');
      const before = await header.boundingBox();
      await editor.locator('[data-template-scroll]').evaluate((element) => {
        element.scrollTop = element.scrollHeight;
      });
      expect(await header.boundingBox()).toEqual(before);
      await expect(editor.getByRole('button', { name: /^(Save|保存)$/ })).toBeInViewport();
      await editor.getByRole('button', { name: /^(Add Field|添加字段)$/ }).click();
      const sheet = page.locator('.android-sheet');
      await expect(sheet).toBeVisible();
      await noHorizontalOverflow(page, '.android-sheet');
      const heading = sheet.locator('header');
      const headingBefore = await heading.boundingBox();
      await sheet.locator('[data-template-sheet-scroll]').evaluate((element) => {
        element.scrollTop = element.scrollHeight;
      });
      expect(await heading.boundingBox()).toEqual(headingBefore);
      await expect(sheet.getByRole('radio', { name: /^(File|文件)$/ })).toBeInViewport();
      await page.screenshot({ path: test.info().outputPath('picker.png'), animations: 'disabled' });
      await page.keyboard.press('Escape');
      await expect(sheet).toBeHidden();
      await page.screenshot({ path: test.info().outputPath('editor.png'), animations: 'disabled' });
      expect(await writeCount(page)).toBe(0);
    });
  }
}

test('Android type selection, sensitivity, add and page choice save the correct draft', async ({
  page,
}) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await setup(page);
  await openEdit(page);
  const editor = page.locator('[data-android-template-editor]');
  const field = editor.locator('[data-template-field="f0"]');
  await field.getByRole('textbox').fill('Edited name');
  await field.getByRole('button', { name: /Field type/ }).click();
  await page.getByRole('radio', { name: 'Number', exact: true }).click();
  await field.getByRole('button', { name: /Sensitivity/ }).click();
  await page.getByRole('radio', { name: 'Critical', exact: true }).click();
  await editor.getByRole('button', { name: /Page.*Identity/ }).click();
  await expect(page.getByRole('radio', { name: 'Deleted Public Page' })).toBeDisabled();
  await page.getByRole('radio', { name: longPage, exact: true }).click();
  await editor.getByRole('button', { name: 'Add Field', exact: true }).click();
  await page.getByRole('radio', { name: 'Date', exact: true }).click();
  const added = editor.locator('[data-template-field-name]').last();
  await expect(added).toBeFocused();
  await added.fill('New date');
  await editor.getByRole('button', { name: 'Save', exact: true }).click();
  await expect(editor).toBeHidden();
  const writes = await page.evaluate(
    () =>
      (
        window as unknown as {
          __TEMPLATE_WRITES__: {
            category: string;
            properties: { id: string; name: string; type: string; sensitivityLevel: string }[];
          }[];
        }
      ).__TEMPLATE_WRITES__,
  );
  expect(writes).toHaveLength(1);
  expect(writes[0].category).toBe('custom-active');
  expect(writes[0].properties.find((prop) => prop.id === 'f0')).toMatchObject({
    name: 'Edited name',
    type: 'number',
    sensitivityLevel: 'critical',
  });
  expect(writes[0].properties.at(-1)).toMatchObject({ name: 'New date', type: 'date' });
});

test('Android options cancel, deduplicate, and back preserve the template draft', async ({
  page,
}) => {
  await setup(page);
  await openEdit(page);
  const editor = page.locator('[data-android-template-editor]');
  const field = editor.locator('[data-template-field="f1"]');
  await editor.getByRole('textbox', { name: 'Name', exact: true }).fill('Unsaved title');
  await field.getByRole('button', { name: /Edit options/ }).click();
  await page.locator('.android-sheet textarea').fill('Beta\nGamma\nBeta\n\n');
  await page.locator('.android-sheet').getByRole('button', { name: 'Cancel', exact: true }).click();
  await field.getByRole('button', { name: /Edit options/ }).click();
  await expect(page.locator('.android-sheet textarea')).toHaveValue('Alpha\nBeta');
  await page.locator('.android-sheet textarea').fill('Beta\nGamma\nBeta\n\n');
  await page
    .locator('.android-sheet')
    .getByRole('button', { name: 'Confirm', exact: true })
    .click();
  await expect(field.getByRole('button', { name: /Edit options \(2\)/ })).toBeVisible();
  await field.getByRole('button', { name: /Edit options/ }).click();
  await expect(page.locator('.android-sheet textarea')).toHaveValue('Beta\nGamma');
  await page.evaluate(() => history.back());
  await expect(page.locator('.android-sheet')).toBeHidden();
  await expect(editor.getByRole('textbox', { name: 'Name', exact: true })).toHaveValue(
    'Unsaved title',
  );
  await editor.getByRole('button', { name: 'Save', exact: true }).click();
  await expect.poll(() => writeCount(page)).toBe(1);
  const saved = await page.evaluate(
    () =>
      (
        window as unknown as {
          __TEMPLATE_WRITES__: { properties: { id: string; options: string[] }[] }[];
        }
      ).__TEMPLATE_WRITES__[0],
  );
  expect(saved.properties.find((prop) => prop.id === 'f1')!.options).toEqual(['Beta', 'Gamma']);
});

test('Android whole-editor cancel and hardware back never persist edits', async ({ page }) => {
  await setup(page);
  await openEdit(page);
  const editor = page.locator('[data-android-template-editor]');
  await editor.getByRole('textbox', { name: 'Name', exact: true }).fill('Discard me');
  await page.evaluate(() => history.back());
  await expect(editor).toBeHidden();
  expect(await writeCount(page)).toBe(0);
  await expect(page.locator('#root')).not.toHaveAttribute('inert');
  await openEdit(page);
  await expect(editor.getByRole('textbox', { name: 'Name', exact: true })).toHaveValue(
    'Public Template',
  );
  await editor.getByRole('button', { name: 'Cancel', exact: true }).click();
  expect(await writeCount(page)).toBe(0);
});

test('Android new template validates the name and cancels type choice without adding a field', async ({
  page,
}) => {
  await setup(page);
  await page.getByRole('button', { name: 'New Template', exact: true }).click();
  const editor = page.locator('[data-android-template-editor]');
  await editor.getByRole('button', { name: 'Save', exact: true }).click();
  await expect(editor.getByText('Name is required', { exact: true }).last()).toBeVisible();
  expect(await writeCount(page)).toBe(0);
  await editor.getByRole('textbox', { name: 'Name', exact: true }).fill('New public template');
  await editor.getByRole('button', { name: 'Add Field', exact: true }).click();
  await page.keyboard.press('Escape');
  await expect(editor.locator('[data-template-field]')).toHaveCount(0);
  await editor.getByRole('button', { name: 'Add Field', exact: true }).click();
  await page.getByRole('radio', { name: 'Email', exact: true }).click();
  await expect(editor.locator('[data-template-field-name]')).toBeFocused();
  await editor.getByRole('button', { name: 'Save', exact: true }).click();
  await expect.poll(() => writeCount(page)).toBe(1);
  const saved = await page.evaluate(
    () =>
      (
        window as unknown as {
          __TEMPLATE_WRITES__: { name: string; properties: { type: string }[] }[];
        }
      ).__TEMPLATE_WRITES__[0],
  );
  expect(saved.name).toBe('New public template');
  expect(saved.properties).toEqual([expect.objectContaining({ type: 'email' })]);
});

test('Android a rejected save keeps the full draft for retry', async ({ page }) => {
  await setup(page);
  await openEdit(page);
  const editor = page.locator('[data-android-template-editor]');
  await editor.getByRole('textbox', { name: 'Name', exact: true }).fill('Retry this draft');
  await page.evaluate(() => {
    const target = window as unknown as { __E2E_MOCKS__: Record<string, unknown> };
    target.__E2E_MOCKS__.template_update = () => {
      throw new Error('Public save failure');
    };
  });
  await editor.getByRole('button', { name: 'Save', exact: true }).click();
  await expect(editor.getByText(/Public save failure/)).toBeVisible();
  await expect(editor.getByRole('textbox', { name: 'Name', exact: true })).toHaveValue(
    'Retry this draft',
  );
  expect(await writeCount(page)).toBe(0);
});

test('Android external navigation with a picker open releases the background', async ({ page }) => {
  await setup(page);
  await openEdit(page);
  await page
    .locator('[data-android-template-editor]')
    .getByRole('button', { name: 'Add Field', exact: true })
    .click();
  await expect(page.locator('.android-sheet')).toBeVisible();
  await page.evaluate(() => {
    history.pushState({ idx: (history.state?.idx ?? 0) + 1 }, '', '/settings');
    dispatchEvent(new PopStateEvent('popstate', { state: history.state }));
  });
  await expect(page.locator('[data-android-template-editor]')).toHaveCount(0);
  await expect(page.locator('.android-sheet')).toHaveCount(0);
  await expect(page.locator('#root')).not.toHaveAttribute('inert');
  await expect(page).toHaveURL('/settings');
  expect(await writeCount(page)).toBe(0);
});

test('Android used fields retain the archive confirmation and can be restored', async ({
  page,
}) => {
  await setup(page);
  await openEdit(page);
  const editor = page.locator('[data-android-template-editor]');
  const field = editor.locator('[data-template-field="f0"]');
  await field.getByRole('button', { name: /^Remove:/ }).click();
  const confirm = page
    .getByRole('dialog')
    .filter({ has: page.getByRole('button', { name: 'Confirm', exact: true }) });
  await expect(confirm).toBeVisible();
  await page.evaluate(() => history.back());
  await expect(confirm).toBeHidden();
  await expect(field).toBeVisible();
  await field.getByRole('button', { name: /^Remove:/ }).click();
  await confirm.getByRole('button', { name: 'Confirm', exact: true }).click();
  await expect(field).toHaveCount(0);
  await editor.getByRole('button', { name: /Deprecated/ }).click();
  const archived = editor
    .locator('div')
    .filter({ has: page.locator('span', { hasText: longName }) })
    .filter({ has: page.getByRole('button', { name: 'Restore', exact: true }) })
    .last();
  await expect(archived.getByRole('button', { name: 'Clean up', exact: true })).toHaveCount(0);
  await archived.getByRole('button', { name: 'Restore', exact: true }).click();
  await expect(field).toBeVisible();
  await editor.getByRole('button', { name: 'Save', exact: true }).click();
  await expect.poll(() => writeCount(page)).toBe(1);
});

test('Android plugin selection uses one sheet and nested back keeps editing', async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 568 });
  await setup(page);
  await openEdit(page);
  const editor = page.locator('[data-android-template-editor]');
  await editor
    .locator('[data-template-field="f1"]')
    .getByRole('button', { name: 'Plugin Binding', exact: true })
    .click();
  const sheet = page.locator('.android-sheet');
  await sheet.getByRole('button', { name: /Select plugin contract/ }).click();
  await expect(sheet).toHaveCount(1);
  await noHorizontalOverflow(page, '.android-sheet');
  await page.getByRole('radio', { name: new RegExp(longContract) }).click();
  await sheet.getByRole('button', { name: /Select role/ }).click();
  await page.evaluate(() => history.back());
  await expect(sheet.getByRole('button', { name: /Select role/ })).toBeVisible();
  await sheet.getByRole('button', { name: /Select role/ }).click();
  await page.getByRole('radio', { name: longName, exact: true }).click();
  await sheet.getByRole('button', { name: 'Add', exact: true }).click();
  await expect(sheet.getByRole('button', { name: 'Add', exact: true })).toBeDisabled();
  await sheet.getByRole('button', { name: 'Close', exact: true }).click();
  await expect(editor).toBeVisible();
  await editor.getByRole('button', { name: 'Save', exact: true }).click();
  await expect.poll(() => writeCount(page)).toBe(1);
  const bindings = await page.evaluate(
    () =>
      (
        window as unknown as {
          __TEMPLATE_WRITES__: { properties: { id: string; contractBindings: unknown[] }[] }[];
        }
      ).__TEMPLATE_WRITES__[0].properties.find((prop) => prop.id === 'f1')!.contractBindings,
  );
  expect(bindings).toEqual([{ contractTypeId: 'public.contract', roleId: 'name' }]);
});

test('Android dynamic fields preserve the final allowed type and save limits', async ({ page }) => {
  await setup(page);
  await openEdit(page);
  const editor = page.locator('[data-android-template-editor]');
  await editor.getByRole('checkbox', { name: /Enable dynamic/i }).check();
  await editor.getByRole('button', { name: 'Dynamic field settings', exact: true }).click();
  const sheet = page.locator('.android-sheet');
  const boxes = sheet.getByRole('checkbox');
  for (let index = 1; index < (await boxes.count()); index++) await boxes.nth(index).uncheck();
  await expect(boxes.first()).toBeDisabled();
  await sheet.getByRole('spinbutton').fill('7');
  await sheet.getByRole('radio', { name: 'Sensitive', exact: true }).click();
  await sheet.getByRole('button', { name: 'Close', exact: true }).click();
  await editor.getByRole('button', { name: 'Save', exact: true }).click();
  await expect.poll(() => writeCount(page)).toBe(1);
  const dynamic = await page.evaluate(() =>
    (
      window as unknown as { __TEMPLATE_WRITES__: { properties: { type: string }[] }[] }
    ).__TEMPLATE_WRITES__[0].properties.find((prop) => prop.type === 'dynamic_group'),
  );
  expect(dynamic).toMatchObject({
    allowedTypes: ['text'],
    maxItems: 7,
    sensitivityLevel: 'sensitive',
  });
});

test('Android short viewport, safe areas and large text keep save and choices reachable', async ({
  page,
}) => {
  await page.setViewportSize({ width: 320, height: 400 });
  await setup(page);
  await page.evaluate(() => {
    const root = document.documentElement;
    root.style.fontSize = '24px';
    for (const edge of ['top', 'bottom', 'left', 'right'])
      root.style.setProperty(`--android-native-safe-area-${edge}`, '16px');
    root.style.setProperty('--android-keyboard-inset', '100px');
  });
  await openEdit(page);
  await noHorizontalOverflow(page, '[data-android-template-editor]');
  const editor = page.locator('[data-android-template-editor]');
  const save = (await editor.getByRole('button', { name: 'Save', exact: true }).boundingBox())!;
  expect(save.y).toBeGreaterThanOrEqual(16);
  expect(save.y + save.height).toBeLessThan(284);
  await editor.getByRole('button', { name: 'Add Field', exact: true }).click();
  await noHorizontalOverflow(page, '.android-sheet');
  await page.getByRole('radio', { name: 'File', exact: true }).click();
  await expect(editor.locator('[data-template-field-name]').last()).toBeFocused();
});

for (const platform of ['windows', 'macos']) {
  for (const theme of ['light', 'dark']) {
    test(`${platform} ${theme}: desktop keeps its dialog and inline field controls`, async ({
      page,
    }) => {
      await page.setViewportSize({ width: 1100, height: 800 });
      await setup(page, platform, theme);
      await page
        .locator('[data-ui-card]')
        .filter({ hasText: 'Public Template' })
        .getByRole('button', { name: 'Edit', exact: true })
        .click();
      await expect(page.locator('[data-android-template-editor]')).toHaveCount(0);
      const dialog = page.getByRole('dialog');
      await expect(dialog).toBeVisible();
      expect((await dialog.boundingBox())!.width).toBeLessThanOrEqual(480);
      const selects = dialog.locator('select');
      await expect(selects.first()).toHaveValue('identity');
      await selects.last().selectOption('date');
      await dialog.getByRole('button', { name: 'Add Field', exact: true }).click();
      await dialog.getByRole('button', { name: 'Save', exact: true }).click();
      await expect.poll(() => writeCount(page)).toBe(1);
      const last = await page.evaluate(() =>
        (
          window as unknown as { __TEMPLATE_WRITES__: { properties: { type: string }[] }[] }
        ).__TEMPLATE_WRITES__[0].properties.at(-1),
      );
      expect(last!.type).toBe('date');
    });
  }
}
