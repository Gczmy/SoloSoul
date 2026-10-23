import { expect, test } from '@playwright/test';
import { login, setupTauriMock } from './fixtures/auth';

for (const platform of ['windows', 'macos', 'android']) {
  for (const theme of ['light', 'dark']) {
    test(`${platform} ${theme}: edit lock keeps content readable and follows object navigation`, async ({
      page,
    }, info) => {
      test.skip(
        (platform === 'android') !== (info.project.name === 'mobile'),
        'Use the matching platform viewport',
      );
      if (platform !== 'android') await page.setViewportSize({ width: 1100, height: 800 });
      await setupTauriMock(page);
      await page.addInitScript(
        ({ platform, theme }) => {
          const objects = ['A', 'B'].map((id) => ({
            id: `object-${id}`,
            accountId: 'e2e-account',
            name: `Public ${id}`,
            typeId: 'identity',
            templateId: 'public-template',
            properties: {
              title: `Public text ${id}`,
              choice: 'A',
              __editLocked: id === 'A',
              __attachments: [{ id: 'attachment', fileName: 'public.txt' }],
            },
            sensitivityLevel: 'public',
            createdAt: '2026-10-23',
            updatedAt: '2026-10-23',
          }));
          const calls: string[] = [];
          const preferences = {
            theme,
            language: 'en-US',
            hasSeenOnboarding: true,
            autoLockTimeoutMinutes: 0,
          };
          let frame = false;
          Object.assign(window, {
            __MOCK_PLATFORM__: platform,
            __LOCK_OBJECTS__: objects,
            __LOCK_CALLS__: calls,
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
              object_field_suggestions: () => [
                {
                  objectId: 'public-source',
                  objectName: 'Public source',
                  fieldKey: 'title',
                  fieldName: 'Public field',
                  sensitivityLevel: 'public',
                  value: 'Public recommendation',
                },
              ],
              attachment_count_batch: () => ({ 'object-A': 1, 'object-B': 1 }),
              template_list: () => [
                {
                  id: 'public-template',
                  accountId: 'e2e-account',
                  name:
                    platform === 'android'
                      ? 'Public template with a long name for narrow screen verification'
                      : 'Public template',
                  category: 'identity',
                  createdAt: '2026-10-23',
                  properties: [
                    { id: 'title', name: 'Public field', type: 'text', sensitivityLevel: 'public' },
                    {
                      id: 'choice',
                      name: 'Public choice',
                      type: 'select',
                      options: ['A', 'B'],
                      sensitivityLevel: 'public',
                    },
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
                if (object.properties.__editLocked)
                  throw { code: 'OBJECT_LOCKED', safeDetails: null, retryable: false };
                calls.push('save');
                object.name = input.name;
                Object.assign(object.properties, input.properties);
                return object;
              },
              object_set_locked: ({ objectId, locked }: { objectId: string; locked: boolean }) => {
                const object = objects.find((object) => object.id === objectId)!;
                object.properties.__editLocked = locked;
                object.updatedAt = new Date().toISOString();
                calls.push(locked ? 'lock' : 'unlock');
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
                        captionBounds: [window.innerWidth - 146, 0, 146, 30],
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
      if (platform === 'android') {
        await page.locator('.android-navigation a[href="/workspace"]').click();
        const card = page.locator('.android-object-row').filter({ hasText: 'Public A' });
        const otherCard = page.locator('.android-object-row').filter({ hasText: 'Public B' });
        const trigger = card.getByRole('button', { name: 'Actions for Public A' });
        const sheet = page.getByRole('dialog', { name: 'Public A', exact: true });
        for (const width of [320, 390]) {
          await page.setViewportSize({ width, height: 844 });
          for (const [row, label] of [
            [card, 'Public A'],
            [otherCard, 'Public B'],
          ] as const) {
            await row.locator('.android-row-button').click();
            const detail = page.getByRole('dialog', { name: label, exact: true });
            const status = detail.locator('[data-object-edit-lock]');
            const metadata = detail.getByText(/^Identity/);
            await expect(metadata).toContainText('Public template with a long name');
            const statusBox = (await status.boundingBox())!;
            const metadataBox = (await metadata.boundingBox())!;
            expect(metadataBox.y).toBeGreaterThanOrEqual(statusBox.y + statusBox.height + 3);
            const close = detail.getByTestId('object-detail-close');
            const closeBox = (await close.boundingBox())!;
            expect(metadataBox.x + metadataBox.width).toBeLessThanOrEqual(closeBox.x);
            for (const date of [detail.getByText(/^created:/), detail.getByText(/^updated:/)]) {
              const box = (await date.boundingBox())!;
              const lines = await date.evaluate((el) => {
                const range = document.createRange();
                range.selectNodeContents(el);
                return new Set(Array.from(range.getClientRects(), (rect) => rect.top)).size;
              });
              expect(lines).toBe(1);
              expect(box.x + box.width).toBeLessThanOrEqual(closeBox.x);
            }
            expect(await detail.evaluate((el) => el.scrollWidth <= el.clientWidth + 1)).toBe(true);
            if (width === 320 && label === 'Public B')
              await page.screenshot({
                path: info.outputPath(`android-${theme}-detail-metadata.png`),
              });
            await close.click();
            await expect(detail).toHaveCount(0);
          }
        }
        await trigger.click();
        await expect(sheet.getByRole('button', { name: 'Delete', exact: true })).toBeDisabled();
        await sheet.getByRole('button', { name: 'Unlock', exact: true }).click();
        await expect(sheet).toHaveCount(0);
        await expect(card.locator('[data-object-edit-lock]')).toHaveAttribute(
          'data-object-edit-lock',
          'editable',
        );
        await expect(otherCard.locator('[data-object-edit-lock]')).toHaveAttribute(
          'data-object-edit-lock',
          'editable',
        );
        await expect(page).toHaveURL('/workspace');
        await trigger.click();
        await expect(sheet.getByRole('button', { name: 'Delete', exact: true })).toBeEnabled();
        await sheet.evaluate(async (panel) => {
          await Promise.all(panel.getAnimations().map((animation) => animation.finished));
        });
        await page.screenshot({ path: info.outputPath(`android-${theme}-lock-menu.png`) });
        await sheet.getByRole('button', { name: 'Lock', exact: true }).click();
        await expect(sheet).toHaveCount(0);
        await expect(card.locator('[data-object-edit-lock]')).toHaveAttribute(
          'data-object-edit-lock',
          'locked',
        );
        expect(await page.evaluate(() => Reflect.get(window, '__LOCK_CALLS__'))).toEqual([
          'unlock',
          'lock',
        ]);
        await page.evaluate(() => {
          (Reflect.get(window, '__LOCK_CALLS__') as string[]).length = 0;
        });
      }
      const openObject = async (id: string) => {
        await page.evaluate((id) => {
          history.pushState(null, '', `/editor/object-${id}`);
          dispatchEvent(new PopStateEvent('popstate'));
        }, id);
        await expect(page.getByLabel('Object Name', { exact: true })).toHaveValue(`Public ${id}`);
      };
      await openObject('A');
      const field = page.locator('input#title');
      await expect(field).toHaveValue('Public text A');
      await expect(field).toHaveAttribute('readonly', '');
      await expect(page.getByTestId('field-suggestions')).toHaveCount(0);
      await expect(page.getByRole('button', { name: 'Unlock', exact: true })).toBeEnabled();
      await expect(page.getByRole('button', { name: 'Save', exact: true })).toHaveCount(0);
      await expect(page.getByRole('button', { name: 'Cancel', exact: true })).toHaveCount(0);
      expect(
        await field.evaluate((el: HTMLInputElement) => {
          el.select();
          return el.selectionEnd! - el.selectionStart!;
        }),
      ).toBe('Public text A'.length);
      await page.getByRole('button', { name: 'Unlock', exact: true }).click();
      await expect(field).not.toHaveAttribute('readonly', '');
      await expect(page.getByRole('button', { name: 'Save', exact: true })).toBeEnabled();
      await expect(page.getByTestId('field-suggestions')).toBeVisible();
      await expect(
        page.locator('[data-object-editor-form] [data-object-edit-lock]'),
      ).toHaveAttribute('data-object-edit-lock', 'editable');
      if (platform !== 'android') {
        await expect(
          page.locator('[data-page-pane] [data-object-id="object-A"] [data-object-edit-lock]'),
        ).toHaveCount(0);
        await page.locator('[data-page-pane] [data-object-id="object-A"]').hover();
        const preview = page.locator('[data-page-pane-preview]');
        await expect(preview).toBeVisible();
        await expect(preview.locator('[data-object-edit-lock]')).toHaveAttribute(
          'data-object-edit-lock',
          'editable',
        );
        await page.keyboard.press('Escape');
        await expect(preview).toHaveCount(0);
      }
      await field.fill('Edited public text');
      await page.getByRole('button', { name: 'Lock', exact: true }).click();
      await expect(page.getByRole('button', { name: 'Unlock', exact: true })).toBeEnabled();
      await expect(page.getByTestId('field-suggestions')).toHaveCount(0);
      await expect(field).toHaveAttribute('readonly', '');
      await expect(field).toHaveValue('Edited public text');
      expect(await page.evaluate(() => Reflect.get(window, '__LOCK_CALLS__'))).toEqual([
        'unlock',
        'save',
        'lock',
      ]);
      if (platform !== 'android') {
        const status = page.locator(
          '[data-page-pane] [data-object-id="object-A"] [data-object-edit-lock]',
        );
        await expect(status).toHaveAttribute('data-object-edit-lock', 'locked');
      }
      await page.screenshot({ path: info.outputPath(`${platform}-${theme}-locked.png`) });
      await openObject('B');
      await expect(field).not.toHaveAttribute('readonly', '');
      await openObject('A');
      await expect(field).toHaveAttribute('readonly', '');
      await expect(field).toHaveValue('Edited public text');
    });
  }
}
