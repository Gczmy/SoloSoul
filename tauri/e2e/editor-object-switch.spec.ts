import { test, expect } from '@playwright/test';
import { login, setupTauriMock } from './fixtures/auth';

type Probe = Window & {
  hold: boolean;
  reads: { id: string; finish: (missing?: boolean) => void }[];
  finishSave?: () => void;
  writes: { objectId: string; input: { name: string } }[];
  blankFrames: number;
  stopFrames: () => void;
  quick: boolean;
  quickFrames: { opacity: number; notice: boolean }[];
};

test.use({ viewport: { width: 800, height: 800 }, hasTouch: false, isMobile: false });

for (const platform of ['macos', 'windows'] as const) {
  for (const theme of ['light', 'dark']) {
    test(`${platform} ${theme}: switch objects without blank frames or stale writes`, async ({
      page,
    }, info) => {
      test.skip(info.project.name === 'mobile', 'Desktop pointer layout');
      await setupTauriMock(page);
      await page.addInitScript(
        ({ platform, theme }) => {
          const probe = window as unknown as Probe;
          Object.assign(probe, {
            hold: false,
            reads: [],
            writes: [],
            quick: false,
            quickFrames: [],
          });
          const templates = ['A', 'B'].map((id) => ({
            id: `template-${id}`,
            accountId: 'e2e-account',
            name: `Template ${id}`,
            category: 'identity',
            createdAt: '2026-10-21',
            properties: [
              { id: `field-${id}`, name: `Field ${id}`, type: 'text', sensitivityLevel: 'public' },
            ],
          }));
          const objects = ['A', 'B'].map((id) => ({
            id: `object-${id}`,
            accountId: 'e2e-account',
            name: `Object ${id}`,
            typeId: 'identity',
            templateId: `template-${id}`,
            templateType: 'user',
            properties: { [`field-${id}`]: `Value ${id}` },
            sensitivityLevel: 'public',
            createdAt: '2026-10-21',
            updatedAt: '2026-10-21',
          }));
          const preferences = {
            theme,
            language: 'en-US',
            hasSeenOnboarding: true,
            autoLockTimeoutMinutes: 0,
          };
          let frame = false;
          Object.assign(window, {
            __MOCK_PLATFORM__: platform,
            __E2E_MOCKS__: {
              ui_get_preferences: () => preferences,
              user_data_get_preferences: () => preferences,
              vault_check_directory: () => true,
              ocr_get_model_status: () => ({ installed: true, bundled: true }),
              sync_list_conflicts: () => [],
              object_field_suggestions: () => [],
              attachment_count_batch: () => ({}),
              template_list: () => templates,
              object_list: ({ filter }: { filter?: { typeId?: string } }) =>
                filter?.typeId === 'page' ? [] : objects,
              object_get: async ({ objectId }: { objectId: string }) => {
                let missing = false;
                if (probe.quick)
                  await new Promise<void>((resolve) =>
                    requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
                  );
                else if (probe.hold)
                  await new Promise<void>((resolve) => {
                    probe.reads.push({
                      id: objectId,
                      finish: (fail = false) => {
                        missing = fail;
                        resolve();
                      },
                    });
                  });
                return missing ? null : (objects.find((object) => object.id === objectId) ?? null);
              },
              object_update: async (args: { objectId: string; input: { name: string } }) => {
                probe.writes.push(args);
                await new Promise<void>((resolve) => {
                  probe.finishSave = resolve;
                });
                return { ...objects.find((object) => object.id === args.objectId)!, ...args.input };
              },
              set_titlebar_color: () => ({
                platform,
                material: platform === 'macos' ? 'liquid-glass' : 'mica',
                titlebarHeight: 52,
                trafficLightsRight: platform === 'macos' ? 79 : 0,
                reduceMotion: false,
                highContrast: false,
              }),
              set_titlebar_controls: (args: { windowsFrame?: boolean }) => {
                frame = args.windowsFrame ?? frame;
              },
              get_window_layout: () => ({
                platform,
                titlebarHeight: 52,
                trafficLightsRight: platform === 'macos' ? 79 : 0,
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
      const pane = page.locator('#desktop-page-pane');
      const name = page.getByLabel('Object Name', { exact: true });
      const save = page.locator('[data-appbar]').getByRole('button', { name: 'Save', exact: true });
      const cancel = page
        .locator('[data-appbar]')
        .getByRole('button', { name: 'Cancel', exact: true });
      const body = page.locator('[data-object-editor-form]');
      const select = (id: string) => pane.locator(`[data-object-id="object-${id}"]`).click();
      const release = (index: number, missing = false) =>
        page.evaluate(
          ({ index, missing }) => {
            (window as unknown as Probe).reads[index].finish(missing);
          },
          { index, missing },
        );
      const ready = async (id: string) => {
        await expect(name).toHaveValue(`Object ${id}`);
        await expect(page.locator(`input[id="field-${id}"]`)).toHaveValue(`Value ${id}`);
        await expect(page.getByText(`Template ${id}`, { exact: false })).toBeVisible();
        await expect(save).toBeEnabled();
        await expect(body).not.toHaveAttribute('inert');
      };
      const pending = async (id: string, count: number) => {
        await select(id);
        await expect
          .poll(() => page.evaluate(() => (window as unknown as Probe).reads.length))
          .toBe(count);
        await expect(save).toBeVisible();
        await expect(save).toHaveAttribute('data-switch-sentinel', 'preserved');
        await expect(save).toBeDisabled();
        await expect(cancel).toBeVisible();
        await expect(body).toHaveAttribute('inert', '');
        await expect(body).toHaveAttribute('aria-busy', 'true');
      };
      await select('A');
      await ready('A');
      await save.evaluate((button) => {
        button.setAttribute('data-switch-sentinel', 'preserved');
      });
      const positions = [await save.boundingBox(), await cancel.boundingBox()];
      // 按绘制帧检查切换过程，不只检查请求结束后的截图。
      await page.evaluate(() => {
        const probe = window as unknown as Probe;
        probe.hold = true;
        probe.blankFrames = 0;
        let stopped = false;
        const sample = () => {
          if (stopped) return;
          const buttons = [...document.querySelectorAll('[data-appbar] button')];
          if (
            !document.querySelector('[data-object-editor-form] input') ||
            !buttons.some((button) => button.textContent === 'Save') ||
            !buttons.some((button) => button.textContent === 'Cancel')
          )
            probe.blankFrames++;
          if (probe.quick) {
            const save = buttons.find((button) => button.textContent === 'Save');
            probe.quickFrames.push({
              opacity: save ? Number(getComputedStyle(save).opacity) : 0,
              notice: Boolean(document.querySelector('[data-editor-loading-notice]')),
            });
          }
          requestAnimationFrame(sample);
        };
        requestAnimationFrame(sample);
        probe.stopFrames = () => {
          stopped = true;
        };
      });
      await pending('B', 1);
      await expect(name).toHaveValue('Object A');
      expect(await save.boundingBox()).toEqual(positions[0]);
      expect(await cancel.boundingBox()).toEqual(positions[1]);
      await expect(page.locator('[data-editor-loading-notice]')).toBeVisible();
      await expect(save).toHaveCSS('opacity', '0.45');
      await page.screenshot({ path: info.outputPath('switch-pending.png') });
      await release(0);
      await ready('B');
      await expect(page.locator('input[id="field-A"]')).toHaveCount(0);
      // 返回已缓存对象也必须等待此次有效响应，不能拿旧缓存开放保存。
      await pending('A', 2);
      await expect(name).toHaveValue('Object B');
      await release(1);
      await ready('A');
      await pending('B', 3);
      await pending('A', 4);
      await release(3);
      await ready('A');
      await release(2);
      await expect(name).toHaveValue('Object A');
      await expect(page.locator('input[id="field-B"]')).toHaveCount(0);
      // A 的保存晚到时，不得导航离开当前 B 页面。
      await name.fill('Saved A draft');
      await save.click();
      await expect
        .poll(() => page.evaluate(() => (window as unknown as Probe).writes.length))
        .toBe(1);
      await pending('B', 5);
      await release(4);
      await ready('B');
      await page.evaluate(() => (window as unknown as Probe).finishSave!());
      await expect(page).toHaveURL('/editor/object-B');
      await ready('B');
      await pending('A', 6);
      await release(5, true);
      await expect(
        page.getByRole('alert').filter({ hasText: 'Failed to load object' }).first(),
      ).toBeVisible();
      await expect(save).toBeDisabled();
      await expect(body).toHaveAttribute('inert', '');
      await page.getByRole('button', { name: 'Retry', exact: true }).click();
      await expect
        .poll(() => page.evaluate(() => (window as unknown as Probe).reads.length))
        .toBe(7);
      await release(6);
      await ready('A');
      await expect(save).toHaveCSS('opacity', '1');
      await page.evaluate(() => {
        (window as unknown as Probe).quick = true;
      });
      for (const id of ['B', 'A', 'B', 'A']) {
        await select(id);
        await ready(id);
        await expect(save).toHaveAttribute('data-switch-sentinel', 'preserved');
      }
      await page.screenshot({ path: info.outputPath('switch-complete.png') });
      const result = await page.evaluate(() => {
        const probe = window as unknown as Probe;
        probe.stopFrames();
        return {
          blankFrames: probe.blankFrames,
          writes: probe.writes,
          quickFrames: probe.quickFrames,
        };
      });
      expect(result.blankFrames).toBe(0);
      expect(result.quickFrames.length).toBeGreaterThan(0);
      expect(result.quickFrames.every((frame) => frame.opacity === 1 && !frame.notice)).toBe(true);
      expect(result.writes).toMatchObject([
        { objectId: 'object-A', input: { name: 'Saved A draft' } },
      ]);
    });
  }
}
