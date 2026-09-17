import { test, expect, type Page } from '@playwright/test';
import { setupTauriMock, login } from './fixtures/auth';

const routes = [
  '/',
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

async function setup(page: Page, platform: 'macos' | 'windows', theme = 'light') {
  await setupTauriMock(page);
  await page.addInitScript(
    ({ platform, theme }) => {
      const prefs = {
        theme,
        language: 'en-US',
        hasSeenOnboarding: true,
        autoLockTimeoutMinutes: 0,
        lastBackupAt: new Date().toISOString(),
      };
      const layout = {
        platform,
        titlebarHeight: platform === 'macos' ? 52 : 0,
        trafficLightsRight: platform === 'macos' ? 79 : 0,
      };
      Object.assign(window, {
        __MOCK_PLATFORM__: platform,
        __E2E_MOCKS__: {
          ui_get_preferences: () => prefs,
          user_data_get_preferences: () => prefs,
          set_titlebar_color: () => ({
            ...layout,
            material: platform === 'macos' ? 'liquid-glass' : 'mica',
            reduceMotion: false,
            highContrast: false,
          }),
          get_window_layout: () => layout,
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
          llm_get_embed_models: () => [],
          llm_check_embedding_available: () => false,
          llm_list_conversations: () => [],
          llm_usage_stats: () => ({ totalRequests: 0, totalTokens: 0, byProvider: [] }),
          sync_list_conflicts: () => [],
          sync_list_devices: () => [],
          sync_get_history: () => [],
          get_app_info: () => ({
            version: '2.12.3',
            buildType: 'debug',
            os: platform,
            arch: 'aarch64',
          }),
          desktop_check_update: () => ({ currentVersion: '2.12.3', latestVersion: '2.12.3' }),
        },
      });
      localStorage.setItem('i18nextLng', 'en-US');
    },
    { platform, theme },
  );
  await login(page);
  await expect(page.locator('html')).toHaveAttribute('data-desktop-platform', platform);
}

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
  if (path !== '/') await expect(page.locator('h1').first()).not.toHaveText('Home');
  await page.evaluate(
    () =>
      new Promise<void>((resolve) =>
        requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
      ),
  );
}

