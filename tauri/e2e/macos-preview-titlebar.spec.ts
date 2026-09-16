import { test, expect, type Locator, type Page } from '@playwright/test';
import { setupTauriMock, login } from './fixtures/auth';

async function setupPreview(page: Page, platform = 'macos', kind = 'text', theme = 'light') {
  await setupTauriMock(page);
  await page.addInitScript(
    ({ platform, kind, theme }) => {
      const prefs = {
        theme,
        language: 'en-US',
        hasSeenOnboarding: true,
        autoLockTimeoutMinutes: 0,
      };
      const photo = kind === 'image';
      const fileName = photo
        ? 'Travel photo.png'
        : kind === 'pdf'
          ? 'Travel report.pdf'
          : 'A long attachment name for checking title truncation in a narrow window.txt';
      const attachment = {
        id: 'preview-file',
        objectId: 'preview-object',
        fileName,
        mimeType: photo ? 'image/png' : kind === 'pdf' ? 'application/pdf' : 'text/plain',
        sizeBytes: 128,
        createdAt: '2026-09-16',
        vaultPath: `/mock-vault/${fileName}`,
      };
      const imageUrl =
        'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+aG6sAAAAASUVORK5CYII=';
      const layout = {
        platform,
        titlebarHeight: platform === 'macos' ? 52 : 0,
        trafficLightsRight: platform === 'macos' ? 79 : 0,
      };
      Object.assign(window, {
        __MOCK_PLATFORM__: platform,
        __previewLayout: layout,
        __previewAppearanceCalls: 0,
        __E2E_MOCKS__: {
          ui_get_preferences: () => prefs,
          user_data_get_preferences: () => prefs,
          vault_check_directory: () => true,
          set_titlebar_color: () => {
            (window as any).__previewAppearanceCalls++;
            return {
              ...layout,
              material: platform === 'macos' ? 'liquid-glass' : 'solid',
              reduceMotion: false,
              highContrast: false,
            };
          },
          get_window_layout: () => layout,
          set_titlebar_controls: ({ regions }: any) => {
            (window as any).__previewRegions = regions;
          },
          attachment_count_stats: () => ({ attachmentCount: 1, photoCount: photo ? 1 : 0 }),
          attachment_list: () => [attachment],
          object_get: () => ({
            id: attachment.objectId,
            name: 'Travel documents',
            typeId: 'travel',
            properties: {},
          }),
          attachment_list_all: () => ({
            pages: [
              {
                pageName: 'travel',
                objects: [
                  {
                    objectId: attachment.objectId,
                    objectName: 'Travel documents',
                    attachments: [attachment],
                  },
                ],
              },
            ],
            trashPages: [],
          }),
          fs_read_file_as_text: () => 'Preview test content',
          fs_read_file_as_data_url: () =>
            kind === 'pdf' ? `data:application/pdf;base64,${btoa('%PDF-1.4\n%%EOF')}` : imageUrl,
          fs_read_image_preview: () => imageUrl,
        },
      });
      localStorage.setItem('i18nextLng', 'en-US');
    },
    { platform, kind, theme },
  );
  await login(page);
  await page.evaluate(() => {
    history.pushState(
      { ...history.state, idx: history.state.idx + 1, usr: null },
      '',
      '/settings/attachments',
    );
    window.dispatchEvent(new PopStateEvent('popstate', { state: history.state }));
  });
  await expect(page.getByRole('heading', { level: 1 })).toHaveText('Global Attachment Manager');
}

async function expectNativeControls(header: Locator) {
  await expect
    .poll(() =>
      header.evaluate((element) => {
        const regions = (window as any).__previewRegions || [];
        return Array.from(element.querySelectorAll('button'))
          .filter(
            (button) =>
              button.getClientRects().length > 0 &&
              getComputedStyle(button).visibility !== 'hidden' &&
              !button.closest('[inert]'),
          )
          .every((button) => {
            const r = button.getBoundingClientRect();
            return regions.some(
              (region: any) =>
                r.x >= region.x - 1 &&
                r.right <= region.x + region.width + 1 &&
                r.y >= region.y - 1 &&
                r.bottom <= region.y + region.height + 1,
            );
          });
      }),
    )
    .toBe(true);
}

