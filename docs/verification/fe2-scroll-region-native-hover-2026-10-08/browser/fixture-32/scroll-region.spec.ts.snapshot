import { expect, test, type Locator, type Page } from '@playwright/test';
import { writeFile } from 'node:fs/promises';

const INACTIVE_THUMB = 'rgb(128 128 128 / 45%)';

/** 使用可见窗口坐标，只移动鼠标；locator.hover() 可能先滚动目标。 */
async function moveInside(page: Page, locator: Locator, x = 40, y = 40) {
  const box = (await locator.boundingBox())!;
  await page.mouse.move(box.x + x, box.y + y);
}

async function expectThumb(locator: Locator, axis: 'x' | 'y', active: boolean) {
  const expected = active
    ? await locator.evaluate((el) => getComputedStyle(el).getPropertyValue('--text-primary').trim())
    : INACTIVE_THUMB;
  await expect
    .poll(() =>
      locator.evaluate(
        (el, axis) => getComputedStyle(el).getPropertyValue(`--scrollbar-thumb-${axis}`).trim(),
        axis,
      ),
    )
    .toBe(expected);
  // 原生 thumb:hover 的伪元素计算样式可能使用宿主 hover；实际绘制另采样截图核验。
}

test.use({ viewport: { width: 1000, height: 700 }, hasTouch: false, isMobile: false });

test.beforeEach(async ({ page }) => {
  await page.goto('/e2e/fixtures/scrollRegion.html');
  await expect(page.locator('html')).toHaveAttribute('data-scroll-fixture-ready', 'true');
  await expect(page.locator('html')).toHaveAttribute('data-scrollbar-region-mode', 'hover');
});

test('pure mouse movement switches regions even when all JS move events are blocked', async ({
  page,
}) => {
  // 在应用加载前阻断 window capture，document capture 也接收不到这些事件。
  await page.addInitScript(() => {
    const events = { moves: 0, clicks: 0, wheels: 0 };
    Object.assign(window, { __SCROLL_INPUT_EVENTS__: events });
    for (const type of [
      'pointermove',
      'pointerover',
      'pointerout',
      'mousemove',
      'mouseover',
      'mouseout',
    ])
      window.addEventListener(
        type,
        (event) => {
          events.moves++;
          event.stopImmediatePropagation();
        },
        true,
      );
    window.addEventListener('click', () => events.clicks++, true);
    window.addEventListener('wheel', () => events.wheels++, true);
  });
  await page.reload();
  await expect(page.locator('html')).toHaveAttribute('data-scrollbar-region-mode', 'hover');
  await page.evaluate(() => {
    const chrome = document.createElement('div');
    chrome.id = 'chrome-space';
    chrome.style.cssText = 'position:fixed;inset:0 0 auto;height:16px;z-index:10000';
    document.body.append(chrome);
  });
  for (let cycle = 0; cycle < 3; cycle++) {
    for (const region of ['main', 'sidebar', 'tools', 'chrome']) {
      await moveInside(
        page,
        page.locator(
          region === 'chrome'
            ? '#chrome-space'
            : region === 'tools'
              ? '#tools-scroll'
              : `#scroll-${region}`,
        ),
        region === 'main' || region === 'chrome' ? 500 : 40,
        region === 'sidebar' ? 400 : region === 'chrome' ? 8 : 40,
      );
      for (const id of ['main', 'sidebar', 'tools'])
        await expectThumb(
          page.locator(id === 'tools' ? '#tools-scroll' : `#scroll-${id}`),
          'y',
          id === region,
        );
    }
  }
  const events = await page.evaluate(
    () =>
      (
        window as unknown as {
          __SCROLL_INPUT_EVENTS__: { moves: number; clicks: number; wheels: number };
        }
      ).__SCROLL_INPUT_EVENTS__,
  );
  expect(events.moves).toBeGreaterThan(0);
  expect(events.clicks).toBe(0);
  expect(events.wheels).toBe(0);
  await expect(page.locator('[data-scrollbar-active]')).toHaveCount(0);
});

test('horizontal child keeps only the effective vertical parent highlighted', async ({ page }) => {
  const main = page.locator('#scroll-main');
  const horizontal = page.locator('#horizontal');
  await expect(horizontal).toHaveAttribute('data-scrollbar-region-x', 'true');
  await expect(horizontal).not.toHaveAttribute('data-scrollbar-region-y');
  await moveInside(page, horizontal, 40, 25);
  await expectThumb(horizontal, 'x', true);
  await expectThumb(horizontal, 'y', false);
  await expectThumb(main, 'y', true);
  await moveInside(page, page.locator('#scroll-sidebar'), 40, 400);
  await expectThumb(main, 'y', false);
  await expectThumb(horizontal, 'x', false);
});

