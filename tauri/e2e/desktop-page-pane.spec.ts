import { test, expect } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { login } from './fixtures/auth';

test.beforeEach(async ({ page }, info) => {
  page.on('pageerror', (error) => console.error('Page error:', error.message));
  page.on('response', (response) => {
    if (response.status() >= 400) console.error('HTTP:', response.status(), response.url());
  });
  page.on('console', (message) => {
    if (message.type() === 'error') console.error('Browser:', message.text());
  });
  test.skip(info.project.name !== 'chromium', 'Desktop pane; mobile navigation tested separately');
  await page.addInitScript({
    content:
      readFileSync('e2e/fixtures/tauriMock.js', 'utf8') +
      `
    window.__MOCK_PLATFORM__ = 'windows';
    localStorage.setItem('i18nextLng', 'en-US');
    const objects = Array.from({ length: 40 }, (_, i) => ({
      id: 'pane-object-' + i, name: i === 0 ? 'Passport for navigation' : i === 1 ? 'Object with a long descriptive name here' : 'Identity record ' + i,
      typeId: 'identity', sectionType: 'identity', iconName: 'id-card',
      sensitivityLevel: 'public', createdAt: '2026-10-20', updatedAt: '2026-10-20',
      isDeleted: false, templateId: null, templateType: null, tags: [], hasAttachments: false, properties: {}
    }));
    let frame = false;
    window.__E2E_MOCKS__ = {
      vault_check_directory: () => true,
      object_list: ({ filter } = {}) => filter?.typeId === 'page' ? objects.filter(o => o.typeId === 'page') : objects,
      object_get: ({ objectId }) => objects.find(o => o.id === objectId),
      object_create: ({ input }) => { const obj = { ...objects[0], ...input }; objects.push(obj); return obj; },
      ocr_get_model_status: () => ({ installed: true, bundled: true }),
      sync_list_conflicts: () => [],
      set_titlebar_color: () => ({ platform: 'windows', material: 'mica', reduceMotion: false, highContrast: false }),
      set_titlebar_controls: args => { if (args.windowsFrame !== undefined) frame = args.windowsFrame; },
      get_window_layout: () => ({ platform: 'windows', windowsFrame: frame ? {
        enabled: true, headerHeight: 52, dpi: 96, maximized: false,
        captionBounds: [window.innerWidth - 146, 0, 146, 30]
      } : undefined })
    };
  `,
  });
  await login(page);
});

