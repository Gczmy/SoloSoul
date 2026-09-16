import { test, expect, type Page } from '@playwright/test';
import { setupTauriMock, login } from './fixtures/auth';

const paths = [
  '/',
  '/tools',
  '/workspace',
  '/search',
  '/editor',
  '/settings',
  '/settings/appearance',
  '/settings/security',
  '/settings/account',
  '/settings/data',
  '/settings/vault-directory',
  '/settings/backup',
  '/settings/export-import',
  '/settings/trash',
  '/settings/operation-log',
  '/settings/templates',
  '/settings/attachments',
  '/settings/ocr',
  '/settings/cloud-sync',
  '/settings/llm',
  '/settings/llm/stats',
  '/llm-chat',
  '/plugins',
  '/local-import',
  '/history',
  '/ocr',
  '/sync',
  '/help',
  '/about',
  '/debug-log',
];

test.beforeEach(async ({ page }, testInfo) => {
  await setupTauriMock(page);
  await page.addInitScript(() => {
    const prefs = {
      theme: 'light',
      language: 'en-US',
      hasSeenOnboarding: true,
      autoLockTimeoutMinutes: 0,
      lastBackupAt: new Date().toISOString(),
    };
    Object.assign(window, {
      __MOCK_PLATFORM__: 'android',
      __E2E_MOCKS__: {
        ui_get_preferences: () => prefs,
        user_data_get_preferences: () => prefs,
        vault_check_directory: () => true,
        vault_get_directory: () => ({ directoryType: 'local', valid: true }),
        get_vault_stats: () => ({
          totalSizeBytes: 1024,
          profileCount: 1,
          profilesSize: 1024,
          objectsSize: 0,
          trashSize: 0,
          snapshotsSize: 0,
          attachmentsSize: 0,
          aiConversationsSize: 0,
        }),
        attachment_count_stats: () => ({ attachmentCount: 0, photoCount: 0 }),
        attachment_list_all: () => ({ pages: [], trashPages: [] }),
        log_get_recent: () => [],
        object_trash_list: () => [],
        export_get_scope_tree: () => [],
        ocr_list_available_tiers: () => [],
        ocr_get_active_tier: () => 'small',
        ocr_get_model_status: () => ({ installed: true, bundled: true }),
        llm_get_config: () => ({ aiFeaturesEnabled: { chat: false }, activeProviderId: '' }),
        llm_get_providers: () => [],
        llm_list_conversations: () => [],
        llm_usage_stats: () => ({ totalRequests: 0, totalTokens: 0, byProvider: [] }),
        sync_list_conflicts: () => [],
        sync_list_devices: () => [],
        sync_get_history: () => [],
        get_app_info: () => ({
          version: '2.12.3',
          buildType: 'debug',
          os: 'android',
          arch: 'aarch64',
        }),
        android_check_update: () => ({ currentVersion: '2.12.3', latestVersion: '2.12.3' }),
      },
    });
    localStorage.setItem('i18nextLng', 'en-US');
  });
  if (!testInfo.title.startsWith('Android login')) await login(page);
});

async function navigate(page: Page, path: string) {
  await page.evaluate((destination) => {
    window.history.pushState(
      { ...window.history.state, idx: (window.history.state?.idx ?? 0) + 1 },
      '',
      destination,
    );
    window.dispatchEvent(new PopStateEvent('popstate', { state: window.history.state }));
  }, path);
  await expect(page).toHaveURL(path);
  // 路由组件按需加载，等待浏览器渲染稳定后测量实际可交互区域。
  if (path !== '/') await expect(page.locator('h1').first()).not.toHaveText('Home');
  await page.evaluate(
    () =>
      new Promise<void>((resolve) =>
        requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
      ),
  );
}

for (const path of paths)
  test(`Android touch targets: ${path}`, async ({ page }) => {
    test.setTimeout(120_000);
    const report = [];
    await navigate(page, path);
    for (const width of [320, 390]) {
      await page.setViewportSize({ width, height: 844 });
      const result = await page.evaluate(() => {
        const buttons = [
          ...document.querySelectorAll<HTMLElement>('button, [role="button"], [role="tab"]'),
        ].filter(
          (element) =>
            element.getClientRects().length &&
            getComputedStyle(element).visibility !== 'hidden' &&
            !element.closest('[inert]'),
        );
        const describe = (element: HTMLElement) => ({
          name: (element.getAttribute('aria-label') || element.title || element.textContent || '')
            .trim()
            .slice(0, 65),
          width: Math.round(element.getBoundingClientRect().width),
          height: Math.round(element.getBoundingClientRect().height),
          className: element.className,
        });
        return {
          buttons: buttons.length,
          small: buttons
            .filter((element) => {
              const rect = (
                element.closest('[data-attachment-toggle-row]') || element
              ).getBoundingClientRect();
              return rect.width < 47.5 || rect.height < 47.5;
            })
            .map(describe),
          clipped: buttons
            .filter((element) => {
              const rect = element.getBoundingClientRect();
              if (rect.left >= -1 && rect.right <= innerWidth + 1) return false;
              // 分类横向列表允许滚动到屏幕外；普通布局中的裁切不允许。
              for (let parent = element.parentElement; parent; parent = parent.parentElement) {
                if (
                  ['auto', 'scroll'].includes(getComputedStyle(parent).overflowX) &&
                  parent.scrollWidth > parent.clientWidth
                )
                  return false;
              }
              return true;
            })
            .map(describe),
          textOverflow: buttons
            .filter((element) => element.scrollWidth > element.clientWidth + 1)
            .map(describe),
          error: /Cannot read properties|Something went wrong|Error loading/.test(
            document.body.innerText,
          ),
          heading: document.querySelector('h1')?.textContent,
        };
      });
      report.push({ path, width, ...result });
      if (width === 390 && ['/settings/data', '/plugins'].includes(path)) {
        await page.screenshot({
          path: test.info().outputPath('page-390.png'),
          animations: 'disabled',
        });
      }
    }
    await test.info().attach('touch-target-audit', {
      body: JSON.stringify(report, null, 2),
      contentType: 'application/json',
    });
    if (process.env.SOLOSOUL_TOUCH_AUDIT) {
      console.log(
        JSON.stringify(
          report.filter(
            (r) => r.small.length || r.clipped.length || r.textOverflow.length || r.error,
          ),
        ),
      );
    } else {
      expect(
        report.filter(
          (r) => r.small.length || r.clipped.length || r.textOverflow.length || r.error,
        ),
      ).toEqual([]);
    }
  });