for (const platform of ['macos', 'windows'] as const) {
  test.describe(platform, () => {
    for (const route of routes) {
      test(`desktop button layout: ${route}`, async ({ page }) => {
        await setup(page, platform);
        await navigate(page, route);
        const reports = [];
        for (const width of [800, 1280]) {
          await page.setViewportSize({ width, height: 800 });
          reports.push(
            await page.evaluate(() => {
              const buttons = [...document.querySelectorAll<HTMLButtonElement>('button')].filter(
                (button) =>
                  button.getClientRects().length &&
                  getComputedStyle(button).visibility !== 'hidden' &&
                  !button.closest('[inert]'),
              );
              const clipped = buttons.filter((button) => {
                const r = button.getBoundingClientRect();
                if (r.left >= -1 && r.right <= innerWidth + 1) return false;
                for (let p = button.parentElement; p; p = p.parentElement) {
                  if (
                    ['auto', 'scroll'].includes(getComputedStyle(p).overflowX) &&
                    p.scrollWidth > p.clientWidth
                  )
                    return false;
                }
                return true;
              });
              return {
                width: innerWidth,
                buttons: buttons.length,
                styled: buttons.filter((button) =>
                  getComputedStyle(button).getPropertyValue('--control-fill').trim(),
                ).length,
                clipped: clipped.map(
                  (button) => button.getAttribute('aria-label') || button.textContent,
                ),
                error: /Cannot read properties|Something went wrong|Error loading/.test(
                  document.body.innerText,
                ),
              };
            }),
          );
        }
        await test.info().attach('desktop-button-audit', {
          body: JSON.stringify(reports, null, 2),
          contentType: 'application/json',
        });
        for (const report of reports) {
          expect(report.error).toBe(false);
          expect(report.clipped).toEqual([]);
          expect(report.styled).toBeGreaterThan(0);
        }
      });
    }

    test('primary and destructive actions retain their meaning inside dialogs', async ({
      page,
    }) => {
      await setup(page, platform);
      await page.evaluate(() => {
        const object = {
          id: 'test-passport',
          name: 'Passport',
          typeId: 'identity',
          properties: {},
          propertyLabels: {},
          createdAt: '2026-09-16',
          updatedAt: '2026-09-16',
        };
        const mocks = (window as unknown as { __E2E_MOCKS__: Record<string, unknown> })
          .__E2E_MOCKS__;
        Object.assign(mocks, {
          object_list: ({ filter }: { filter?: { typeId?: string } }) =>
            filter?.typeId === 'page' ? [] : [object],
          object_get: () => object,
          attachment_count_batch: () => ({}),
          snapshot_list: () => [],
        });
      });
      await page
        .locator('#desktop-navigation')
        .getByRole('button', { name: 'Identity', exact: true })
        .click();
      await page.getByRole('button', { name: /^Passport/ }).click();
      const detail = page.getByTestId('object-detail-modal');
      for (const name of ['History', 'Attachments', 'Edit', 'Close']) {
        await expect(detail.getByRole('button', { name, exact: true })).toHaveCSS(
          'border-radius',
          platform === 'macos' ? '8px' : '4px',
        );
      }
      const remove = detail.getByRole('button', { name: 'Delete', exact: true });
      const dangerColor = await page.evaluate(() => {
        const probe = document.createElement('span');
        probe.style.color = 'var(--accent-danger)';
        document.body.appendChild(probe);
        const color = getComputedStyle(probe).color;
        probe.remove();
        return color;
      });
      await expect(remove).toHaveCSS('color', dangerColor);
      await expect(remove).toHaveCSS('backdrop-filter', 'none');
      await remove.click();
      const cancel = page.getByRole('button', { name: 'Cancel', exact: true });
      const confirm = page
        .locator('[role="dialog"]')
        .filter({ has: cancel })
        .locator('[data-ui-button="danger-outline"]');
      await expect(confirm).toHaveCSS('color', dangerColor);
      await expect(confirm).not.toHaveCSS(
        'background-color',
        await cancel.evaluate((button) => getComputedStyle(button).backgroundColor),
      );
      // 辅助功能回退后也保留危险操作颜色；测试只取消，不执行删除。
      await page
        .locator('html')
        .evaluate((root) => root.setAttribute('data-native-material', 'solid'));
      await expect(confirm).toHaveCSS('color', dangerColor);
      await expect(confirm).toHaveCSS('backdrop-filter', 'none');
      await cancel.click();
      await navigate(page, '/settings/templates');
      await page.getByRole('button', { name: 'New Template', exact: true }).click();
      const save = page.getByRole('dialog').getByRole('button', { name: 'Save', exact: true });
      await expect(save).toHaveAttribute('data-ui-button', 'primary');
      await expect(save).toHaveCSS('color', 'rgb(255, 255, 255)');
      await expect(save).toHaveCSS('backdrop-filter', 'none');
      await page.getByRole('dialog').getByRole('button', { name: 'Cancel', exact: true }).click();
    });

    for (const theme of ['light', 'dark']) {
      test(`${theme} material, selection and accessible fallback`, async ({ page }) => {
        await setup(page, platform, theme);
        await page.setViewportSize({ width: 1100, height: 800 });
        await navigate(page, '/settings/data');
        const backup = page.getByRole('button', { name: 'Create Backup', exact: true });
        await expect(backup).toHaveCSS('border-radius', platform === 'macos' ? '8px' : '4px');
        await expect(backup).toHaveCSS('backdrop-filter', 'none');
        if (platform === 'macos') {
          await expect(backup).toHaveCSS('background-image', 'none');
          await expect(backup).toHaveCSS('box-shadow', 'none');
        }
        const stateProperty = platform === 'macos' ? 'background-color' : 'background-image';
        const resting = await backup.evaluate(
          (button, property) => getComputedStyle(button).getPropertyValue(property),
          stateProperty,
        );
        await backup.hover();
        await expect(backup).not.toHaveCSS(stateProperty, resting);
        await page.mouse.down();
        await expect(backup).toHaveCSS('transform', 'none');
        await page.mouse.move(1000, 700);
        await page.mouse.up();
        await page.keyboard.press('Tab');
        await backup.focus();
        await expect(backup).toHaveCSS('outline-style', 'solid');
        await page.screenshot({ path: test.info().outputPath(`${platform}-${theme}-data.png`) });

        await page.getByRole('button', { name: 'View breakdown', exact: true }).click();
        const panel = page
          .locator('[data-macos-glass="panel"]')
          .filter({ has: page.getByRole('heading', { name: 'Storage Breakdown', exact: true }) });
        const close = panel.getByRole('button', { name: 'Close', exact: true });
        // 内容面板与内部按钮都不叠加实时模糊，正文和操作保持清晰。
        if (platform === 'macos') {
          await expect(panel).toHaveCSS('backdrop-filter', 'none');
          await expect(panel).toHaveCSS('background-image', 'none');
        }
        await expect(close).toHaveCSS('backdrop-filter', 'none');
        await close.click();
        await expect(panel).toHaveCount(0);

        const bounds = await backup.boundingBox();
        await page
          .locator('html')
          .evaluate((root) => root.setAttribute('data-native-material', 'solid'));
        await expect(backup).toHaveCSS('backdrop-filter', 'none');
        expect(await backup.boundingBox()).toEqual(bounds);
        await page
          .locator('html')
          .evaluate((root) => root.setAttribute('data-high-contrast', 'true'));
        await expect(backup).toHaveCSS('background-image', 'none');

        await page.locator('html').evaluate((root, platform) => {
          root.setAttribute('data-native-material', platform === 'macos' ? 'liquid-glass' : 'mica');
          root.setAttribute('data-high-contrast', 'false');
        }, platform);
        await navigate(page, '/settings/attachments');
        const activeAttachments = page.getByRole('button', { name: /^Active \(0\)/ });
        const trashedAttachments = page.locator('main .interactive-danger-tab');
        await expect(activeAttachments).toHaveAttribute('aria-pressed', 'true');
        await trashedAttachments.click();
        await expect(trashedAttachments).toHaveAttribute('aria-pressed', 'true');
        await expect(activeAttachments).toHaveAttribute('aria-pressed', 'false');
        const trashColor = await page.evaluate(() => {
          const probe = document.createElement('span');
          probe.style.color = 'var(--accent-danger)';
          document.body.appendChild(probe);
          const color = getComputedStyle(probe).color;
          probe.remove();
          return color;
        });
        await expect(trashedAttachments).toHaveCSS('color', trashColor);
        await navigate(page, '/plugins');
        const installed = page.getByRole('button', { name: 'Installed 0', exact: true });
        await installed.click();
        await expect(installed).toHaveAttribute('aria-pressed', 'true');
        await page.getByRole('button', { name: 'All', exact: true }).click();
        await expect(installed).toHaveAttribute('aria-pressed', 'false');
        const tier = page.getByRole('button', { name: 'P0', exact: true });
        await tier.click();
        await expect(tier).toHaveAttribute('aria-pressed', 'true');
        const selectedColor = await page.evaluate(() => {
          const probe = document.createElement('span');
          probe.style.color = 'var(--accent-primary)';
          document.body.appendChild(probe);
          const color = getComputedStyle(probe).color;
          probe.remove();
          return color;
        });
        await expect(tier).toHaveCSS('color', selectedColor);
        await page.screenshot({ path: test.info().outputPath(`${platform}-${theme}-plugins.png`) });
        await page.emulateMedia({ forcedColors: 'active' });
        await expect(tier).toHaveCSS('backdrop-filter', 'none');
        const highlight = await page.evaluate(() => {
          const probe = document.createElement('span');
          probe.style.backgroundColor = 'Highlight';
          document.body.appendChild(probe);
          const result = getComputedStyle(probe).backgroundColor;
          probe.remove();
          return result;
        });
        await expect(tier).toHaveCSS('background-color', highlight);
      });
    }
  });
}
