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

test('未悬停时刻度等长，滚动切换当前位置时整条立即高亮', async ({ page }) => {
  const ruler = page.getByRole('navigation', { name: 'Object ruler' });
  await expect(ruler.locator('[data-ruler-index]')).toHaveCount(65);
  await page.mouse.move(400, 100);
  const frames = await ruler.evaluate(async (element) => {
    const ticks = [...element.querySelectorAll<HTMLButtonElement>('[data-ruler-index]')];
    const scroller = document.querySelector('main')!;
    const probe = document.createElement('span');
    probe.style.color = 'var(--accent-primary)';
    element.append(probe);
    const accent = getComputedStyle(probe).color;
    probe.remove();
    const results = [];
    for (const index of [6, 17, 9, 30, 2]) {
      const card = document.getElementById(`workspace-object-ruler-${index}`)!;
      scroller.scrollTop +=
        card.getBoundingClientRect().top -
        scroller.getBoundingClientRect().top -
        scroller.clientHeight * 0.25 +
        1;
      // 保留动画，检查位置切换后的早期帧，避免仅验证动画结束时的外观。
      for (let frame = 0; frame < 3; frame++)
        await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
      const current = ticks.filter((tick) => tick.hasAttribute('aria-current'));
      const line = current[0].firstElementChild!;
      results.push({
        expectedIndex: String(index),
        actualIndices: current.map((tick) => tick.dataset.rulerIndex),
        widths: [
          ...new Set(ticks.map((tick) => tick.firstElementChild!.getBoundingClientRect().width)),
        ],
        opacity: getComputedStyle(line).opacity,
        color: getComputedStyle(line).backgroundColor,
        accent,
      });
    }
    return results;
  });
  for (const frame of frames) {
    expect(frame.actualIndices).toEqual([frame.expectedIndex]);
    expect(frame.widths).toHaveLength(1);
    expect(frame.opacity).toBe('1');
    expect(frame.color).toBe(frame.accent);
  }
});

test('上下滚动后高亮刻度从左到右颜色完整一致', async ({ page }) => {
  const ruler = page.getByRole('navigation', { name: 'Object ruler' });
  await expect(ruler.locator('[data-ruler-index]')).toHaveCount(65);
  await page.mouse.move(400, 100);
  for (const index of [5, 31, 12, 1]) {
    await page.locator(`#workspace-object-ruler-${index}`).evaluate((card) => {
      const scroller = card.closest('main')!;
      scroller.scrollTop +=
        card.getBoundingClientRect().top -
        scroller.getBoundingClientRect().top -
        scroller.clientHeight * 0.25 +
        1;
    });
    const tick = ruler.locator(`[data-ruler-index="${index}"]`);
    await expect(tick).toHaveAttribute('aria-current', 'location');
    const line = tick.locator('span');
    const bounds = (await line.boundingBox())!;
    const clip = {
      x: Math.floor(bounds.x) - 2,
      y: Math.floor(bounds.y) - 2,
      width: Math.ceil(bounds.width) + 4,
      height: Math.ceil(bounds.height) + 4,
    };
    const screenshot = await page.screenshot({ clip, animations: 'allow', scale: 'css' });
    const pixels = await line.evaluate(
      async (element, { png, bounds, clip }) => {
        const image = new Image();
        image.src = `data:image/png;base64,${png}`;
        await image.decode();
        const canvas = document.createElement('canvas');
        canvas.width = image.width;
        canvas.height = image.height;
        const context = canvas.getContext('2d')!;
        context.drawImage(image, 0, 0);
        // 跳过圆角的抗锯齿端点，逐像素检查整条中线，覆盖“只亮半条”的绘制问题。
        const y = Math.floor(bounds.y - clip.y + bounds.height / 2);
        const colors = [];
        for (
          let x = Math.ceil(bounds.x - clip.x + 2);
          x < bounds.x - clip.x + bounds.width - 2;
          x++
        )
          colors.push([...context.getImageData(x, y, 1, 1).data].slice(0, 3));
        const expected = getComputedStyle(element).backgroundColor.match(/\d+/g)!.map(Number);
        return { colors, expected: expected.slice(0, 3) };
      },
      { png: screenshot.toString('base64'), bounds, clip },
    );
    expect(pixels.colors.length).toBeGreaterThan(4);
    for (const color of pixels.colors) expect(color).toEqual(pixels.expected);
  }
});

test('尺标滚动时上下边缘的刻度两端完整可见', async ({ page }) => {
  const ruler = page.getByRole('navigation', { name: 'Object ruler' });
  await expect(ruler.locator('[data-ruler-index]')).toHaveCount(65);
  const clipped = await ruler.evaluate(async (element) => {
    const ticks = [...element.querySelectorAll<HTMLButtonElement>('[data-ruler-index]')];
    const rail = ticks[0].parentElement!;
    const failures: { scrollTop: number; index: string | undefined; edge: string }[] = [];
    const offsets = [1, 3, 9, 15, rail.scrollHeight - rail.clientHeight - 3];
    for (const offset of offsets) {
      rail.scrollTop = offset;
      await new Promise<void>((resolve) =>
        requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
      );
      const bounds = rail.getBoundingClientRect();
      for (const tick of ticks) {
        const line = tick.firstElementChild!;
        const r = line.getBoundingClientRect();
        if (r.bottom <= bounds.top || r.top >= bounds.bottom) continue;
        if (r.top < bounds.top || r.bottom > bounds.bottom) {
          failures.push({
            scrollTop: rail.scrollTop,
            index: tick.dataset.rulerIndex,
            edge: 'vertical',
          });
          continue;
        }
        for (const [edge, x] of [
          ['left', r.left + 0.5],
          ['right', r.right - 0.5],
        ] as const) {
          if (!tick.contains(document.elementFromPoint(x, r.top + r.height / 2))) {
            failures.push({ scrollTop: rail.scrollTop, index: tick.dataset.rulerIndex, edge });
          }
        }
      }
    }
    return failures;
  });
  expect(clipped).toEqual([]);
});

test('尺标触控板小幅输入累计为完整刻度，缩放后也保持整格', async ({ page }) => {
  const ruler = page.getByRole('navigation', { name: 'Object ruler' });
  const rail = ruler.locator('[data-ruler-index]').first().locator('..');
  await expect(ruler.locator('[data-ruler-index]')).toHaveCount(65);
  await rail.hover();
  await rail.evaluate((element) => {
    element.scrollTop = 0;
  });
  await page.mouse.wheel(0, 3);
  await page.mouse.wheel(0, 4);
  await expect.poll(() => rail.evaluate((element) => element.scrollTop)).toBe(6);
  await page.mouse.wheel(0, -2);
  await page.mouse.wheel(0, -5);
  await expect.poll(() => rail.evaluate((element) => element.scrollTop)).toBe(0);
  await page.setViewportSize({ width: 1280, height: 407 });
  await expect(rail).toHaveCSS('height', '294px');
  await rail.hover();
  await page.mouse.wheel(0, 19);
  await expect.poll(() => rail.evaluate((element) => element.scrollTop)).toBe(18);
  await page.screenshot({
    path: 'test-results/object-ruler-whole-ticks.png',
    animations: 'disabled',
  });
});
