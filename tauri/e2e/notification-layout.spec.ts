import { expect, test, type Page } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { login } from './fixtures/auth';

type Platform = 'macos' | 'windows' | 'android';
type Navigation = 'left' | 'right' | 'top' | 'bottom';

// 本文件自行覆盖桌面/手机/平板视口，避免在 mobile project 中重复矩阵并改变 hover 能力。
test.beforeEach(({ browserName }, testInfo) => {
  test.skip(testInfo.project.name !== 'chromium' || browserName !== 'chromium');
});

async function setup(
  page: Page,
  platform: Platform,
  navigation: Navigation = 'left',
  warning = '',
  unlock = true,
  ocr = false,
) {
  await page.addInitScript({
    content:
      readFileSync('e2e/fixtures/tauriMock.js', 'utf8') +
      `
      window.__MOCK_PLATFORM__ = ${JSON.stringify(platform)};
      const platform = window.__MOCK_PLATFORM__;
      const titlebarHeight = platform === 'macos' ? 52 : platform === 'windows' ? 40 : 0;
      const material = platform === 'macos' ? 'liquid-glass' : platform === 'windows' ? 'mica' : 'solid';
      let nextRid = 100;
      const downloads = window.__E2E_UPDATE_DOWNLOADS__ = [];
      function startDownload({ operationId, onEvent }) {
        return new Promise((_, reject) => {
          let index = 0;
          downloads.push({
            operationId,
            cancel: () => reject('UPDATE_DOWNLOAD_CANCELLED'),
            emit: (data) => {
              const message = platform === 'android'
                ? { ...data, progress: data.total ? data.downloaded / data.total * 100 : 0, done: false, error: null }
                : { event: 'Transfer', data };
              window.__TAURI_INTERNALS__.runCallback(onEvent.id, { message, index: index++ });
            },
          });
        });
      }
      const prefs = { theme: 'light', accentColor: 'ocean', hasSeenOnboarding: true,
        language: 'en-US', reduceMotion: true, autoLockTimeoutMinutes: 0,
        sidebarPosition: ${JSON.stringify(navigation)}, sidebarBottomActions: ['search', 'plugins', 'ai_chat'],
        customPages: [] };
      const notes = '**Downloads**\\n\\n| Platform | File |\\n| --- | --- |\\n| macOS | app.dmg |';
      window.__E2E_MOCKS__ = {
        ui_get_preferences: () => prefs,
        user_data_get_preferences: () => prefs,
        vault_check_directory: () => true,
        ocr_get_model_status: () => ({ installed: ${!ocr}, bundled: true }),
        ocr_install_bundled_model_with_progress: () => new Promise(() => {}),
        sync_list_conflicts: () => [],
        set_titlebar_color: () => ({ material, platform, titlebarHeight, trafficLightsRight: platform === 'macos' ? 79 : 0,
          reduceMotion: true, highContrast: false }),
        get_window_layout: () => ({ platform, titlebarHeight, trafficLightsRight: platform === 'macos' ? 79 : 0 }),
        desktop_prepare_update: () => ({ rid: nextRid++, currentVersion: '2.13.0', version: '9.9.9', body: notes, rawJson: {} }),
        android_check_update: () => ({ currentVersion: '2.13.0', latestVersion: '9.9.9',
          downloadUrl: 'https://example.test/app.apk', releaseNotes: notes,
          mandatory: false, checksum: 'verified', checksumWarning: ${JSON.stringify(warning)} }),
        android_is_apk_downloaded: () => false,
        create_update_download: () => nextRid++,
        desktop_download_update: startDownload,
        android_download_apk: startDownload,
        cancel_update_download: ({ operationId }) => downloads.find((download) => download.operationId === operationId)?.cancel(),
        object_list: ({ filter }) => filter?.typeId === 'page' ? [] : Array.from({ length: 35 }, (_, i) => ({
          id: 'notice-' + i, name: 'Notice object ' + i, typeId: 'identity', sensitivityLevel: 'public',
          createdAt: '2026-09-20T00:00:00Z', updatedAt: '2026-09-20T00:00:00Z',
          properties: { city: 'Kyoto' }, propertyLabels: { city: 'public' }
        })),
        llm_get_config: () => ({ activeProviderId: 'layout-provider',
          aiFeaturesEnabled: { chat: true }, includeSystemPrompt: true }),
        llm_get_providers: () => [{ id: 'layout-provider', name: 'Layout Provider', model: 'test-model',
          baseUrl: 'http://127.0.0.1:9', apiType: 'openAI' }],
        llm_get_api_key: () => '',
        llm_check_connection: () => true,
        llm_list_conversations: () => [],
        llm_list_trash: () => [],
      };
      localStorage.setItem('i18nextLng', 'en-US');
    `,
  });
  if (unlock) await login(page);
  else await page.goto('/login');
  await expect(
    page.locator('[data-shell-notifications]').getByRole('button', { name: 'Update Now' }),
  ).toBeVisible();
}