async function expectPreviewTitlebar(header: Locator, trafficLightsRight: number) {
  await expect(header).toBeVisible();
  await expect(header).toHaveAttribute('data-tauri-drag-region', 'false');
  const r = (await header.boundingBox())!;
  expect(r.y).toBe(0);
  expect(r.height).toBe(52);
  const buttons = await header.locator('button').all();
  for (const button of buttons) {
    const rect = (await button.boundingBox())!;
    expect(rect.x).toBeGreaterThanOrEqual(trafficLightsRight ? trafficLightsRight + 12 : 14);
    expect(rect.y).toBeGreaterThanOrEqual(0);
    expect(rect.y + rect.height).toBeLessThanOrEqual(52);
    expect(rect.x + rect.width).toBeLessThanOrEqual(r.width);
  }
  expect((await buttons[0].boundingBox())!.x).toBe(
    trafficLightsRight ? trafficLightsRight + 12 : 14,
  );
  await expect(header).toHaveCSS('background-color', 'rgba(0, 0, 0, 0)');
  expect(
    await header.evaluate((element) => {
      const window = element.closest('[data-preview-window]')!;
      const surface = getComputedStyle(window, '::before');
      return {
        windowFill: getComputedStyle(window).backgroundColor,
        windowBlur: getComputedStyle(window).backdropFilter,
        surfaceTop: surface.top,
        // 原页面与多层遮罩不能从透明的整行顶栏透出；不只检查交通灯一角。
        pagePaintsInTitlebar: [4, innerWidth / 2, innerWidth - 4].some((x) =>
          document
            .elementsFromPoint(x, 4)
            .some((el) => el.closest('#root') || el.matches('[data-macos-glass-backdrop]')),
        ),
      };
    }),
  ).toEqual({
    windowFill: 'rgba(0, 0, 0, 0)',
    windowBlur: 'none',
    surfaceTop: '52px',
    pagePaintsInTitlebar: false,
  });
  await expectNativeControls(header);
}

// 排除原页面正文在顶栏下沿投出的 3px 阴影；预览时该阴影应一起消失。
const trafficLightsClip = { x: 0, y: 0, width: 79, height: 48 };

async function expectWholeRowGlass(page: Page) {
  // 浏览器模拟的原生底色为均匀色：跨越侧栏分界、中央和右沿应完全连续。
  // 采样顶部 4px，避开文件名和按钮；真实 macOS 桌面玻璃可呈现不同背景色。
  const reference = await page.screenshot({ clip: { x: 0, y: 0, width: 8, height: 4 } });
  for (const x of [
    232,
    Math.floor(page.viewportSize()!.width / 2),
    page.viewportSize()!.width - 8,
  ]) {
    expect(await page.screenshot({ clip: { x, y: 0, width: 8, height: 4 } })).toEqual(reference);
  }
}

for (const theme of ['light', 'dark']) {
  for (const kind of ['text', 'pdf', 'image']) {
    test(`macOS ${theme} ${kind} preview preserves traffic lights and native hit regions`, async ({
      page,
    }) => {
      await setupPreview(page, 'macos', kind, theme);
      const originalGlass = await page.screenshot({
        clip: trafficLightsClip,
        path: test.info().outputPath('glass-before.png'),
      });
      const appearanceCalls = await page.evaluate(() => (window as any).__previewAppearanceCalls);
      await page.getByRole('button', { name: 'Preview', exact: true }).click();
      const overlay = page.getByTestId('attachment-preview-overlay');
      const header = overlay.locator('[data-preview-titlebar]');
      await expectPreviewTitlebar(header, 79);
      await page.screenshot({ path: test.info().outputPath('preview-open.png') });
      expect(
        await page.screenshot({
          clip: trafficLightsClip,
          path: test.info().outputPath('glass-after.png'),
        }),
      ).toEqual(originalGlass);
      await expectWholeRowGlass(page);
      for (const [width, right] of [
        [800, 79],
        [560, 106],
        [800, 0],
        [800, 79],
      ]) {
        await page.evaluate((right) => {
          Object.assign((window as any).__previewLayout, {
            titlebarHeight: right ? 52 : 0,
            trafficLightsRight: right,
          });
          window.dispatchEvent(new Event('resize'));
        }, right);
        await page.setViewportSize({ width, height: 600 });
        await expect(header).toHaveCSS('padding-left', `${right ? right + 12 : 14}px`);
        await expectPreviewTitlebar(header, right);
      }
      await page.screenshot({ path: test.info().outputPath('preview.png') });
      await header.click({ position: { x: 400, y: 4 } });
      await expect(overlay).toBeVisible();
      await header.getByRole('button', { name: 'Back', exact: true }).click();
      await expect(overlay).toHaveCount(0);
      await expect(page.locator('#root')).toHaveCSS('clip-path', 'none');
      await expectNativeControls(page.locator('[data-appbar]'));
      expect(await page.evaluate(() => (window as any).__previewAppearanceCalls)).toBe(
        appearanceCalls,
      );
    });
  }
}

