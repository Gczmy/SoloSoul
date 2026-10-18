import { expect, test, type Page } from '@playwright/test';
import { setupTauriMock, login } from './fixtures/auth';
import { measureControlContrast } from './fixtures/contrast';

async function setupWindowsObjects(page: Page, theme: string) {
  await setupTauriMock(page);
  await page.addInitScript((theme) => {
    let frame = false;
    const writes: unknown[] = [];
    const fields = Array.from({ length: 12 }, (_, i) => ({
      id: `field-${i}`,
      name: `Public field ${i}`,
      type: 'text',
      sensitivityLevel: 'public',
    }));
    const object = {
      id: 'chrome-object',
      accountId: 'e2e-account',
      name: 'Public chrome object',
      typeId: 'identity',
      templateId: 'public-template',
      properties: Object.fromEntries(fields.map((field) => [field.id, 'Public value'])),
      sensitivityLevel: 'public',
      propertyLabels: { note: 'public' },
      createdAt: '2026-10-14',
      updatedAt: '2026-10-14',
    };
    const preferences = {
      theme,
      language: 'en-US',
      hasSeenOnboarding: true,
      autoLockTimeoutMinutes: 0,
    };
    Object.assign(window, {
      __MOCK_PLATFORM__: 'windows',
      __FRAME_CONTROLS__: [],
      __OBJECT_WRITES__: writes,
      __E2E_MOCKS__: {
        ui_get_preferences: () => preferences,
        user_data_get_preferences: () => preferences,
        vault_check_directory: () => true,
        ocr_get_model_status: () => ({ installed: true, bundled: true }),
        sync_list_conflicts: () => [],
        object_list: ({ filter }: { filter?: { typeId?: string; parentId?: string } }) =>
          filter?.parentId || filter?.typeId === 'page' ? [] : [object],
        object_get: () => object,
        template_list: () => [
          {
            id: 'public-template',
            accountId: 'e2e-account',
            name: 'Public template',
            category: 'identity',
            properties: fields,
            createdAt: '2026-10-14',
          },
        ],
        object_field_suggestions: () => [],
        object_update: async (args: { objectId: string; input: { name: string } }) => {
          writes.push(args);
          await new Promise<void>((resolve) => Object.assign(window, { __FINISH_SAVE__: resolve }));
          object.name = args.input.name;
          return object;
        },
        attachment_count_batch: () => ({ 'chrome-object': 0 }),
        set_titlebar_color: () => ({
          platform: 'windows',
          material: 'mica',
          reduceMotion: false,
          highContrast: false,
        }),
        set_titlebar_controls: (args: { windowsFrame?: boolean; regions: unknown[] }) => {
          if (args.windowsFrame !== undefined) frame = args.windowsFrame;
          Object.assign(window, { __FRAME_CONTROLS__: args.regions });
        },
        get_window_layout: () => ({
          platform: 'windows',
          windowsFrame: frame
            ? {
                enabled: true,
                headerHeight: 52,
                dpi: 96,
                maximized: false,
                captionBounds: [window.innerWidth - 146, 0, 146, 30],
              }
            : undefined,
        }),
      },
    });
    localStorage.setItem('i18nextLng', 'en-US');
  }, theme);
  await login(page);
  await expect(page.locator('html')).toHaveAttribute('data-windows-frame', 'true');
  await page
    .getByRole('button', { name: /^Identity/ })
    .first()
    .click();
}

test.use({ viewport: { width: 1100, height: 800 }, hasTouch: false, isMobile: false });