type Transfer = {
  downloaded: number;
  total: number;
  source: string;
  bytesPerSecond: number;
  phase: 'probing' | 'downloading' | 'switching';
};
type DownloadMockWindow = Window & {
  __E2E_UPDATE_DOWNLOADS__: Array<{ operationId: number; emit: (data: Transfer) => void }>;
};

async function emitTransfer(page: Page, index: number, data: Transfer) {
  await expect
    .poll(() => page.evaluate(() => (window as DownloadMockWindow).__E2E_UPDATE_DOWNLOADS__.length))
    .toBeGreaterThan(index);
  await page.evaluate(
    ({ index, data }) => {
      (window as DownloadMockWindow).__E2E_UPDATE_DOWNLOADS__[index].emit(data);
    },
    { index, data },
  );
}

/** 检查真实布局边界和元素命中，而不是只检查声明了某段 CSS。 */
async function expectUnobscuredContent(page: Page) {
  await expect
    .poll(() =>
      page.evaluate(() => {
        const content = document.querySelector<HTMLElement>('[data-shell-content]')!;
        const slot = document.querySelector<HTMLElement>('[data-shell-notifications]')!;
        const main = document.querySelector<HTMLElement>('[data-shell-main]')!;
        if (!content || !slot || !main) return false;
        const rect = content.getBoundingClientRect();
        const notice = slot.getBoundingClientRect();
        const root = getComputedStyle(document.documentElement);
        const top = parseFloat(root.getPropertyValue('--shell-content-top'));
        const height = parseFloat(root.getPropertyValue('--shell-content-height'));
        const hit = document.elementFromPoint(
          rect.left + Math.min(24, rect.width / 2),
          rect.top + 4,
        );
        return (
          Math.abs(rect.top - notice.bottom) < 1 &&
          rect.height > 120 &&
          Math.abs(top - rect.top) < 1 &&
          Math.abs(height - rect.height) < 1 &&
          slot.parentElement === content.parentElement &&
          !['fixed', 'absolute'].includes(getComputedStyle(slot).position) &&
          !!hit &&
          content.contains(hit) &&
          document.documentElement.scrollWidth <= window.innerWidth
        );
      }),
    )
    .toBe(true);
}

for (const platform of ['macos', 'windows'] as const) {
  for (const navigation of ['left', 'right', 'top', 'bottom'] as const) {
    test(`${platform} ${navigation} 通知占位，关闭后正文收回空间且顶栏不移动`, async ({ page }) => {
      await page.setViewportSize({ width: 1280, height: 720 });
      await setup(page, platform, navigation);
      await expectUnobscuredContent(page);
      if (platform === 'macos' && (navigation === 'left' || navigation === 'top'))
        await page.screenshot({ path: `test-results/notification-macos-${navigation}.png` });
      const bar = page.locator('header[data-appbar]');
      const barBefore = (await bar.boundingBox())!;
      const slot = page.locator('[data-shell-notifications]');
      const slotBefore = (await slot.boundingBox())!;
      const contentBefore = (await page.locator('[data-shell-content]').boundingBox())!;
      expect(slotBefore.y).toBe(barBefore.height + (navigation === 'top' ? 48 : 0));
      const nav = page.locator('#desktop-navigation, header:not([data-appbar])');
      const navBefore = await nav.boundingBox();
      await slot.getByRole('button', { name: 'Close', exact: true }).click();
      await expect(slot).toBeEmpty();
      await expectUnobscuredContent(page);
      expect(await bar.boundingBox()).toEqual(barBefore);
      expect(await nav.boundingBox()).toEqual(navBefore);
      const contentAfter = (await page.locator('[data-shell-content]').boundingBox())!;
      expect(contentBefore.y - contentAfter.y).toBeCloseTo(slotBefore.height, 1);
    });
  }
}

