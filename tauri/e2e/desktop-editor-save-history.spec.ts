import { expect, test } from '@playwright/test';
import { login, setupTauriMock } from './fixtures/auth';

for (const platform of ['windows', 'macos']) {
  for (const theme of ['light', 'dark']) {
    test(`${platform} ${theme}: saved edits refresh history in place without duplicate writes`, async ({
      page,
    }, info) => {
      test.skip(info.project.name !== 'chromium', 'Desktop layout');
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
          const reads: string[] = [];
          const snapshots = Object.fromEntries(
            objects.map((object) => [
              object.id,
              Array.from({ length: 24 }, (_, i) => ({
                id: `${object.id}-history-${i}`,
                timestamp: 1700000000000 + i * 86400000,
                triggeredBy: 'user_edit',
                diffSummary: 'diff_updated',
              })),
            ]),
          );
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
            __HISTORY_READS__: reads,
            __FAIL_NEXT_SAVE__: false,
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
              attachment_list: ({ objectId }: { objectId: string }) => [
                {
                  id: `${objectId}-file`,
                  objectId,
                  fileName: `${objectId}-public.txt`,
                  sizeBytes: 42,
                  mimeType: 'text/plain',
                  createdAt: '2026-10-23',
                },
              ],
              snapshot_list: async ({ objectId }: { objectId: string }) => {
                reads.push(objectId);
                await new Promise((resolve) => setTimeout(resolve, 60));
                return snapshots[objectId];
              },
              snapshot_get_data: () => ({ name: 'Expanded public snapshot', properties: {} }),
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
                const testWindow = window as unknown as { __FAIL_NEXT_SAVE__: boolean };
                if (testWindow.__FAIL_NEXT_SAVE__) {
                  testWindow.__FAIL_NEXT_SAVE__ = false;
                  throw { code: 'INTERNAL_ERROR', safeDetails: null, retryable: true };
                }
                const object = objects.find((object) => object.id === objectId)!;
                object.name = input.name;
                Object.assign(object.properties, input.properties);
                object.updatedAt = new Date().toISOString();
                writes.push(objectId);
                snapshots[objectId].push({
                  id: `${objectId}-saved-${writes.length}`,
                  timestamp: Date.now(),
                  triggeredBy: 'user_edit',
                  diffSummary: 'diff_updated',
                });
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
      await page.locator('[data-object-inspector-toggle]').click();
      const panel = page.locator('[data-object-inspector]');
      const historyTab = panel.getByRole('tab', { name: 'History', exact: true });
      await historyTab.click();
      const rows = panel.locator('[data-history-snapshot]');
      await expect(rows).toHaveCount(20);
      const oldRow = rows.first();
      const oldId = await oldRow.getAttribute('data-history-snapshot');
      await oldRow.click();
      await expect(panel.getByText('Expanded public snapshot')).toBeVisible();
      const list = panel.locator('[data-object-inspector-scroll]');
      await list.evaluate((el) => {
        el.scrollTop = 150;
      });
      const content = await panel.locator('[data-inspector-content]').elementHandle();
      const form = await page.locator('[data-object-editor-form]').elementHandle();
      await name.fill('Saved public A');
      await field.fill('Saved public field');
      await save.click();
      await expect(save).toBeEnabled();
      await expect(page).toHaveURL('/editor/object-A');
      await expect(name).toHaveValue('Saved public A');
      await expect(field).toHaveValue('Saved public field');
      await expect(rows.first()).toHaveAttribute('data-history-snapshot', 'object-A-saved-1');
      await expect(panel.locator(`[data-history-snapshot="${oldId}"]`)).toHaveAttribute(
        'aria-expanded',
        'true',
      );
      expect(
        await content!.evaluate(
          (node) => node === document.querySelector('[data-inspector-content]'),
        ),
      ).toBe(true);
      expect(await list.evaluate((el) => el.scrollTop)).toBeGreaterThan(0);
      await page.screenshot({ path: info.outputPath('saved-history.png') });
      const readCount = await page.evaluate(
        () => (window as unknown as { __HISTORY_READS__: string[] }).__HISTORY_READS__.length,
      );
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
      expect(
        await page.evaluate(
          () => (window as unknown as { __HISTORY_READS__: string[] }).__HISTORY_READS__.length,
        ),
      ).toBe(readCount);
      await field.fill('Failed public draft');
      await page.evaluate(() => {
        (window as unknown as { __FAIL_NEXT_SAVE__: boolean }).__FAIL_NEXT_SAVE__ = true;
      });
      await save.click();
      await expect(save).toBeEnabled();
      await expect(field).toHaveValue('Failed public draft');
      expect(
        await page.evaluate(
          () => (window as unknown as { __HISTORY_READS__: string[] }).__HISTORY_READS__.length,
        ),
      ).toBe(readCount);
      await panel.getByRole('tab', { name: 'Attachments', exact: true }).click();
      await expect(panel.getByText('object-A-public.txt')).toBeVisible();
      await field.fill('Saved with attachments open');
      await save.click();
      await expect(save).toBeEnabled();
      await expect(panel.getByRole('tab', { name: 'Attachments', exact: true })).toHaveAttribute(
        'aria-selected',
        'true',
      );
      await expect(panel.getByText('object-A-public.txt')).toBeVisible();
      await historyTab.click();
      await expect(rows.first()).toHaveAttribute('data-history-snapshot', 'object-A-saved-2');
      await page.evaluate(() => {
        history.pushState(null, '', '/editor/object-B');
        dispatchEvent(new PopStateEvent('popstate'));
      });
      await expect(name).toHaveValue('Public B');
      await expect(rows.first()).toHaveAttribute('data-history-snapshot', 'object-B-history-23');
      await expect(panel.locator('[data-history-snapshot^="object-A"]')).toHaveCount(0);
      await page.getByRole('button', { name: 'Cancel', exact: true }).click();
      await expect(page).toHaveURL('/editor/object-A');
      await expect(name).toHaveValue('Saved public A');
      await page.locator('[data-object-inspector-toggle]').click();
      await expect(panel).toHaveCount(0);
      await field.fill('Saved with panel closed');
      await save.click();
      await expect(save).toBeEnabled();
      await expect(panel).toHaveCount(0);
      await page.locator('[data-object-inspector-toggle]').click();
      await expect(rows.first()).toHaveAttribute('data-history-snapshot', 'object-A-saved-3');
    });
  }
}
