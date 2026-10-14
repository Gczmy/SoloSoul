import { expect, test } from '@playwright/test';
import { login, setupTauriMock } from './fixtures/auth';

test.use({ viewport: { width: 1100, height: 800 }, hasTouch: false, isMobile: false });

for (const theme of ['light', 'dark']) {
  test(`Windows ${theme}: real object details synchronize and release the native caption mask`, async ({
    page,
  }, testInfo) => {
    test.skip(testInfo.project.name !== 'chromium', 'Windows desktop layout');
    await setupTauriMock(page);
    await page.addInitScript((theme) => {
      let frame = false;
      const object = {
        id: 'public-mask',
        accountId: 'e2e-account',
        name: 'Public masked object',
        typeId: 'identity',
        properties: { note: 'Public detail', __fields: { note: { name: 'Note', type: 'text' } } },
        propertyLabels: { note: 'public' },
        sensitivityLevel: 'public',
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
        __CAPTION_BACKDROP__: null,
        __E2E_MOCKS__: {
          ui_get_preferences: () => preferences,
          user_data_get_preferences: () => preferences,
          vault_check_directory: () => true,
          ocr_get_model_status: () => ({ installed: true, bundled: true }),
          sync_list_conflicts: () => [],
          object_list: ({ filter }: { filter?: { typeId?: string; parentId?: string } }) =>
            filter?.parentId || filter?.typeId === 'page' ? [] : [object],
          object_get: () => object,
          attachment_count_batch: () => ({ 'public-mask': 0 }),
          set_titlebar_color: () => ({
            platform: 'windows',
            material: 'mica',
            reduceMotion: false,
            highContrast: false,
          }),
          set_titlebar_controls: (args: { windowsFrame?: boolean; backdrop?: unknown }) => {
            if (args.windowsFrame !== undefined) frame = args.windowsFrame;
            if (args.backdrop) Object.assign(window, { __CAPTION_BACKDROP__: args.backdrop });
          },
          get_window_layout: () => ({
            platform: 'windows',
            windowsFrame: frame
              ? {
                  enabled: true,
                  headerHeight: 52,
                  dpi: 96,
                  maximized: false,
                  fullWindowBackdrop: true,
                  captionBounds: [window.innerWidth - 146, 0, 146, 30],
                }
              : undefined,
          }),
        },
      });
      localStorage.setItem('i18nextLng', 'en-US');
    }, theme);
    await login(page);
    await page
      .getByRole('button', { name: /^Identity/ })
      .first()
      .click();
    await page
      .getByRole('button', { name: /^Public masked object/ })
      .first()
      .click();
    const detail = page.getByTestId('object-detail-modal');
    await expect(detail).toBeVisible();
    const expected = await detail.evaluate((el) => {
      const mask = el.closest('[data-macos-glass-backdrop]')!;
      const channels = getComputedStyle(mask)
        .outlineColor.match(/[\d.]+/g)!
        .map(Number);
      return {
        red: channels[0],
        green: channels[1],
        blue: channels[2],
        alpha: Math.round((channels[3] ?? 1) * 255),
      };
    });
    const actual = () =>
      page.evaluate(
        () => (window as unknown as { __CAPTION_BACKDROP__: unknown }).__CAPTION_BACKDROP__,
      );
    const colorOnly = async () => {
      const state = (await actual()) as typeof expected | null;
      return state && { red: state.red, green: state.green, blue: state.blue, alpha: state.alpha };
    };
    await expect.poll(colorOnly).toEqual(expected);
    expect(
      await detail.evaluate(
        (el) => getComputedStyle(el.closest('[data-macos-glass-backdrop]')!).backgroundColor,
      ),
    ).toBe('rgba(0, 0, 0, 0)');
    await expect
      .poll(async () => {
        const state = (await actual()) as {
          layers?: Array<{ fullWindow: boolean; exclusions: unknown[] }>;
        } | null;
        return state?.layers?.some((layer) => layer.fullWindow && layer.exclusions.length > 0);
      })
      .toBe(true);
    await page.setViewportSize({ width: 900, height: 650 });
    await expect.poll(colorOnly).toEqual(expected);
    await detail.getByRole('button', { name: 'Close', exact: true }).click();
    await expect(detail).toHaveCount(0);
    await expect.poll(actual).toEqual({ red: 0, green: 0, blue: 0, alpha: 0 });
  });
}