for (const width of [320, 390, 1024]) {
  test(`Android ${width}px 通知换行、下载取消和底部导航不遮挡正文`, async ({ page }) => {
    await page.setViewportSize({ width, height: 844 });
    await setup(
      page,
      'android',
      'bottom',
      'The signed checksum cannot currently be retrieved. Please try again later.',
    );
    await expectUnobscuredContent(page);
    if (width < 768)
      await page.screenshot({ path: `test-results/notification-android-${width}.png` });
    const slot = page.locator('[data-shell-notifications]');
    const bar = page.locator('.android-appbar');
    const barBefore = await bar.boundingBox();
    const navigation = page.locator('.android-navigation');
    const navBefore = (await navigation.boundingBox())!;
    const contentBefore = (await page.locator('[data-shell-content]').boundingBox())!;
    if (width < 768)
      expect(contentBefore.y + contentBefore.height).toBeLessThanOrEqual(navBefore.y + 1);
    else expect(contentBefore.x).toBeGreaterThanOrEqual(navBefore.width);
    await slot.getByRole('button', { name: 'Update Now' }).click();
    await expect(slot.getByRole('progressbar')).toBeVisible();
    await expectUnobscuredContent(page);
    await slot.getByRole('button', { name: 'Cancel download' }).click();
    await expect(slot.getByRole('button', { name: 'Update Now' })).toBeVisible();
    await expectUnobscuredContent(page);
    expect(await bar.boundingBox()).toEqual(barBefore);
    expect(await navigation.boundingBox()).toEqual(navBefore);
  });
}

for (const platform of ['macos', 'windows', 'android'] as const) {
  test(`${platform} 下载选源、换源及取消续传保持正文可见`, async ({ page }) => {
    await page.setViewportSize(
      platform === 'android' ? { width: 320, height: 844 } : { width: 1000, height: 650 },
    );
    await setup(page, platform);
    const slot = page.locator('[data-shell-notifications]');
    await slot.getByRole('button', { name: 'Update Now' }).click();
    await expect(slot.getByText('Choosing a download source…')).toBeVisible();
    await expect(slot.getByRole('button', { name: 'Cancel download' })).toBeEnabled();
    await expectUnobscuredContent(page);

    const progress: Transfer = {
      downloaded: 40,
      total: 100,
      source: 'cdn.example.test',
      bytesPerSecond: 2097152,
      phase: 'downloading',
    };
    await emitTransfer(page, 0, progress);
    await expect(slot.getByText('Source: cdn.example.test · 2.0 MB/s')).toBeVisible();
    await expect(slot.getByRole('progressbar')).toHaveAttribute('aria-valuenow', '40');
    await expectUnobscuredContent(page);
    await emitTransfer(page, 0, {
      ...progress,
      downloaded: 55,
      source: 'backup.example.test',
      bytesPerSecond: 0,
      phase: 'switching',
    });
    await expect(
      slot.getByText('Switching download source… · Source: backup.example.test'),
    ).toBeVisible();
    await expect
      .poll(async () => Number(await slot.getByRole('progressbar').getAttribute('aria-valuenow')))
      .toBeCloseTo(55, 6);
    await expectUnobscuredContent(page);
    await slot.getByRole('button', { name: 'Cancel download' }).click();
    await expect(slot.getByRole('button', { name: 'Update Now' })).toBeVisible();
    await expect(slot.locator('[data-update-transfer-phase]')).toHaveCount(0);
    await expectUnobscuredContent(page);

    await slot.getByRole('button', { name: 'Update Now' }).click();
    await expect(slot.getByText('Choosing a download source…')).toBeVisible();
    await emitTransfer(page, 1, { ...progress, downloaded: 55, source: 'backup.example.test' });
    await emitTransfer(page, 0, { ...progress, downloaded: 99, source: 'old.example.test' });
    await expect
      .poll(async () => Number(await slot.getByRole('progressbar').getAttribute('aria-valuenow')))
      .toBeCloseTo(55, 6);
    await expect(slot.getByText('Source: backup.example.test · 2.0 MB/s')).toBeVisible();
    await expect(slot.getByText(/old\.example\.test/)).toHaveCount(0);
    await expectUnobscuredContent(page);
    await slot.getByRole('button', { name: 'Cancel download' }).click();
    await expect(slot.getByRole('button', { name: 'Update Now' })).toBeVisible();
  });
}