test('nearest same-axis child owns highlight and non-overflowing child yields it', async ({
  page,
}) => {
  const sidebar = page.locator('#scroll-sidebar');
  const tools = page.locator('#tools-scroll');
  await moveInside(page, tools, 25, 35);
  await expectThumb(tools, 'y', true);
  await expectThumb(sidebar, 'y', false);
  await expect(tools).toHaveCSS('scrollbar-width', 'auto');
  await tools.locator('> div').evaluate((el) => {
    el.style.height = '40px';
  });
  await expect(tools).not.toHaveAttribute('data-scrollbar-region-y');
  await expectThumb(tools, 'y', false);
  await expectThumb(sidebar, 'y', true);
});

test('each nested axis paints independently when parent and child can scroll', async ({
  page,
}, testInfo) => {
  const main = page.locator('#scroll-main');
  const child = page.locator('#horizontal');
  await main
    .locator('> div')
    .last()
    .evaluate((el) => {
      el.style.width = '1800px';
    });
  await expect(main).toHaveAttribute('data-scrollbar-region-x', 'true');
  await moveInside(page, main, 500, 300);
  await expectThumb(main, 'x', true);
  await expectThumb(main, 'y', true);
  await page.screenshot({ path: testInfo.outputPath('all-axes.png') });
  await moveInside(page, child, 40, 25);
  await expectThumb(main, 'x', false);
  await expectThumb(main, 'y', true);
  await expectThumb(child, 'x', true);
  await page.screenshot({ path: testInfo.outputPath('vertical-parent-horizontal-child.png') });
  await child.evaluate((el) => {
    el.style.overflowY = 'auto';
    (el.firstElementChild as HTMLElement).style.height = '500px';
  });
  await expect(child).toHaveAttribute('data-scrollbar-region-y', 'true');
  await expectThumb(child, 'x', true);
  await expectThumb(child, 'y', true);
  await expectThumb(main, 'x', false);
  await expectThumb(main, 'y', false);
});

test('keyboard focus and synthetic events cannot replace native hover', async ({ page }) => {
  const main = page.locator('#scroll-main');
  const sidebar = page.locator('#scroll-sidebar');
  await page.locator('#main-focus').focus();
  await moveInside(page, sidebar, 40, 400);
  await page.keyboard.press('ArrowDown');
  await main.dispatchEvent('mousemove', { clientX: 700, clientY: 300 });
  await page.evaluate(() => {
    window.dispatchEvent(new Event('blur'));
    window.dispatchEvent(new Event('focus'));
  });
  await expectThumb(sidebar, 'y', true);
  await expectThumb(main, 'y', false);
  await moveInside(page, main, 500, 300);
  await expectThumb(main, 'y', true);
  await expectThumb(sidebar, 'y', false);
});

test('style-only overlay changes recompute stationary native hover', async ({ page }) => {
  await page.evaluate(() => {
    const overlay = document.createElement('div');
    overlay.id = 'scroll-overlay';
    overlay.setAttribute('data-macos-glass-backdrop', '');
    overlay.style.cssText = 'position:fixed;inset:0;z-index:100000;display:none';
    document.body.append(overlay);
  });
  const main = page.locator('#scroll-main');
  await moveInside(page, main, 500, 200);
  await expectThumb(main, 'y', true);
  await page.locator('#scroll-overlay').evaluate((el) => {
    el.style.display = 'block';
  });
  await expectThumb(main, 'y', false);
  await page.locator('#scroll-overlay').evaluate((el) => {
    el.style.display = 'none';
  });
  await expectThumb(main, 'y', true);
});

for (const boundary of ['role', 'aria-modal', 'data-macos-glass-backdrop']) {
  test(`nested ${boundary} boundary suppresses hover outside dialog`, async ({ page }) => {
    const main = page.locator('#scroll-main');
    await main.evaluate((el, boundary) => {
      const dialog = document.createElement('div');
      dialog.id = 'nested-dialog';
      dialog.setAttribute(
        boundary,
        boundary === 'role' ? 'dialog' : boundary === 'aria-modal' ? 'true' : '',
      );
      dialog.style.cssText =
        'position:fixed;left:350px;top:150px;width:220px;height:220px;padding:20px;z-index:10000';
      const body = document.createElement('div');
      body.id = 'dialog-scroll';
      body.style.cssText = 'height:160px;overflow:auto';
      const content = document.createElement('div');
      content.style.height = '1000px';
      content.textContent = 'Dialog content';
      body.append(content);
      dialog.append(body);
      el.append(dialog);
    }, boundary);
    const dialog = page.locator('#nested-dialog');
    const body = page.locator('#dialog-scroll');
    await expect(body).toHaveAttribute('data-scrollbar-region-y', 'true');
    await moveInside(page, body, 40, 40);
    expect(await main.evaluate((el) => el.matches(':hover'))).toBe(true);
    await expectThumb(body, 'y', true);
    await expectThumb(main, 'y', false);
    await moveInside(page, dialog, 5, 5);
    await expectThumb(body, 'y', false);
    await expectThumb(main, 'y', false);
  });
}