test('fixed rail, persistent pane, object edit routing and floating page creation', async ({
  page,
}) => {
  const rail = page.locator('#desktop-navigation');
  const pane = page.locator('#desktop-page-pane');
  const toggle = page.locator('[data-appbar] button[aria-controls="desktop-page-pane"]');
  await expect(rail).toHaveCSS('width', '64px');
  await expect(rail).toHaveAttribute('data-expanded', 'false');
  await expect(rail.locator('button[aria-controls="desktop-navigation"]')).toHaveCount(0);
  expect(
    (await rail.getByRole('button', { name: 'Home', exact: true }).boundingBox())!.y,
  ).toBeLessThan(70);
  const identity = pane.locator('[data-page-id="identity"]');
  await expect(identity).toHaveAttribute('aria-expanded', 'true');
  await expect(pane.locator('[data-object-id]')).toHaveCount(40);
  const scroller = pane.locator('[data-page-pane-scroll]');
  const search = pane.getByRole('searchbox');
  const searchBefore = await search.boundingBox();
  const addBefore = await pane.getByRole('button', { name: 'Add Page', exact: true }).boundingBox();
  await scroller.evaluate((el) => {
    el.scrollTop = el.scrollHeight;
  });
  expect(await search.boundingBox()).toEqual(searchBefore);
  expect(await pane.getByRole('button', { name: 'Add Page', exact: true }).boundingBox()).toEqual(
    addBefore,
  );
  await search.fill('Passport');
  await pane.getByRole('button', { name: 'Passport for navigation', exact: true }).click();
  await expect(page).toHaveURL(/\/editor\/pane-object-0$/);
  await expect(pane.locator('[data-object-id="pane-object-0"]')).toHaveAttribute(
    'aria-current',
    'page',
  );
  const content = page.locator('[data-shell-content]');
  await content.evaluate((el) => {
    el.dataset.mountSentinel = 'preserved';
    el.scrollTop = 120;
  });
  await toggle.click();
  await expect(pane).toHaveAttribute('inert', '');
  await expect(rail).toHaveCSS('width', '64px');
  await toggle.click();
  await expect(search).toHaveValue('Passport');
  await expect(content).toHaveAttribute('data-mount-sentinel', 'preserved');
  await search.fill('');
  const paneBefore = await pane.boundingBox();
  await pane.getByRole('button', { name: 'Add Page', exact: true }).click();
  const input = page.getByPlaceholder('Page name', { exact: true });
  await expect(input).toBeVisible();
  await expect(input).toBeInViewport();
  expect(await input.evaluate((el) => !!el.closest('#desktop-page-pane'))).toBe(false);
  expect(await pane.boundingBox()).toEqual(paneBefore);
  await input.fill('Created from pane');
  await input.press('Enter');
  await expect(page).toHaveURL(/\/workspace\/custom\//);
  await expect(pane.getByRole('button', { name: 'Created from pane', exact: true })).toBeVisible();
});

test('resizer supports pointer capture, bounds, keyboard, reset and persistence', async ({
  page,
}) => {
  const pane = page.locator('#desktop-page-pane');
  const divider = page.locator('[role="separator"][aria-controls="desktop-page-pane"]');
  await expect(pane).toHaveCSS('width', '264px');
  let box = (await divider.boundingBox())!;
  await page.mouse.move(box.x + 4, box.y + box.height / 2);
  await page.mouse.down();
  await page.mouse.move(box.x + 800, box.y + box.height / 2, { steps: 8 });
  await page.mouse.up();
  await expect(pane).toHaveCSS('width', '400px');
  await divider.focus();
  await page.keyboard.press('Home');
  await expect(pane).toHaveCSS('width', '200px');
  await page.keyboard.press('Shift+ArrowRight');
  await expect(pane).toHaveCSS('width', '232px');
  await page.keyboard.press('ArrowRight');
  await expect(divider).toHaveAttribute('aria-valuenow', '240');
  await page.locator('[data-appbar] button[aria-controls="desktop-page-pane"]').click();
  await expect(divider).toHaveAttribute('tabindex', '-1');
  await page.locator('[data-appbar] button[aria-controls="desktop-page-pane"]').click();
  await expect(pane).toHaveCSS('width', '240px');
  await page.reload();
  await login(page);
  await expect(pane).toHaveCSS('width', '240px');
  box = (await divider.boundingBox())!;
  await page.mouse.move(box.x + 4, box.y + box.height / 2);
  await page.mouse.down();
  await page.mouse.move(box.x - 500, box.y + box.height / 2, { steps: 6 });
  await page.mouse.up();
  await expect(pane).toHaveCSS('width', '200px');
  await divider.dblclick();
  await expect(pane).toHaveCSS('width', '264px');
  await expect(page.locator('#desktop-navigation')).toHaveCSS('width', '64px');
});

test('pane and editor move together, rapid reversal preserves DOM, reduced motion settles immediately', async ({
  page,
}) => {
  await page.emulateMedia({ reducedMotion: 'no-preference' });
  const body = page.locator('[data-page-pane-body]');
  const content = page.locator('[data-shell-content]');
  await content.evaluate((el) => {
    el.dataset.motionSentinel = 'retained';
  });
  const frames = await page.evaluate(async () => {
    const toggle = document.querySelector<HTMLButtonElement>(
      '[data-appbar] button[aria-controls="desktop-page-pane"]',
    )!;
    const pane = document.querySelector<HTMLElement>('#desktop-page-pane')!;
    const content = document.querySelector<HTMLElement>('[data-shell-content]')!;
    const rail = document.querySelector<HTMLElement>('#desktop-navigation')!;
    const values = [];
    toggle.click();
    for (let i = 0; i < 14; i++) {
      await new Promise(requestAnimationFrame);
      const r = content.getBoundingClientRect();
      values.push({
        x: r.x,
        right: r.right,
        paneWidth: pane.getBoundingClientRect().width,
        railWidth: rail.getBoundingClientRect().width,
      });
    }
    toggle.click();
    await new Promise(requestAnimationFrame);
    await new Promise(requestAnimationFrame);
    toggle.click();
    return values;
  });
  expect(new Set(frames.map((frame) => Math.round(frame.x))).size).toBeGreaterThan(3);
  expect(
    frames.every(
      (frame) =>
        Math.abs(frame.right - frames[0].right) < 1 &&
        frame.paneWidth === 264 &&
        frame.railWidth === 64,
    ),
  ).toBe(true);
  await expect(body).not.toHaveAttribute('data-pane-animating');
  await expect(content).toHaveAttribute('data-motion-sentinel', 'retained');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.locator('[data-appbar] button[aria-controls="desktop-page-pane"]').click();
  await expect(body).not.toHaveAttribute('data-pane-animating');
  const x = (await content.boundingBox())!.x;
  await page.waitForTimeout(80);
  expect((await content.boundingBox())!.x).toBe(x);
});

test('long object names scroll alone and remeasure after pane resizing', async ({ page }) => {
  const row = page.locator('#desktop-page-pane [data-object-id="pane-object-1"]');
  const label = row.locator('[data-nav-label]');
  const track = row.locator('[data-label-track]');
  await expect(label).toHaveAttribute('data-overflow', 'true');
  const icon = await row.locator('svg').boundingBox();
  const rowBox = await row.boundingBox();
  await row.hover();
  await expect(label).toHaveAttribute('data-scroll-state', 'forward');
  await expect(label).toHaveAttribute('data-scroll-state', 'tail', { timeout: 10000 });
  await expect(label).toHaveAttribute('data-fade', 'left');
  const clip = (await label.boundingBox())!;
  const text = (await track.boundingBox())!;
  expect(Math.abs(clip.x + clip.width - text.x - text.width)).toBeLessThan(1);
  expect(await row.locator('svg').boundingBox()).toEqual(icon);
  expect(await row.boundingBox()).toEqual(rowBox);
  await page.getByRole('separator', { name: 'Resize pages and objects' }).focus();
  await page.keyboard.press('End');
  await expect(label).toHaveAttribute('data-overflow', 'false');
  await expect(track).toHaveCSS('transform', 'none');
});
