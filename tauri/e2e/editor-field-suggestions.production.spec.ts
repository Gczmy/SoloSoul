import { expect, test } from '@playwright/test';
import { login, setupTauriMock } from './fixtures/auth';

type SuggestionProbe = Window & {
  holdSuggestions: boolean;
  pendingSuggestions: { id: string; finish: (fail?: boolean) => void }[];
  pendingObjects: { id: string; finish: () => void }[];
  suggestionCalls: string[];
  stopSuggestionFrames: () => { name: string; sources: string[] }[];
};

for (const platform of ['windows', 'macos'] as const)
  for (const theme of ['light', 'dark'] as const) {
    test(`${platform} ${theme}: field recommendations switch with the complete editor snapshot`, async ({
      page,
    }, info) => {
      await setupTauriMock(page);
      await page.setViewportSize({ width: 1280, height: 900 });
      const errors: string[] = [];
      page.on('pageerror', (error) => errors.push(error.message));
      await page.addInitScript(
        ({ platform, theme }) => {
          const probe = window as unknown as SuggestionProbe;
          probe.holdSuggestions = false;
          probe.pendingSuggestions = [];
          probe.pendingObjects = [];
          probe.suggestionCalls = [];
          const objects = ['A', 'B', 'C'].map((id) => ({
            id,
            accountId: 'e2e-account',
            name: `Object ${id}`,
            typeId: 'identity',
            sensitivityLevel: 'public',
            createdAt: '2026-10-22',
            updatedAt: '2026-10-22',
            properties: {
              personName: `Name ${id}`,
              __fields: { personName: { name: '姓名', type: 'text', sensitivityLevel: 'public' } },
            },
          }));
          const prefs = {
            theme,
            language: 'en-US',
            hasSeenOnboarding: true,
            autoLockTimeoutMinutes: 0,
          };
          Object.assign(window, {
            __MOCK_PLATFORM__: platform,
            __E2E_MOCKS__: {
              ui_get_preferences: () => prefs,
              user_data_get_preferences: () => prefs,
              vault_check_directory: () => true,
              ocr_get_model_status: () => ({ installed: true, bundled: true }),
              sync_list_conflicts: () => [],
              attachment_count_batch: () => ({}),
              template_list: () => [],
              object_list: ({ filter }: { filter?: { typeId?: string } }) =>
                filter?.typeId === 'page' ? [] : objects,
              object_get: async ({ objectId }: { objectId: string }) => {
                if (probe.holdSuggestions)
                  await new Promise<void>((resolve) => {
                    probe.pendingObjects.push({ id: objectId, finish: resolve });
                  });
                return objects.find((object) => object.id === objectId);
              },
              object_field_suggestions: async ({
                excludeObjectId,
              }: {
                excludeObjectId: string;
              }) => {
                probe.suggestionCalls.push(excludeObjectId);
                if (probe.holdSuggestions)
                  await new Promise<void>((resolve, reject) => {
                    probe.pendingSuggestions.push({
                      id: excludeObjectId,
                      finish: (fail) =>
                        fail ? reject(new Error('Fixture recommendation failure')) : resolve(),
                    });
                  });
                return objects
                  .filter((object) => object.id !== excludeObjectId)
                  .map((object) => ({
                    objectId: object.id,
                    objectName: object.name,
                    fieldKey: 'personName',
                    fieldName: '姓名',
                    value: `Recommended ${object.id}`,
                    sensitivityLevel: object.id === 'C' ? 'sensitive' : 'public',
                  }));
              },
            },
          });
          localStorage.setItem('i18nextLng', 'en-US');
        },
        { platform, theme },
      );
      await login(page);
      const select = (id: string) =>
        page.locator(`#desktop-page-pane [data-object-id="${id}"]`).click();
      const name = page.getByLabel('Object Name', { exact: true });
      const suggestions = page.getByTestId('field-suggestions');
      const form = page.locator('[data-object-editor-form]');
      const ready = async (id: string) => {
        await expect(name).toHaveValue(`Object ${id}`);
        await expect(form).not.toHaveAttribute('inert');
        await expect(suggestions).toBeVisible();
        await expect(suggestions.getByTestId('field-suggestion-item')).toHaveCount(2);
        for (const other of ['A', 'B', 'C'].filter((other) => other !== id)) {
          await expect(suggestions.getByText(`Object ${other}`, { exact: true })).toBeVisible();
        }
        await expect(suggestions.getByText(`Object ${id}`, { exact: true })).toHaveCount(0);
      };
      const pending = async (id: string, count: number) => {
        await select(id);
        await expect
          .poll(() =>
            page.evaluate(() => (window as unknown as SuggestionProbe).pendingSuggestions.length),
          )
          .toBe(count);
        await expect
          .poll(() =>
            page.evaluate(() => (window as unknown as SuggestionProbe).pendingObjects.length),
          )
          .toBe(count);
      };
      const release = (kind: 'object' | 'suggestion', index: number, fail = false) =>
        page.evaluate(
          ({ kind, index, fail }) => {
            const probe = window as unknown as SuggestionProbe;
            if (kind === 'object') probe.pendingObjects[index].finish();
            else probe.pendingSuggestions[index].finish(fail);
          },
          { kind, index, fail },
        );

      await select('A');
      await ready('A');
      // Verify that switching objects does not carry revealed recommendation values over.
      await suggestions
        .getByTestId('field-suggestion-item')
        .filter({ hasText: 'Object C' })
        .click();
      await expect(suggestions.getByText('Recommended C', { exact: true })).toBeVisible();
      await page.evaluate(() => {
        const probe = window as unknown as SuggestionProbe;
        probe.holdSuggestions = true;
        let stopped = false;
        const frames: { name: string; sources: string[] }[] = [];
        const sample = () => {
          if (stopped) return;
          frames.push({
            name: (
              document.querySelector(
                '[data-object-editor-form] input[aria-label="Object Name"]',
              ) as HTMLInputElement
            )?.value,
            sources: [
              ...document.querySelectorAll(
                '[data-testid="field-suggestion-item"] > span:first-child',
              ),
            ]
              .map((el) => el.textContent ?? '')
              .sort(),
          });
          requestAnimationFrame(sample);
        };
        requestAnimationFrame(sample);
        probe.stopSuggestionFrames = () => {
          stopped = true;
          return frames;
        };
      });
      // Object first: the old, inert editor remains complete while recommendations are pending.
      await pending('B', 1);
      await release('object', 0);
      await expect(name).toHaveValue('Object A');
      await expect(form).toHaveAttribute('inert', '');
      await expect(suggestions).toBeVisible();
      await release('suggestion', 0);
      await ready('B');
      await expect(suggestions.getByText('Recommended C', { exact: true })).toHaveCount(0);
      await suggestions.getByTestId('field-suggestion-fill').first().click();
      await expect(page.locator('input[id="personName"]')).toHaveValue('Recommended A');
      // Recommendations first: no early replacement of the displayed object's recommendation list.
      await pending('A', 2);
      await release('suggestion', 1);
      await expect(name).toHaveValue('Object B');
      await expect(suggestions.getByText('Object A', { exact: true })).toBeVisible();
      await release('object', 1);
      await ready('A');
      // Skip C and complete B; late C results must never appear in the editor.
      await pending('C', 3);
      await release('object', 2);
      await pending('B', 4);
      await release('suggestion', 3);
      await release('object', 3);
      await ready('B');
      await release('suggestion', 2);
      await ready('B');
      const calls = await page.evaluate(
        () => (window as unknown as SuggestionProbe).suggestionCalls.length,
      );
      await select('B');
      expect(
        await page.evaluate(() => (window as unknown as SuggestionProbe).suggestionCalls.length),
      ).toBe(calls);
      const frames = await page.evaluate(() =>
        (window as unknown as SuggestionProbe).stopSuggestionFrames(),
      );
      expect(frames.length).toBeGreaterThan(8);
      for (const frame of frames) {
        expect(frame.sources).toEqual(
          ['A', 'B', 'C'].map((id) => `Object ${id}`).filter((source) => source !== frame.name),
        );
      }
      await info.attach('recommendation-frame-samples', {
        body: JSON.stringify({ platform, theme, frames }),
        contentType: 'application/json',
      });
      await page.screenshot({ path: info.outputPath(`${platform}-${theme}-recommendations.png`) });
      // An optional recommendation failure must not retain another object's values or prevent editing.
      await pending('A', 5);
      await release('suggestion', 4, true);
      await release('object', 4);
      await expect(name).toHaveValue('Object A');
      await expect(form).not.toHaveAttribute('inert');
      await expect(suggestions).toHaveCount(0);
      expect(errors).toEqual([]);
    });
  }
