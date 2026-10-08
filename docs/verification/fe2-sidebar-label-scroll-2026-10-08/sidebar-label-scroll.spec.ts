import { expect, test, type Page } from '@playwright/test';
import { login, setupTauriMock } from './fixtures/auth';

const longName = '公开页面名称和个人资料备注';
test.use({ viewport: { width: 1200, height: 800 }, isMobile: false, hasTouch: false });

async function prepare(page: Page, platform = 'macos', position = 'left', reduceMotion = false) {
  await setupTauriMock(page);
  await page.addInitScript(
    ({ platform, position, reduceMotion, longName }) => {
      const prefs = {
        theme: 'light',
        language: 'en-US',
        hasSeenOnboarding: true,
        autoLockTimeoutMinutes: 0,
        lastBackupReminderAt: Date.now(),
        sidebarPosition: position,
        reduceMotion,
      };
      const pages = ['短页', longName].map((name, index) => ({
        id: index ? 'label-long' : 'label-short',
        name,
        typeId: 'page',
        iconName: 'document',
        properties: { sortOrder: index },
        propertyLabels: {},
        parentId: null,
        templateId: null,
        templateType: null,
        isDeleted: false,
        createdAt: '2026-10-08T00:00:00Z',
        updatedAt: '2026-10-08T00:00:00Z',
      }));
      Object.assign(window, {
        __MOCK_PLATFORM__: platform,
        __E2E_MOCKS__: {
          ui_get_preferences: () => prefs,
          user_data_get_preferences: () => prefs,
          vault_check_directory: () => true,
          sync_list_conflicts: () => [],
          object_list: ({ filter }: { filter?: { typeId?: string } }) =>
            filter?.typeId === 'page' ? pages : [],
          object_get: ({ objectId }: { objectId: string }) => pages.find((p) => p.id === objectId),
          object_update: ({ objectId, input }: { objectId: string; input: object }) => {
            const item = pages.find((p) => p.id === objectId)!;
            Object.assign(item, input);
            return item;
          },
          set_titlebar_color: () => ({
            platform,
            material: platform === 'macos' ? 'liquid-glass' : 'mica',
            highContrast: false,
            reduceMotion: false,
            titlebarHeight: 52,
            trafficLightsRight: 79,
          }),
        },
      });
      localStorage.setItem('i18nextLng', 'en-US');
    },
    { platform, position, reduceMotion, longName },
  );
  await login(page);
  await expect(
    page.getByRole('navigation').getByRole('button', { name: longName, exact: true }),
  ).toBeVisible();
}

for (const platform of ['macos', 'windows']) {
  for (const position of ['left', 'right']) {
    test(`${platform} ${position}: long names scroll alone, reveal tail and reset; rename stays usable`, async ({
      page,
    }) => {
      await prepare(page, platform, position);
      const nav = page.locator('#desktop-navigation');
      const button = nav.getByRole('button', { name: longName, exact: true });
      const viewport = button.locator('[data-nav-label]');
      const track = viewport.locator('[data-label-track]');
      await expect(
        nav.getByRole('button', { name: '短页', exact: true }).locator('[data-nav-label]'),
      ).toHaveAttribute('data-overflow', 'false');
      await expect(viewport).toHaveAttribute('data-overflow', 'true');
      await expect(viewport).toHaveCSS('text-overflow', 'clip');
      expect(await viewport.evaluate((e) => getComputedStyle(e).maskImage)).toContain('gradient');
      const iconBefore = await button.locator('svg').boundingBox();
      const buttonBefore = await button.boundingBox();
      await page.screenshot({ path: test.info().outputPath('idle.png') });
      await button.hover();
      await expect(viewport).toHaveAttribute('data-scroll-state', 'waiting');
      await expect(viewport).toHaveAttribute('data-scroll-state', 'forward');
      await expect
        .poll(async () => (await track.boundingBox())!.x)
        .toBeLessThan((await viewport.boundingBox())!.x - 4);
      await expect(viewport).toHaveAttribute('data-scroll-state', 'tail', { timeout: 10000 });
      await expect(viewport).toHaveAttribute('data-fade', 'left');
      const textBox = (await track.boundingBox())!;
      const clip = (await viewport.boundingBox())!;
      expect(Math.abs(textBox.x + textBox.width - clip.x - clip.width)).toBeLessThanOrEqual(1);
      expect(await button.locator('svg').boundingBox()).toEqual(iconBefore);
      expect(await button.boundingBox()).toEqual(buttonBefore);
      await page.screenshot({ path: test.info().outputPath('tail.png') });
      await expect(viewport).toHaveAttribute('data-scroll-state', 'reverse');
      await page.mouse.move(600, 700);
      await expect(track).toHaveCSS('transform', 'none');
      await expect(viewport).not.toHaveAttribute('data-scroll-state');
      await button.click();
      await expect(page).toHaveURL(/workspace\/custom\/label-long$/);
      await button.dblclick();
      const input = page.locator('[data-macos-glass="panel"] input').first();
      await expect(input).toHaveValue(longName);
      await input.fill('新名称');
      await input.press('Enter');
      const renamed = nav.getByRole('button', { name: '新名称', exact: true });
      await expect(renamed.locator('[data-nav-label]')).toHaveAttribute('data-overflow', 'false');
      await expect(renamed.locator('[data-nav-label]')).toHaveCSS('mask-image', 'none');
      await page.getByRole('button', { name: 'Collapse sidebar', exact: true }).click();
      await expect(renamed.locator('[data-nav-label]')).toHaveCount(0);
      await renamed.hover();
      await expect(page.locator('[role="tooltip"]').filter({ hasText: '新名称' })).toBeVisible();
    });
  }
}