test('macOS photo viewer returns through album and restores each titlebar hit region', async ({
  page,
}) => {
  await setupPreview(page, 'macos', 'image');
  const originalGlass = await page.screenshot({ clip: trafficLightsClip });
  await page.getByRole('button', { name: /Photo Album/ }).click();
  const album = page.getByTestId('photo-album-overlay');
  const albumHeader = album.locator(':scope > [data-preview-titlebar]');
  await expectPreviewTitlebar(albumHeader, 79);
  await expectWholeRowGlass(page);
  expect(await page.screenshot({ clip: trafficLightsClip })).toEqual(originalGlass);
  await album.getByRole('button', { name: 'Travel photo.png', exact: true }).click();
  const viewer = page.getByTestId('photo-viewer');
  const header = viewer.locator('[data-preview-titlebar]');
  await expectPreviewTitlebar(header, 79);
  await expect(albumHeader).toBeHidden();
  await expectWholeRowGlass(page);
  expect(await page.screenshot({ clip: trafficLightsClip })).toEqual(originalGlass);
  // 后方 AppBar 重排不能覆盖前方查看器的命中区。
  await page.setViewportSize({ width: 800, height: 600 });
  await expectPreviewTitlebar(header, 79);
  await header.getByRole('button', { name: 'Back to album', exact: true }).click();
  await expect(viewer).toHaveCount(0);
  await expectPreviewTitlebar(albumHeader, 79);
  await albumHeader.getByRole('button', { name: 'Close', exact: true }).click();
  await expect(album).toHaveCount(0);
  await expect(page.locator('#root')).toHaveCSS('clip-path', 'none');
  await expectNativeControls(page.locator('[data-appbar]'));
});

test('macOS preview also removes the object attachment card backdrop from the traffic lights', async ({
  page,
}) => {
  await setupPreview(page);
  const originalGlass = await page.screenshot({ clip: trafficLightsClip });
  await page.evaluate(() => {
    history.pushState(
      { ...history.state, idx: history.state.idx + 1, usr: null },
      '',
      '/workspace?objectId=preview-object',
    );
    window.dispatchEvent(new PopStateEvent('popstate', { state: history.state }));
  });
  const detail = page.getByTestId('object-detail-modal');
  await detail.getByRole('button', { name: 'Attachments', exact: true }).click();
  await page.getByRole('button', { name: 'Preview', exact: true }).last().click();
  const overlay = page.getByTestId('attachment-preview-overlay').filter({ visible: true });
  await expectPreviewTitlebar(overlay.locator('[data-preview-titlebar]'), 79);
  await expectWholeRowGlass(page);
  expect(await page.screenshot({ clip: trafficLightsClip })).toEqual(originalGlass);
  await overlay.getByRole('button', { name: 'Edit Attachment Attributes', exact: true }).click();
  const editor = page.getByRole('dialog').filter({ has: page.getByLabel('Name', { exact: true }) });
  await expect(editor.getByLabel('Name', { exact: true })).toBeVisible();
  await editor.getByRole('button', { name: 'Cancel', exact: true }).click();
  await expect(overlay).toBeVisible();
  await overlay.getByRole('button', { name: 'Back', exact: true }).click();
  await expect(detail).toBeVisible();
});

for (const platform of ['windows', 'android']) {
  test(`${platform} preview keeps its existing top spacing`, async ({ page }) => {
    await setupPreview(page, platform);
    if (platform === 'android') {
      await page.setViewportSize({ width: 390, height: 844 });
      await page.getByRole('button', { name: /^Attachment actions:/ }).click();
      await page.getByRole('dialog').getByRole('button', { name: 'Preview', exact: true }).click();
    } else await page.getByRole('button', { name: 'Preview', exact: true }).click();
    const overlay = page.getByTestId('attachment-preview-overlay');
    const header = overlay.locator('[data-preview-titlebar]');
    await expect(header).toHaveCSS('padding-left', '14px');
    await expect(header).toHaveCSS('padding-top', '10px');
    await header.getByRole('button', { name: 'Close', exact: true }).click();
    await expect(overlay).toHaveCount(0);
  });
}