test('对象尺标和悬停卡片跟随通知后的正文边界', async ({ page }) => {
  await page.setViewportSize({ width: 1000, height: 720 });
  await setup(page, 'macos');
  await page
    .locator('#desktop-navigation')
    .getByRole('button', { name: 'Identity', exact: true })
    .click();
  const ruler = page.getByRole('navigation', { name: 'Object ruler' });
  await expect(ruler).toBeVisible();
  await expectUnobscuredContent(page);
  const content = (await page.locator('[data-shell-content]').boundingBox())!;
  expect((await ruler.boundingBox())!.y).toBeGreaterThanOrEqual(content.y);
  await ruler.locator('[data-ruler-index="1"]').hover();
  const preview = page.getByRole('region', { name: 'Object preview' });
  await expect(preview).toBeVisible();
  expect((await preview.boundingBox())!.y).toBeGreaterThanOrEqual(content.y);
  await preview.getByRole('button', { name: 'Locate this object' }).click();
  const card = page.locator('#workspace-object-notice-1');
  await expect(card).toHaveAttribute('data-ruler-target', 'true');
  expect((await card.boundingBox())!.y).toBeGreaterThanOrEqual(content.y);
  const scroller = page.locator('[data-shell-content]');
  await scroller.evaluate((element) => {
    element.scrollTop = 800;
  });
  const scrollBefore = await scroller.evaluate((element) => element.scrollTop);
  expect(scrollBefore).toBeGreaterThan(0);
  await page
    .locator('[data-shell-notifications]')
    .getByRole('button', { name: 'Close', exact: true })
    .click();
  await expectUnobscuredContent(page);
  await expect.poll(() => scroller.evaluate((element) => element.scrollTop)).toBe(scrollBefore);
  await expect.poll(async () => (await ruler.boundingBox())!.y).toBe(68);
});

test('外观设置与固定聊天面板使用通知后的可用高度', async ({ page }) => {
  await page.setViewportSize({ width: 1000, height: 650 });
  await setup(page, 'windows');
  const sidebar = page.locator('#desktop-navigation');
  await sidebar.getByRole('button', { name: 'Settings', exact: true }).click();
  await page.getByText('Theme & Appearance', { exact: true }).click();
  await page.getByRole('button', { name: 'More Appearances' }).click();
  const panel = page.getByRole('region', { name: 'Theme Schemes' });
  await expect(panel).toBeVisible();
  const content = (await page.locator('[data-shell-content]').boundingBox())!;
  const panelBox = (await panel.boundingBox())!;
  expect(panelBox.y).toBeGreaterThanOrEqual(content.y);
  expect(panelBox.y + panelBox.height).toBeLessThanOrEqual(content.y + content.height + 1);
  await panel.getByRole('button', { name: 'Close' }).click();
  await sidebar.getByRole('button', { name: 'Tools', exact: true }).hover();
  await sidebar.locator('[data-ai-button] button').click();
  await page.locator('[data-ai-quick-chat]').getByRole('button', { name: /full/i }).click();
  await expect(page).toHaveURL(/\/llm-chat$/);
  await expectUnobscuredContent(page);
  const chat = page.locator('[data-shell-content] > div[style*="position: fixed"]');
  await expect(chat).toBeVisible();
  await expect
    .poll(async () => {
      const chatBox = (await chat.boundingBox())!;
      const body = (await page.locator('[data-shell-content]').boundingBox())!;
      return Math.abs(chatBox.y - body.y) < 1 && Math.abs(chatBox.height - body.height) < 1;
    })
    .toBe(true);
});

test('登录页通知位于卡片上方，短窗多条通知可滚动且正文末尾可达', async ({ page }) => {
  await page.setViewportSize({ width: 800, height: 600 });
  await setup(page, 'macos', 'left', '', false, true);
  const slot = page.locator('[data-shell-notifications]');
  await expect(slot.locator('[data-notification-banner="ocr"]')).toBeVisible();
  await expect(page.locator('[data-auth-content]')).toBeVisible();
  const slotBox = (await slot.boundingBox())!;
  const cardBox = (await page.locator('[class*="loginCard"]').boundingBox())!;
  expect(cardBox.y).toBeGreaterThanOrEqual(slotBox.y + slotBox.height);
  expect(slotBox.y).toBeGreaterThanOrEqual(52);
  await page.screenshot({ path: 'test-results/notification-login.png' });
  await page.locator('input[type="text"]').fill('any-password');
  await page.locator('button[type="submit"]').click();
  await page.waitForURL('/');
  await page.setViewportSize({ width: 800, height: 460 });
  await expectUnobscuredContent(page);
  await page
    .locator('#desktop-navigation')
    .getByRole('button', { name: 'Identity', exact: true })
    .click();
  await expect(page.getByTestId('workspace-object-card')).toHaveCount(35);
  await page.locator('[data-shell-content]').evaluate((element) => {
    element.scrollTop = element.scrollHeight;
  });
  await expect(page.getByTestId('workspace-object-card').last()).toBeInViewport();
  const main = (await page.locator('[data-shell-content]').boundingBox())!;
  expect(main.height).toBeGreaterThan(120);
});