test('keyboard focus scrolls; system and native reduced motion cancel and show complete name', async ({
  page,
  browserName,
}) => {
  await prepare(page);
  const nav = page.locator('#desktop-navigation');
  const button = nav.getByRole('button', { name: longName, exact: true });
  const viewport = button.locator('[data-nav-label]');
  const track = viewport.locator('[data-label-track]');
  await page.mouse.move(600, 700);
  await nav.getByRole('button', { name: '短页', exact: true }).focus();
  await page.keyboard.press(browserName === 'webkit' ? 'Alt+Tab' : 'Tab');
  await expect(button).toBeFocused();
  await expect(viewport).toHaveAttribute('data-scroll-state', 'forward');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await expect(track).toHaveCSS('transform', 'none');
  await expect(page.locator('[role="tooltip"]').filter({ hasText: longName })).toBeVisible();
  await expect(page.locator('[role="tooltip"]').filter({ hasText: longName })).toHaveCSS(
    'animation-name',
    'none',
  );
  await page.emulateMedia({ reducedMotion: 'no-preference' });
  await expect(viewport).toHaveAttribute('data-scroll-state', 'forward');
  await page.locator('html').evaluate((root) => {
    root.dataset.reduceMotion = 'true';
  });
  await expect(track).toHaveCSS('transform', 'none');
  await expect(page.locator('[role="tooltip"]').filter({ hasText: longName })).toBeVisible();
  await page.locator('html').evaluate((root) => {
    delete root.dataset.reduceMotion;
  });
  await page.emulateMedia({ forcedColors: 'active' });
  await expect(viewport).toHaveCSS('mask-image', 'none');
  await expect(track).toHaveCSS('transform', 'none');
});

test('application reduced motion keeps the whole name in the hover card', async ({ page }) => {
  await prepare(page, 'macos', 'left', true);
  const button = page
    .locator('#desktop-navigation')
    .getByRole('button', { name: longName, exact: true });
  await button.hover();
  await expect(button.locator('[data-label-track]')).toHaveCSS('transform', 'none');
  await expect(page.locator('[role="tooltip"]').filter({ hasText: longName })).toBeVisible();
});

for (const position of ['top', 'bottom']) {
  test(`${position}: horizontal navigation retains its complete-name tooltip`, async ({ page }) => {
    await prepare(page, 'macos', position);
    const button = page
      .getByRole('navigation')
      .getByRole('button', { name: longName, exact: true });
    await expect(button.locator('[data-nav-label]')).toHaveCount(0);
    await button.hover();
    await expect(page.locator('[role="tooltip"]').filter({ hasText: longName })).toBeVisible();
  });
}

test('label remeasures font/container changes and falls back when animation is unavailable', async ({
  page,
}) => {
  await prepare(page);
  const button = page.getByRole('navigation').getByRole('button', { name: longName, exact: true });
  const viewport = button.locator('[data-nav-label]');
  await expect(viewport).toHaveAttribute('data-overflow', 'true');
  await button.evaluate((element) => {
    element.style.width = '400px';
  });
  await expect(viewport).toHaveAttribute('data-overflow', 'false');
  await expect(viewport).toHaveCSS('mask-image', 'none');
  await button.evaluate((element) => {
    element.style.removeProperty('width');
  });
  await expect(viewport).toHaveAttribute('data-overflow', 'true');
  await viewport.evaluate((element) => {
    element.style.fontSize = '6px';
  });
  await expect(viewport).toHaveAttribute('data-overflow', 'false');
  await viewport.evaluate((element) => {
    element.style.removeProperty('font-size');
  });
  await expect(viewport).toHaveAttribute('data-overflow', 'true');
  await button.locator('[data-label-track]').evaluate((element) => {
    Object.defineProperty(element, 'animate', { value: undefined, configurable: true });
  });
  await button.hover();
  await expect(page.locator('[role="tooltip"]').filter({ hasText: longName })).toBeVisible();
  await expect(button.locator('[data-label-track]')).toHaveCSS('transform', 'none');
});