for (const theme of ['light', 'dark']) {
  for (const width of [800, 1100]) {
    test(`Windows ${theme} ${width}px: editor actions stay in the titlebar and preserve save/cancel`, async ({
      page,
    }, testInfo) => {
      test.skip(testInfo.project.name !== 'chromium', 'Windows desktop layout');
      await page.setViewportSize({ width, height: 800 });
      await setupWindowsObjects(page, theme);
      const openEditor = async () => {
        await page
          .getByRole('button', { name: /^Public chrome object/ })
          .first()
          .click();
        await page
          .getByTestId('object-detail-modal')
          .getByRole('button', { name: 'Edit', exact: true })
          .click();
        await expect(page.getByLabel('Object Name', { exact: true })).toHaveValue(
          'Public chrome object',
        );
      };
      await openEditor();
      const header = page.locator('[data-appbar]');
      const save = header.getByRole('button', { name: 'Save', exact: true });
      const cancel = header.getByRole('button', { name: 'Cancel', exact: true });
      await expect(save).toBeVisible();
      await expect(cancel).toBeVisible();
      const idle = await measureControlContrast(save, '[data-appbar]');
      expect(idle.foregroundContrast).toBeGreaterThanOrEqual(4.5);
      if (theme === 'light') await expect(save).toHaveCSS('color', 'rgb(255, 255, 255)');
      await save.hover();
      await expect
        .poll(async () => (await measureControlContrast(save, '[data-appbar]')).foregroundContrast)
        .toBeGreaterThanOrEqual(4.5);
      if (theme === 'light') await expect(save).toHaveCSS('color', 'rgb(255, 255, 255)');
      await page.screenshot({ path: testInfo.outputPath('editor-primary-actions.png') });
      await expect(
        page.locator('[data-shell-content]').getByRole('button', { name: /^(Save|Cancel)$/ }),
      ).toHaveCount(0);
      await page.getByLabel('Object Name', { exact: true }).fill('Cancelled draft');
      await page.locator('[data-shell-content]').evaluate((el) => {
        el.scrollTop = el.scrollHeight;
      });
      for (const button of [save, cancel]) {
        const rect = (await button.boundingBox())!;
        expect(rect.y).toBeGreaterThanOrEqual(0);
        expect(rect.y + rect.height).toBeLessThanOrEqual(52);
        expect(rect.x + rect.width).toBeLessThanOrEqual(width - 146);
        expect(
          await button.evaluate((el) => {
            const r = el.getBoundingClientRect();
            return el.contains(document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2));
          }),
        ).toBe(true);
        await expect
          .poll(() =>
            button.evaluate((el) => {
              const r = el.getBoundingClientRect();
              const controls = (
                window as unknown as {
                  __FRAME_CONTROLS__: { x: number; y: number; width: number; height: number }[];
                }
              ).__FRAME_CONTROLS__;
              return controls.some(
                (c) =>
                  c.x <= r.x &&
                  c.y <= r.y &&
                  c.x + c.width >= r.right - 1 &&
                  c.y + c.height >= r.bottom - 1,
              );
            }),
          )
          .toBe(true);
      }
      await page.screenshot({ path: testInfo.outputPath('editor-titlebar-actions.png') });
      await cancel.click();
      await expect(page).toHaveURL('/workspace?section=identity');
      expect(
        await page.evaluate(
          () => (window as unknown as { __OBJECT_WRITES__: unknown[] }).__OBJECT_WRITES__,
        ),
      ).toEqual([]);
      await expect(header.getByRole('button', { name: 'Save', exact: true })).toHaveCount(0);
      await openEditor();
      await page.getByLabel('Object Name', { exact: true }).fill('Saved public object');
      await save.click();
      await expect(save).toBeDisabled();
      await expect
        .poll(() =>
          page.evaluate(
            () => (window as unknown as { __OBJECT_WRITES__: unknown[] }).__OBJECT_WRITES__,
          ),
        )
        .toMatchObject([{ objectId: 'chrome-object', input: { name: 'Saved public object' } }]);
      await page.evaluate(() =>
        (window as unknown as { __FINISH_SAVE__: () => void }).__FINISH_SAVE__(),
      );
      await expect(page).toHaveURL('/workspace?section=identity');
      await expect(header.getByRole('button', { name: 'Save', exact: true })).toHaveCount(0);
    });
  }
}
