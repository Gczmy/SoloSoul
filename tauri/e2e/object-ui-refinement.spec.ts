import { expect, test, type Page } from '@playwright/test';
import { writeFile } from 'node:fs/promises';
import { setupTauriMock } from './fixtures/auth';

async function setup(page: Page, platform: string, theme: string, language = 'en-US') {
  await setupTauriMock(page);
  await page.addInitScript(
    ({ platform, theme, language }) => {
      const preferences = { theme, language, hasSeenOnboarding: true, autoLockTimeoutMinutes: 0 };
      const objects = Array.from({ length: 40 }, (_, index) => ({
        id: `ui-${index}`,
        name: `Object ${String(index).padStart(2, '0')}`,
        typeId: 'identity',
        properties: {
          note: 'INTERNAL_DETAIL',
          secret: 'SENSITIVE_DETAIL',
          key: 'CRITICAL_DETAIL',
          __fields: {
            note: { name: 'Note', type: 'text' },
            secret: { name: 'Secret', type: 'text' },
            key: { name: 'Key', type: 'text' },
          },
        },
        propertyLabels: { note: 'internal', secret: 'sensitive', key: 'critical' },
        sensitivityLevel: 'critical',
        createdAt: '2026-10-08',
        updatedAt: '2026-10-08',
      }));
      const loads: string[] = [];
      Object.assign(window, {
        __MOCK_PLATFORM__: platform,
        __HISTORY_LOADS__: loads,
        __E2E_MOCKS__: {
          ui_get_preferences: () => preferences,
          user_data_get_preferences: () => preferences,
          vault_check_directory: () => true,
          sync_list_conflicts: () => [],
          object_list: ({ filter }: { filter?: { typeId?: string; parentId?: string } }) =>
            filter?.parentId || filter?.typeId === 'page' ? [] : objects,
          object_get: () => objects[0],
          snapshot_count_batch: () => Object.fromEntries(objects.map((o) => [o.id, 3])),
          attachment_count_batch: () => Object.fromEntries(objects.map((o) => [o.id, 125])),
          snapshot_list: () =>
            ['one', 'two', 'three'].map((id, i) => ({
              id,
              timestamp: Date.now() - i * 1000,
              triggeredBy: 'user_edit',
              diffSummary: 'diff_updated',
            })),
          snapshot_get_data: async ({ snapshotId }: { snapshotId: string }) => {
            loads.push(snapshotId);
            await new Promise((resolve) => setTimeout(resolve, 80));
            return {
              properties: {
                note: `INTERNAL_${snapshotId}`,
                secret: 'HISTORY_SECRET',
                extra: snapshotId === 'two' ? 'Long historical text. '.repeat(500) : 'Short text',
              },
              propertyLabels: { note: 'internal', secret: 'sensitive', extra: 'public' },
            };
          },
        },
      });
      localStorage.setItem('i18nextLng', language);
    },
    { platform, theme, language },
  );
}

async function passwordBounds(page: Page, show: string) {
  const input = page.locator('[data-login-card] input[type=text]');
  await input.fill('long-password-'.repeat(80));
  const toggle = page.getByRole('button', { name: show, exact: true });
  const inputBox = (await input.boundingBox())!;
  const buttonBox = (await toggle.boundingBox())!;
  expect(inputBox.x + inputBox.width).toBeLessThanOrEqual(buttonBox.x);
  expect(inputBox.width).toBeGreaterThan(30);
  await toggle.click();
  const exposed = (await input.boundingBox())!;
  expect(exposed.x + exposed.width).toBeLessThanOrEqual(buttonBox.x);
}

test.use({ viewport: { width: 1100, height: 800 }, hasTouch: false, isMobile: false });