test('content becoming scrollable updates each axis without moving pointer', async ({ page }) => {
  const main = page.locator('#scroll-main');
  const content = main.locator('> div').last();
  await content.evaluate((el) => {
    el.style.height = '20px';
  });
  await expect(main).not.toHaveAttribute('data-scrollbar-region-y');
  await moveInside(page, main, 500, 300);
  await expectThumb(main, 'y', false);
  await content.evaluate((el) => {
    el.style.height = '2000px';
    el.style.width = '1800px';
  });
  await expect(main).toHaveAttribute('data-scrollbar-region-x', 'true');
  await expect(main).toHaveAttribute('data-scrollbar-region-y', 'true');
  await expectThumb(main, 'x', true);
  await expectThumb(main, 'y', true);
  await content.evaluate((el) => {
    el.style.width = 'auto';
    el.style.height = '20px';
  });
  await expect(main).not.toHaveAttribute('data-scrollbar-region-x');
  await expect(main).not.toHaveAttribute('data-scrollbar-region-y');
  await expectThumb(main, 'x', false);
  await expectThumb(main, 'y', false);
});

test('text node updates and viewport resizing refresh overflow registration', async ({ page }) => {
  await page.locator('#scroll-main').evaluate((el) => {
    const region = document.createElement('div');
    region.id = 'dynamic-text-scroll';
    region.style.cssText =
      'position:fixed;left:500px;top:300px;width:160px;height:80px;overflow:auto';
    region.append(document.createTextNode('Short text'));
    el.append(region);
  });
  const region = page.locator('#dynamic-text-scroll');
  await moveInside(page, region, 40, 40);
  await expect(region).not.toHaveAttribute('data-scrollbar-region-y');
  await region.evaluate((el) => {
    el.firstChild!.textContent = 'Long text with spaces. '.repeat(100);
  });
  await expect(region).toHaveAttribute('data-scrollbar-region-y', 'true');
  await expectThumb(region, 'y', true);
  await expectThumb(page.locator('#scroll-main'), 'y', false);
  await region.evaluate((el) => {
    el.firstChild!.textContent = 'Short text';
  });
  await expect(region).not.toHaveAttribute('data-scrollbar-region-y');
  await region.evaluate((el) => el.remove());
  await page
    .locator('#scroll-main > div')
    .last()
    .evaluate((el) => {
      el.style.height = '20px';
    });
  await expect(page.locator('#scroll-main')).not.toHaveAttribute('data-scrollbar-region-y');
  await page.setViewportSize({ width: 1000, height: 200 });
  await expect(page.locator('#scroll-main')).toHaveAttribute('data-scrollbar-region-y', 'true');
});

test('textarea input changes yield vertical axis and input/select remain excluded', async ({
  page,
}) => {
  const main = page.locator('#scroll-main');
  const textarea = page.locator('#text-scroll');
  await moveInside(page, textarea, 40, 40);
  await expectThumb(textarea, 'y', true);
  await expectThumb(main, 'y', false);
  await textarea.evaluate((el) => {
    (el as HTMLTextAreaElement).value = 'Short text';
    el.dispatchEvent(new Event('input', { bubbles: true }));
  });
  await expect(textarea).not.toHaveAttribute('data-scrollbar-region-y');
  await expectThumb(textarea, 'y', false);
  await expectThumb(main, 'y', true);
  await textarea.evaluate((el) => {
    (el as HTMLTextAreaElement).value = 'Long line\n'.repeat(40);
    el.dispatchEvent(new Event('input', { bubbles: true }));
  });
  await expect(textarea).toHaveAttribute('data-scrollbar-region-y', 'true');
  await expectThumb(textarea, 'y', true);
  await main.evaluate((el) => {
    const input = document.createElement('input');
    input.id = 'overflow-input';
    input.value = 'Long input text '.repeat(100);
    input.style.cssText = 'display:block;width:100px;overflow:auto';
    const select = document.createElement('select');
    select.id = 'overflow-select';
    select.multiple = true;
    select.style.cssText = 'display:block;height:50px;overflow:auto';
    select.innerHTML = '<option>Option</option>'.repeat(20);
    el.prepend(input, select);
  });
  for (const id of ['overflow-input', 'overflow-select']) {
    const control = page.locator(`#${id}`);
    await moveInside(page, control, 10, 10);
    await expect(control).not.toHaveAttribute('data-scrollbar-region-x');
    await expect(control).not.toHaveAttribute('data-scrollbar-region-y');
    await expectThumb(control, 'x', false);
    await expectThumb(control, 'y', false);
    await expectThumb(main, 'y', true);
  }
});

