import { expect, test } from '@playwright/test';
import { login, setupTauriMock } from './fixtures/auth';

for (const platform of ['windows', 'macos']) {
  for (const theme of ['light', 'dark']) {
    test(`${platform} ${theme}: existing editor saves in place without duplicate writes`, async ({
      page,
    }) => {
      await page.setViewportSize({ width: 1200, height: 850 });
      await setupTauriMock(page);
      await page.addInitScript(
        ({ platform, theme }) => {
          const objects = ['A', 'B'].map((id) => ({
            id: `object-${id}`,
            accountId: 'e2e-account',
            name: `Public ${id}`,
            typeId: 'identity',
            templateId: 'public-template',
            properties: { title: `Public text ${id}`, __editLocked: false },
            sensitivityLevel: 'public',
            createdAt: '2026-10-23',
            updatedAt: '2026-10-23',
          }));
          const writes: string[] = [];
          const preferences = {
            theme,
            language: 'en-US',
            hasSeenOnboarding: true,
            autoLockTimeoutMinutes: 0,
          };
          let frame = false;
          Object.assign(window, {
            __MOCK_PLATFORM__: platform,
            __EDITOR_WRITES__: writes,
            __E2E_MOCKS__: {
              ui_get_preferences: () => preferences,
              user_data_get_preferences: () => preferences,
              vault_check_directory: () => true,
              ocr_get_model_status: () => ({ installed: true, bundled: true }),
              sync_list_conflicts: () => [],
              object_list: ({ filter }: { filter?: { typeId?: string } }) =>
                filter?.typeId === 'page' ? [] : objects,
              object_get: ({ objectId }: { objectId: string }) =>
                objects.find((object) => object.id === objectId),
              object_field_suggestions: () => [],
              attachment_count_batch: () => ({}),
              template_list: () => [
                {
                  id: 'public-template',
                  accountId: 'e2e-account',
                  name: 'Public template',
                  category: 'identity',
                  createdAt: '2026-10-23',
                  properties: [
                    { id: 'title', name: 'Public field', type: 'text', sensitivityLevel: 'public' },
                  ],
                },
              ],
              object_update: ({
                objectId,
                input,
              }: {
                objectId: string;
                input: { name: string; properties: Record<string, unknown> };
              }) => {
                const object = objects.find((object) => object.id === objectId)!;
                object.name = input.name;
                Object.assign(object.properties, input.properties);
                object.updatedAt = new Date().toISOString();
                writes.push(objectId);
                return object;
              },
              set_titlebar_color: () => ({
                platform,
                material: platform === 'windows' ? 'mica' : 'none',
                reduceMotion: false,
                highContrast: false,
              }),
              set_titlebar_controls: ({ windowsFrame }: { windowsFrame?: boolean }) => {
                if (windowsFrame !== undefined) frame = windowsFrame;
              },
              get_window_layout: () => ({
                platform,
                windowsFrame:
                  platform === 'windows' && frame
                    ? {
                        enabled: true,
                        headerHeight: 52,
                        dpi: 96,
                        maximized: false,
                        captionBounds: [innerWidth - 146, 0, 146, 30],
                      }
                    : undefined,
              }),
            },
          });
          localStorage.setItem('i18nextLng', 'en-US');
        },
        { platform, theme },
      );
      await login(page);
      await page.evaluate(() => {
        history.pushState(null, '', '/editor/object-A');
        dispatchEvent(new PopStateEvent('popstate'));
      });
      const name = page.getByLabel('Object Name', { exact: true });
      const field = page.locator('input#title');
      const save = page.getByRole('button', { name: 'Save', exact: true });
      await expect(field).toHaveValue('Public text A');
      const form = await page.locator('[data-object-editor-form]').elementHandle();
      await name.fill('Saved public A');
      await field.fill('Saved public field');
      await save.click();
      await expect(save).toBeEnabled();
      await expect(page).toHaveURL('/editor/object-A');
      await expect(name).toHaveValue('Saved public A');
      await expect(field).toHaveValue('Saved public field');
      expect(
        await form!.evaluate(
          (node) => node === document.querySelector('[data-object-editor-form]'),
        ),
      ).toBe(true);
      await save.click();
      await expect(save).toBeEnabled();
      expect(
        await page.evaluate(
          () => (window as unknown as { __EDITOR_WRITES__: string[] }).__EDITOR_WRITES__,
        ),
      ).toEqual(['object-A']);
      await page.getByRole('button', { name: 'Cancel', exact: true }).click();
      await expect(page).not.toHaveURL('/editor/object-A');
    });
  }
}
