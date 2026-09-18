import { expect, test, type Page } from '@playwright/test';
import { login, setupTauriMock } from './fixtures/auth';
import type { PluginInstallProgress } from '../src/lib/plugin';

async function openPluginPanel(page: Page) {
  await page
    .locator('#desktop-navigation')
    .getByRole('button', { name: 'Tools', exact: true })
    .hover();
  await page
    .locator('#desktop-navigation')
    .getByRole('button', { name: 'Plugins', exact: true })
    .click();
}

async function setupPlugin(page: Page, platform: 'macos' | 'windows', initiallyInstalled = true) {
  await setupTauriMock(page);
  await page.addInitScript(
    ({ platform, initiallyInstalled }) => {
      let installed = initiallyInstalled;
      let operationId = 0;
      let pending:
        | { rid: number; resolve: () => void; reject: (error: string) => void }
        | undefined;
      const installCalls: string[] = [];
      let reportProgress: ((progress: PluginInstallProgress) => void) | undefined;
      const plugin = {
        id: 'com.solosoul.official.address-fmt',
        name: 'Address Formatter',
        version: '1.0.0',
        author: 'SoloSoul',
        description: 'Format addresses',
        tier: 'p1',
        category: 'productivity',
        permissions: [],
      };
      const prefs = {
        language: 'en-US',
        hasSeenOnboarding: true,
        autoLockTimeoutMinutes: 0,
        sidebarButtonModes: { plugins: 'card' },
        lastBackupAt: new Date().toISOString(),
      };
      Object.assign(window, {
        __MOCK_PLATFORM__: platform,
        __E2E_UNINSTALL_CALLS__: [] as string[],
        __E2E_INSTALL_CALLS__: installCalls,
        __E2E_INSTALL_PROGRESS__: (
          percent: number,
          phase: PluginInstallProgress['phase'] = 'downloading',
        ) => {
          reportProgress?.({ percent, phase, downloadedBytes: percent * 10, totalBytes: 1000 });
        },
        __E2E_FINISH_INSTALL__: (error?: string) => {
          const operation = pending;
          pending = undefined;
          if (!operation) throw new Error('No pending install');
          if (error) operation.reject(error);
          else {
            installed = true;
            operation.resolve();
          }
        },
        __E2E_MOCKS__: {
          ui_get_preferences: () => prefs,
          user_data_get_preferences: () => prefs,
          vault_check_directory: () => true,
          ocr_get_model_status: () => ({ installed: true, bundled: true }),
          sync_list_conflicts: () => [],
          plugin_list_all: () => [
            {
              pluginId: plugin.id,
              installedVersion: installed ? plugin.version : undefined,
              hasUpdate: false,
              isCompatible: true,
              tier: plugin.tier,
              category: plugin.category,
              registryEntry: { ...plugin, latestVersion: plugin.version, params: [] },
            },
          ],
          plugin_list_installed: () => (installed ? [plugin] : []),
          create_plugin_install: () => ++operationId,
          plugin_install: ({
            pluginId,
            operationId,
            onProgress,
          }: {
            pluginId: string;
            operationId: number;
            onProgress: { onmessage: (progress: PluginInstallProgress) => void };
          }) => {
            installCalls.push(pluginId);
            reportProgress = (progress) => onProgress.onmessage(progress);
            return new Promise<void>((resolve, reject) => {
              pending = { rid: operationId, resolve, reject };
            });
          },
          'plugin:resources|close': ({ rid }: { rid: number }) => {
            if (pending?.rid === rid) {
              const operation = pending;
              pending = undefined;
              operation.reject('PLUGIN_INSTALL_CANCELLED');
            }
          },
          plugin_uninstall: ({ pluginId }: { pluginId: string }) => {
            (
              window as unknown as { __E2E_UNINSTALL_CALLS__: string[] }
            ).__E2E_UNINSTALL_CALLS__.push(pluginId);
            installed = false;
          },
        },
      });
      localStorage.setItem('i18nextLng', 'en-US');
    },
    { platform, initiallyInstalled },
  );
  await login(page);
  await openPluginPanel(page);
}

async function expectInstallCalls(page: Page, count: number) {
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          (window as unknown as { __E2E_INSTALL_CALLS__: string[] }).__E2E_INSTALL_CALLS__.length,
      ),
    )
    .toBe(count);
}