for (const platform of ['macos', 'windows']) {
  for (const theme of ['light', 'dark']) {
    test(`${platform} ${theme}: password, localized counts, stable cached history and scoped scrollbar`, async ({
      page,
    }, testInfo) => {
      const zh = theme === 'dark';
      await setup(page, platform, theme, zh ? 'zh-CN' : 'en-US');
      await page.goto('/login');
      await passwordBounds(page, zh ? '显示密码' : 'Show password');
      await page.locator('button[type=submit]').click();
      await page.waitForURL('/');
      await page
        .locator('#desktop-navigation')
        .getByRole('button', { name: zh ? '身份' : 'Identity', exact: true })
        .click();
      const main = page.locator('[data-shell-content]');
      await main.getByRole('button', { name: /^Object 00/ }).hover();
      const history = main
        .getByTitle(zh ? '历史记录' : 'History', { exact: true })
        .filter({ visible: true })
        .first();
      await expect(history).toContainText('3');
      const attachment = main
        .getByTitle(zh ? '附件' : 'Attachments', { exact: true })
        .filter({ visible: true })
        .first();
      await expect(attachment).toContainText('99+');
      const count = history.locator('span');
      await expect(count).toHaveCSS('background-color', 'rgba(0, 0, 0, 0)');
      await expect(count).toHaveCSS('position', 'static');
      await expect(
        main
          .getByTitle(zh ? '编辑' : 'Edit', { exact: true })
          .filter({ visible: true })
          .first(),
      ).toBeVisible();
      await expect(
        main
          .getByTitle(zh ? '移入回收站' : 'Move to trash', { exact: true })
          .filter({ visible: true })
          .first(),
      ).toBeVisible();
      await main.getByRole('button', { name: /^Object 00/ }).click();
      const detail = page.getByTestId('object-detail-modal');
      await expect(main).toHaveCSS('--scrollbar-thumb-y', 'rgb(128 128 128 / 45%)');
      await expect(detail.getByText('INTERNAL_DETAIL', { exact: true })).toBeVisible();
      expect(await detail.innerHTML()).not.toContain('SENSITIVE_DETAIL');
      expect(await detail.innerHTML()).not.toContain('CRITICAL_DETAIL');
      await detail.getByRole('button', { name: zh ? '关闭' : 'Close', exact: true }).click();
      await main.hover({ position: { x: 350, y: 100 } });
      await expect(main).toHaveCSS('--scrollbar-thumb-y', theme === 'dark' ? '#ffffff' : '#111111');
      const thumb = () =>
        main.evaluate((el) => getComputedStyle(el, '::-webkit-scrollbar-thumb').backgroundColor);
      if (test.info().project.name !== 'webkit')
        expect(await thumb()).toBe(theme === 'dark' ? 'rgb(255, 255, 255)' : 'rgb(17, 17, 17)');
      await page.locator('#desktop-navigation').hover({ position: { x: 30, y: 150 } });
      await expect(main).toHaveCSS('--scrollbar-thumb-y', 'rgb(128 128 128 / 45%)');
      if (test.info().project.name !== 'webkit')
        expect(await thumb()).toBe('rgba(128, 128, 128, 0.45)');
      // 在完整 AppShell / CSS Modules / 原生平台样式中核对连续区域切换的实际绘制。
      const paintSamples = [];
      for (let cycle = 0; cycle < 3; cycle++) {
        for (const region of ['main', 'sidebar']) {
          if (region === 'main') await main.hover({ position: { x: 350, y: 100 } });
          else await page.locator('#desktop-navigation').hover({ position: { x: 30, y: 150 } });
          if (region === 'main')
            await expect(main).toHaveCSS(
              '--scrollbar-thumb-y',
              theme === 'dark' ? '#ffffff' : '#111111',
            );
          else await expect(main).toHaveCSS('--scrollbar-thumb-y', 'rgb(128 128 128 / 45%)');
          const box = (await main.boundingBox())!;
          const path = testInfo.outputPath(`paint-app-${cycle}-${region}.png`);
          await page.screenshot({ path });
          paintSamples.push({
            path,
            x: Math.round(box.x + box.width - 3),
            y: Math.round(box.y + 60),
            active: region === 'main',
            theme,
            region,
            fullApp: true,
          });
        }
      }
      await writeFile(
        testInfo.outputPath('paint-samples.json'),
        JSON.stringify(paintSamples, null, 2),
      );
      await history.click();
      const panel = page.locator('[data-history-viewer]');
      const box = (await panel.boundingBox())!;
      await expect(panel.getByText('INTERNAL_one', { exact: true })).toBeVisible();
      expect(await panel.innerHTML()).not.toContain('HISTORY_SECRET');
      await expect(panel.getByRole('button').filter({ hasText: '••••••••' })).toHaveCSS(
        'background-color',
        'rgba(0, 0, 0, 0)',
      );
      await expect
        .poll(() =>
          page.evaluate(
            () => (window as unknown as { __HISTORY_LOADS__: string[] }).__HISTORY_LOADS__.length,
          ),
        )
        .toBe(2);
      await panel.getByTitle(zh ? '上一个' : 'Previous', { exact: true }).click();
      await expect(panel.getByText('INTERNAL_two', { exact: true })).toBeVisible();
      const nextBox = (await panel.boundingBox())!;
      expect(nextBox.height).toBe(box.height);
      expect(nextBox.y).toBe(box.y);
      const body = panel.locator('[data-history-scroll-region]');
      expect(await body.evaluate((el) => el.scrollHeight > el.clientHeight)).toBe(true);
      await body.hover({ position: { x: 120, y: 100 } });
      await expect(body).toHaveCSS('--scrollbar-thumb-y', theme === 'dark' ? '#ffffff' : '#111111');
      await expect(main).toHaveCSS('--scrollbar-thumb-y', 'rgb(128 128 128 / 45%)');
      await page.screenshot({
        path: `/tmp/solosoul-object-ui-${test.info().project.name}-${platform}-${theme}.png`,
      });
      await panel.getByTitle(zh ? '下一个' : 'Next', { exact: true }).click();
      await expect(panel.getByText('INTERNAL_one', { exact: true })).toBeVisible();
      expect((await panel.boundingBox())!.height).toBe(box.height);
      const loads = await page.evaluate(
        () => (window as unknown as { __HISTORY_LOADS__: string[] }).__HISTORY_LOADS__,
      );
      expect(loads.filter((id) => id === 'one')).toHaveLength(1);
      expect(loads.filter((id) => id === 'two')).toHaveLength(1);
    });
  }
}

for (const platform of ['android', 'ios']) {
  test(`${platform}: long password stays clear of controls at 320px`, async ({ page }) => {
    await page.setViewportSize({ width: 320, height: 844 });
    await setup(page, platform, 'light');
    await page.goto('/login');
    await passwordBounds(page, 'Show password');
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(
      320,
    );
  });
}
