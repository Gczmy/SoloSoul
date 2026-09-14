import { expect, test } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { login } from './fixtures/auth';

test.beforeEach(async ({ page }, testInfo) => {
  await page.addInitScript({
    content:
      readFileSync('e2e/fixtures/tauriMock.js', 'utf8') +
      `
    localStorage.setItem('i18nextLng', 'en-US');
    window.__E2E_MOCKS__ = {
      vault_check_directory: () => true,
      ocr_get_model_status: () => ({ installed: true, bundled: true }),
      sync_list_conflicts: () => [],
      object_list: (args) => args.filter?.typeId === 'page' ? [] : Array.from({ length: 65 }, (_, i) => ({
        id: 'ruler-' + i, name: 'Object ' + String(i + 1).padStart(2, '0'), typeId: 'identity',
        sensitivityLevel: 'internal', createdAt: '2026-09-14T00:00:00Z', updatedAt: '2026-09-14T00:00:00Z',
        properties: { city: 'Kyoto ' + (i + 1), passport: 'SECRET-PASSPORT-' + i, note: 'SECRET-NOTE-' + i },
        propertyLabels: { city: 'public', passport: 'critical' }
      })),
    };
    const originalInvoke = window.__TAURI_INTERNALS__.invoke;
    window.__TAURI_INTERNALS__.invoke = async (cmd, args) => {
      const result = await originalInvoke(cmd, args);
      if (cmd === 'user_data_get_preferences') result.sidebarPosition = '${testInfo.title.startsWith('右侧') ? 'right' : 'left'}';
      return result;
    };
  `,
  });
  await login(page);
  await page
    .locator('#desktop-navigation')
    .getByRole('button', { name: 'Identity', exact: true })
    .click();
});

test('尺标预览、跨加载批次定位及搜索结果同步', async ({ page }) => {
  const ruler = page.getByRole('navigation', { name: 'Object ruler' });
  const ticks = ruler.locator('[data-ruler-index]');
  const cards = page.getByTestId('workspace-object-card');
  await expect(cards).toHaveCount(50);
  await expect(ticks).toHaveCount(65);
  await ticks.nth(2).hover();
  const preview = page.getByRole('region', { name: 'Object preview' });
  await expect(preview).toBeVisible();
  await expect(preview).toContainText('Object 03');
  await expect(preview).toContainText('Kyoto 3');
  expect(await preview.innerHTML()).not.toContain('SECRET-');
  await page.screenshot({ path: 'test-results/object-ruler-preview.png', animations: 'disabled' });
  await preview.getByRole('button', { name: 'Locate this object' }).click();
  await expect(page.locator('#workspace-object-ruler-2')).toHaveAttribute(
    'data-ruler-target',
    'true',
  );
  await expect(page.locator('#workspace-object-ruler-2')).toBeInViewport();
  await expect(page.getByRole('dialog')).toHaveCount(0);

  await ticks.first().focus();
  await page.keyboard.press('End');
  await expect(ticks.last()).toBeFocused();
  await expect(preview).toContainText('Object 65');
  await page.keyboard.press('Enter');
  await expect(cards).toHaveCount(65);
  const last = page.locator('#workspace-object-ruler-64');
  await expect(last).toBeInViewport();
  await expect(last).toHaveAttribute('data-ruler-target', 'true');
  await expect(ticks.last()).toHaveAttribute('aria-current', 'location');
  await expect(last.locator('[role="button"]').first()).toBeFocused();

  const search = page.getByPlaceholder('Search objects…');
  await search.fill('Object 0');
  await expect(ticks).toHaveCount(9);
  await expect(cards).toHaveCount(9);
  await expect(preview).toHaveCount(0);
  await search.fill('does not exist');
  await expect(ruler).toHaveCount(0);
});

test('右侧导航和窄视口保持正确位置，减少动态效果时立即定位', async ({ page }) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  const ruler = page.getByRole('navigation', { name: 'Object ruler' });
  await expect(ruler).toHaveCSS('right', '240px');
  const tick = ruler.locator('[data-ruler-index="15"]');
  await tick.hover();
  const preview = page.getByRole('region', { name: 'Object preview' });
  await expect(preview).toHaveCSS('right', '280px');
  await tick.click();
  const target = page.locator('#workspace-object-ruler-15');
  await expect(target).toBeInViewport();
  await page.setViewportSize({ width: 390, height: 844 });
  await expect(ruler).toHaveCount(0);
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBe(390);
});