async function finishInstall(page: Page, error?: string) {
  await page.evaluate(
    (error) =>
      (
        window as unknown as { __E2E_FINISH_INSTALL__: (error?: string) => void }
      ).__E2E_FINISH_INSTALL__(error),
    error,
  );
}

async function reportInstallProgress(
  page: Page,
  percent: number,
  phase: PluginInstallProgress['phase'] = 'downloading',
) {
  await page.evaluate(
    ({ percent, phase }) =>
      (
        window as unknown as {
          __E2E_INSTALL_PROGRESS__: (
            percent: number,
            phase: PluginInstallProgress['phase'],
          ) => void;
        }
      ).__E2E_INSTALL_PROGRESS__(percent, phase),
    { percent, phase },
  );
}

for (const platform of ['macos', 'windows'] as const) {
  test(`${platform} 侧栏安装显示进度，重新打开后可取消，失败后可重试`, async ({ page }) => {
    await setupPlugin(page, platform, false);
    const panel = page.getByRole('dialog', { name: 'Plugins', exact: true });
    const install = panel.getByRole('button', { name: 'Install', exact: true });
    const cancel = panel.getByRole('button', { name: 'Cancel installation', exact: true });
    await install.click();
    await expectInstallCalls(page, 1);
    await expect(cancel).toBeVisible();
    await expect(install).toHaveCount(0);
    const ring = panel.getByRole('progressbar');
    await expect(ring).toHaveAttribute('aria-valuenow', '0');
    for (const percent of [25, 50]) {
      await reportInstallProgress(page, percent);
      await expect(ring).toHaveAttribute('aria-valuenow', String(percent));
      await expect(ring.locator('circle').last()).toHaveAttribute(
        'stroke-dashoffset',
        String(100 - percent),
      );
      await expect(ring.locator('circle').last()).toHaveCSS(
        'stroke-dashoffset',
        `${100 - percent}px`,
      );
      await expect(ring.locator('circle').last()).toHaveCSS('animation-name', 'none');
    }
    await page.screenshot({ path: test.info().outputPath('install-progress-50.png') });
    await page.keyboard.press('Escape');
    await expect(panel).toHaveCount(0);
    await openPluginPanel(page);
    await expect(cancel).toBeVisible();
    await expect(ring).toHaveAttribute('aria-valuenow', '50');
    await expectInstallCalls(page, 1);
    await cancel.click();
    await expect(install).toBeVisible();
    await expect(panel.getByRole('alert')).toHaveCount(0);
    await install.click();
    await expectInstallCalls(page, 2);
    await finishInstall(page, 'Download timed out');
    await expect(panel.getByRole('alert')).toContainText('Download timed out');
    await expect(install).toBeVisible();
    await install.click();
    await expectInstallCalls(page, 3);
    await expect(panel.getByRole('alert')).toHaveCount(0);
    await reportInstallProgress(page, 98, 'finalizing');
    await expect(ring).toHaveAttribute('aria-valuenow', '98');
    await finishInstall(page);
    await expect(ring).toHaveAttribute('aria-valuenow', '100');
    await expect(
      panel.getByRole('button', { name: 'Installation complete', exact: true }),
    ).toBeDisabled();
    await expect(ring).toHaveCount(0);
    await expect(cancel).toHaveCount(0);
    await expect(panel.getByRole('button', { name: 'Uninstall', exact: true })).toBeVisible();
    await panel.getByRole('button', { name: 'Installed 1', exact: true }).click();
    await expect(panel.getByText('Address Formatter', { exact: true })).toBeVisible();
  });

  test(`${platform} 侧栏卸载确认执行一次，列表与重新打开后的状态同步`, async ({ page }) => {
    await setupPlugin(page, platform);
    const panel = page.getByRole('dialog', { name: 'Plugins', exact: true });
    const uninstall = panel.getByRole('button', { name: 'Uninstall', exact: true });
    await expect(uninstall).toBeVisible();
    await uninstall.click();
    // 必须使用真实的 mousedown → mouseup → click，才能覆盖 Portal 被父面板提前卸载的故障。
    await page.getByRole('button', { name: 'Confirm', exact: true }).click();
    await expect
      .poll(() =>
        page.evaluate(
          () =>
            (window as unknown as { __E2E_UNINSTALL_CALLS__: string[] }).__E2E_UNINSTALL_CALLS__,
        ),
      )
      .toEqual(['com.solosoul.official.address-fmt']);
    await expect(panel).toBeVisible();
    await expect(panel.getByRole('button', { name: 'Install', exact: true })).toBeVisible();
    await expect(uninstall).toHaveCount(0);
    await panel.getByRole('button', { name: 'Installed', exact: true }).click();
    await expect(panel.getByText('Address Formatter', { exact: true })).toHaveCount(0);
    await panel.getByRole('button', { name: 'All', exact: true }).click();
    await page.getByRole('heading', { name: 'Home', exact: true }).click();
    await expect(panel).toHaveCount(0);
    await page
      .locator('#desktop-navigation')
      .getByRole('button', { name: 'Tools', exact: true })
      .hover();
    await page
      .locator('#desktop-navigation')
      .getByRole('button', { name: 'Plugins', exact: true })
      .click();
    await expect(panel.getByRole('button', { name: 'Install', exact: true })).toBeVisible();
  });

  test(`${platform} 取消和 Escape 只关闭卸载确认，不关闭侧栏卡片`, async ({ page }) => {
    await setupPlugin(page, platform);
    const panel = page.getByRole('dialog', { name: 'Plugins', exact: true });
    const uninstall = panel.getByRole('button', { name: 'Uninstall', exact: true });
    for (const cancel of ['button', 'escape', 'backdrop']) {
      await uninstall.click();
      await expect(
        page.getByRole('heading', { name: 'Uninstall plugin', exact: true }),
      ).toBeVisible();
      if (cancel === 'button')
        await page.getByRole('button', { name: 'Cancel', exact: true }).click();
      else if (cancel === 'escape') await page.keyboard.press('Escape');
      else await page.locator('[data-macos-glass-backdrop]').click({ position: { x: 5, y: 5 } });
      await expect(
        page.getByRole('heading', { name: 'Uninstall plugin', exact: true }),
      ).toHaveCount(0);
      await expect(uninstall).toBeVisible();
    }
    expect(
      await page.evaluate(
        () => (window as unknown as { __E2E_UNINSTALL_CALLS__: string[] }).__E2E_UNINSTALL_CALLS__,
      ),
    ).toEqual([]);
    // 没有子确认框时，原本的 Escape 关闭仍然有效。
    await page.keyboard.press('Escape');
    await expect(panel).toHaveCount(0);
  });
}