test('Android login controls do not cover the password text', async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 844 });
  await page.goto('/login');
  const password = page.locator('.interactive-password-field');
  await password.locator('input').fill('EXAMPLE-PASSWORD');
  await expect(password.getByRole('button')).toHaveCount(2);
  expect(
    await password.evaluate((field) => {
      const input = field.querySelector('input')!;
      const rect = input.getBoundingClientRect();
      const textEnd = rect.right - parseFloat(getComputedStyle(input).paddingRight);
      const buttons = [...field.querySelectorAll('button')].map((button) =>
        button.getBoundingClientRect(),
      );
      return (
        buttons.every(
          (button) =>
            button.width >= 48 &&
            button.height >= 48 &&
            button.left >= textEnd &&
            button.right <= rect.right,
        ) && buttons[0].right <= buttons[1].left
      );
    }),
  ).toBe(true);
  await page.screenshot({ path: test.info().outputPath('login-320.png'), animations: 'disabled' });
});

test('Android template and data dialogs keep actions reachable on narrow screens', async ({
  page,
}) => {
  await page.setViewportSize({ width: 320, height: 844 });
  await navigate(page, '/settings/templates');
  await page.getByRole('button', { name: 'New Template', exact: true }).click();
  const dialog = page.getByRole('dialog');
  await expect(dialog).toBeVisible();
  await dialog.getByRole('button', { name: /Icon/ }).click();
  const grid = dialog.locator('.template-icon-grid');
  await expect(grid).toBeVisible();
  expect(
    await grid.evaluate((element) => {
      const rect = element.getBoundingClientRect();
      return (
        rect.left >= 16 &&
        rect.right <= innerWidth - 16 &&
        element.scrollWidth <= element.clientWidth &&
        [...element.querySelectorAll('button')].every(
          (button) => button.offsetWidth >= 48 && button.offsetHeight >= 48,
        )
      );
    }),
  ).toBe(true);
  await page.screenshot({
    path: test.info().outputPath('template-icons-320.png'),
    animations: 'disabled',
  });
  await dialog.getByRole('button', { name: 'Cancel', exact: true }).click();
  await navigate(page, '/settings/data');
  await page.getByRole('button', { name: 'View breakdown', exact: true }).click();
  const breakdown = page
    .locator('[data-macos-glass="panel"]')
    .filter({ has: page.getByRole('heading', { name: 'Storage Breakdown', exact: true }) })
    .last();
  await expect(breakdown).toBeVisible();
  const bounds = await breakdown.boundingBox();
  expect(bounds!.x).toBeGreaterThanOrEqual(16);
  expect(bounds!.x + bounds!.width).toBeLessThanOrEqual(304);
});

test('Android attachment expansion keeps compact arrows and a full-row touch target', async ({
  page,
}) => {
  await page.setViewportSize({ width: 320, height: 844 });
  await page.evaluate(() => {
    const mocks = (window as unknown as { __E2E_MOCKS__: Record<string, unknown> }).__E2E_MOCKS__;
    mocks.attachment_list_all = () => ({
      pages: [
        {
          pageName: 'Travel',
          objects: [
            {
              objectId: 'travel',
              objectName: 'Travel documents',
              attachments: [
                {
                  id: 'long-report',
                  objectId: 'travel',
                  fileName: 'Very long travel report with detailed itinerary and receipts.pdf',
                  description: 'A detailed description of all the documents in this attachment.',
                  tags: ['Travel', 'Receipts', 'Report', 'Itinerary', 'Flight'],
                  mimeType: 'application/pdf',
                  sizeBytes: 1024,
                  createdAt: '2026-09-16',
                  vaultPath: '/mock-vault/report.pdf',
                },
              ],
            },
          ],
        },
      ],
      trashPages: [],
    });
  });
  await navigate(page, '/settings/attachments');
  const rows = page.locator('[data-attachment-toggle-row]');
  await expect(rows).toHaveCount(3);
  for (const row of await rows.all()) {
    expect(
      await row.evaluate((element) => {
        const rect = element.getBoundingClientRect();
        const arrow = element.querySelector('button')!.getBoundingClientRect();
        return rect.width >= 48 && rect.height >= 48 && arrow.width === 18 && arrow.height === 18;
      }),
    ).toBe(true);
  }
  // 点击整行底部的留白也应展开，避免只是视觉变大、真实目标仍局限于箭头。
  const row = rows.first();
  await row.click({ position: { x: 20, y: 42 } });
  await expect(row.getByRole('button')).toHaveAttribute('aria-expanded', 'true');
  await row.click({ position: { x: 20, y: 42 } });
  await expect(row.getByRole('button')).toHaveAttribute('aria-expanded', 'false');
  await page.screenshot({
    path: test.info().outputPath('attachment-expansion-320.png'),
    animations: 'disabled',
  });
});