test('platform registration works before desktop material initialization', async ({ page }) => {
  const root = page.locator('html');
  await root.evaluate((el) => {
    el.removeAttribute('data-desktop-platform');
    el.setAttribute('data-platform', 'windows');
  });
  await expect(root).not.toHaveAttribute('data-desktop-platform');
  await expect(root).toHaveAttribute('data-scrollbar-region-mode', 'hover');
  await moveInside(page, page.locator('#scroll-main'), 500, 300);
  await expectThumb(page.locator('#scroll-main'), 'y', true);
  await root.evaluate((el) => el.setAttribute('data-platform', 'macos'));
  await expect(root).toHaveAttribute('data-scrollbar-region-mode', 'hover');
  await expectThumb(page.locator('#scroll-main'), 'y', true);
});

test('continuous main, sidebar, tools and chrome transitions paint light and dark thumbs', async ({
  page,
}, testInfo) => {
  await page.evaluate(() => {
    const chrome = document.createElement('div');
    chrome.id = 'chrome-space';
    chrome.style.cssText =
      'position:fixed;top:0;left:0;right:0;height:16px;z-index:10000;background:white';
    document.body.append(chrome);
  });
  const samples: {
    path: string;
    x: number;
    y: number;
    active: boolean;
    theme: string;
    region: string;
  }[] = [];
  for (const theme of ['light', 'dark']) {
    await page.evaluate((theme) => {
      document.documentElement.style.setProperty(
        '--text-primary',
        theme === 'dark' ? '#fff' : '#111',
      );
      document.documentElement.style.setProperty(
        '--bg-base',
        theme === 'dark' ? '#202020' : '#fff',
      );
    }, theme);
    for (let cycle = 0; cycle < 3; cycle++) {
      for (const region of ['main', 'sidebar', 'main', 'tools', 'main', 'chrome']) {
        await moveInside(
          page,
          page.locator(
            region === 'chrome'
              ? '#chrome-space'
              : region === 'tools'
                ? '#tools-scroll'
                : `#scroll-${region}`,
          ),
          region === 'chrome' ? 500 : 40,
          region === 'chrome' ? 8 : region === 'sidebar' ? 400 : 40,
        );
        for (const id of ['main', 'sidebar'])
          await expectThumb(page.locator(`#scroll-${id}`), 'y', id === region);
        await expectThumb(page.locator('#tools-scroll'), 'y', region === 'tools');
        const path = testInfo.outputPath(`paint-${theme}-${cycle}-${samples.length}-${region}.png`);
        await page.screenshot({ path });
        const box = (await page.locator('#scroll-main').boundingBox())!;
        samples.push({
          path,
          x: Math.round(box.x + box.width - 3),
          y: 100,
          active: region === 'main',
          theme,
          region,
        });
        const toolsBox = (await page.locator('#tools-scroll').boundingBox())!;
        samples.push({
          path,
          x: Math.round(toolsBox.x + toolsBox.width - 2),
          y: Math.round(toolsBox.y + 40),
          active: region === 'tools',
          theme,
          region: 'tools-thumb',
        });
      }
    }
  }
  await writeFile(testInfo.outputPath('paint-samples.json'), JSON.stringify(samples, null, 2));
});

test.describe('touch platforms', () => {
  test.use({ hasTouch: true, isMobile: true });
  for (const platform of ['android', 'ios']) {
    test(`${platform} touch never enables desktop hover colors`, async ({ page }) => {
      await page.locator('html').evaluate((el, platform) => {
        el.setAttribute('data-platform', platform);
      }, platform);
      await expect(page.locator('html')).not.toHaveAttribute('data-scrollbar-region-mode');
      await page.locator('#scroll-main').tap({ position: { x: 500, y: 300 } });
      await expect(page.locator('html')).not.toHaveAttribute('data-scrollbar-region-mode');
      expect(
        await page
          .locator('#scroll-main')
          .evaluate((el) => getComputedStyle(el).getPropertyValue('--scrollbar-thumb-y').trim()),
      ).toBe('');
    });
  }
});