test('安装进度从侧栏同步到完整页面，减少动态效果不影响真实进度', async ({ page }) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await setupPlugin(page, 'macos', false);
  const panel = page.getByRole('dialog', { name: 'Plugins', exact: true });
  await panel.getByRole('button', { name: 'Install', exact: true }).click();
  await expectInstallCalls(page, 1);
  await reportInstallProgress(page, 50);
  await panel.getByRole('button', { name: 'View All', exact: true }).click();
  await expect(page).toHaveURL('/plugins');
  const ring = page.getByRole('progressbar', { name: 'Installation progress', exact: true });
  await expect(ring).toHaveAttribute('aria-valuenow', '50');
  await reportInstallProgress(page, 75);
  await expect(ring).toHaveAttribute('aria-valuenow', '75');
  // 全局减少动态效果规则允许 0.01ms 的兼容值，同样应立即到达真实进度。
  expect(
    await ring
      .locator('circle')
      .last()
      .evaluate((element) => parseFloat(getComputedStyle(element).transitionDuration)),
  ).toBeLessThanOrEqual(0.001);
  await expect(ring.locator('circle').last()).toHaveAttribute('stroke-dashoffset', '25');
  await expect(page.getByRole('button', { name: 'Cancel installation', exact: true })).toHaveCSS(
    'border-width',
    '0px',
  );
  await page.screenshot({ path: test.info().outputPath('full-page-install-progress-75.png') });
  await page.getByRole('button', { name: 'Cancel installation', exact: true }).click();
  await expect(ring).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Install', exact: true })).toBeVisible();
  await expectInstallCalls(page, 1);
});
