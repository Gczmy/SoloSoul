# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: scroll-region.spec.ts >> nearest same-axis child owns highlight and non-overflowing child yields it
- Location: e2e/scroll-region.spec.ts:132:1

# Error details

```
Error: expect(received).toBe(expected) // Object.is equality

Expected: "rgba(128, 128, 128, 0.45)"
Received: "rgb(17, 17, 17)"

Call Log:
- Timeout 5000ms exceeded while waiting on the predicate
```

# Page snapshot

```yaml
- generic [ref=e1]:
  - complementary [ref=e2]:
    - button "Sidebar" [ref=e3]
    - generic [ref=e6]: Tools content
    - generic [ref=e7]: Side content
  - main [ref=e8]:
    - button "Main" [ref=e9]
    - generic [ref=e11]: Horizontal content
    - textbox [ref=e12]: Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line
    - generic [ref=e13]: Main content
```

# Test source

```ts
  1   | import { expect, test, type Locator, type Page } from '@playwright/test';
  2   | import { writeFile } from 'node:fs/promises';
  3   | 
  4   | const INACTIVE_THUMB = 'rgb(128 128 128 / 45%)';
  5   | 
  6   | /** 使用可见窗口坐标，只移动鼠标；locator.hover() 可能先滚动目标。 */
  7   | async function moveInside(page: Page, locator: Locator, x = 40, y = 40) {
  8   |   const box = (await locator.boundingBox())!;
  9   |   await page.mouse.move(box.x + x, box.y + y);
  10  | }
  11  | 
  12  | async function expectThumb(locator: Locator, axis: 'x' | 'y', active: boolean) {
  13  |   const expected = active
  14  |     ? await locator.evaluate((el) => getComputedStyle(el).getPropertyValue('--text-primary').trim())
  15  |     : INACTIVE_THUMB;
  16  |   await expect
  17  |     .poll(() =>
  18  |       locator.evaluate(
  19  |         (el, axis) => getComputedStyle(el).getPropertyValue(`--scrollbar-thumb-${axis}`).trim(),
  20  |         axis,
  21  |       ),
  22  |     )
  23  |     .toBe(expected);
  24  |   // WebKit 不返回原生滚动条伪元素的计算样式，另保存实际绘制截图核验。
  25  |   if (axis === 'y' && test.info().project.name === 'chromium') {
  26  |     await expect
  27  |       .poll(() =>
  28  |         locator.evaluate((el) => getComputedStyle(el, '::-webkit-scrollbar-thumb').backgroundColor),
  29  |       )
> 30  |       .toBe(
      |        ^ Error: expect(received).toBe(expected) // Object.is equality
  31  |         active
  32  |           ? expected === '#fff'
  33  |             ? 'rgb(255, 255, 255)'
  34  |             : 'rgb(17, 17, 17)'
  35  |           : 'rgba(128, 128, 128, 0.45)',
  36  |       );
  37  |   }
  38  | }
  39  | 
  40  | test.use({ viewport: { width: 1000, height: 700 }, hasTouch: false, isMobile: false });
  41  | 
  42  | test.beforeEach(async ({ page }) => {
  43  |   await page.goto('/e2e/fixtures/scrollRegion.html');
  44  |   await expect(page.locator('html')).toHaveAttribute('data-scroll-fixture-ready', 'true');
  45  |   await expect(page.locator('html')).toHaveAttribute('data-scrollbar-region-mode', 'hover');
  46  | });
  47  | 
  48  | test('pure mouse movement switches regions even when all JS move events are blocked', async ({
  49  |   page,
  50  | }) => {
  51  |   // 在应用加载前阻断 window capture，document capture 也接收不到这些事件。
  52  |   await page.addInitScript(() => {
  53  |     const events = { moves: 0, clicks: 0, wheels: 0 };
  54  |     Object.assign(window, { __SCROLL_INPUT_EVENTS__: events });
  55  |     for (const type of [
  56  |       'pointermove',
  57  |       'pointerover',
  58  |       'pointerout',
  59  |       'mousemove',
  60  |       'mouseover',
  61  |       'mouseout',
  62  |     ])
  63  |       window.addEventListener(
  64  |         type,
  65  |         (event) => {
  66  |           events.moves++;
  67  |           event.stopImmediatePropagation();
  68  |         },
  69  |         true,
  70  |       );
  71  |     window.addEventListener('click', () => events.clicks++, true);
  72  |     window.addEventListener('wheel', () => events.wheels++, true);
  73  |   });
  74  |   await page.reload();
  75  |   await expect(page.locator('html')).toHaveAttribute('data-scrollbar-region-mode', 'hover');
  76  |   await page.evaluate(() => {
  77  |     const chrome = document.createElement('div');
  78  |     chrome.id = 'chrome-space';
  79  |     chrome.style.cssText = 'position:fixed;inset:0 0 auto;height:16px;z-index:10000';
  80  |     document.body.append(chrome);
  81  |   });
  82  |   for (let cycle = 0; cycle < 3; cycle++) {
  83  |     for (const region of ['main', 'sidebar', 'tools', 'chrome']) {
  84  |       await moveInside(
  85  |         page,
  86  |         page.locator(
  87  |           region === 'chrome'
  88  |             ? '#chrome-space'
  89  |             : region === 'tools'
  90  |               ? '#tools-scroll'
  91  |               : `#scroll-${region}`,
  92  |         ),
  93  |         region === 'main' || region === 'chrome' ? 500 : 40,
  94  |         region === 'sidebar' ? 400 : region === 'chrome' ? 8 : 40,
  95  |       );
  96  |       for (const id of ['main', 'sidebar', 'tools'])
  97  |         await expectThumb(
  98  |           page.locator(id === 'tools' ? '#tools-scroll' : `#scroll-${id}`),
  99  |           'y',
  100 |           id === region,
  101 |         );
  102 |     }
  103 |   }
  104 |   const events = await page.evaluate(
  105 |     () =>
  106 |       (
  107 |         window as unknown as {
  108 |           __SCROLL_INPUT_EVENTS__: { moves: number; clicks: number; wheels: number };
  109 |         }
  110 |       ).__SCROLL_INPUT_EVENTS__,
  111 |   );
  112 |   expect(events.moves).toBeGreaterThan(0);
  113 |   expect(events.clicks).toBe(0);
  114 |   expect(events.wheels).toBe(0);
  115 |   await expect(page.locator('[data-scrollbar-active]')).toHaveCount(0);
  116 | });
  117 | 
  118 | test('horizontal child keeps only the effective vertical parent highlighted', async ({ page }) => {
  119 |   const main = page.locator('#scroll-main');
  120 |   const horizontal = page.locator('#horizontal');
  121 |   await expect(horizontal).toHaveAttribute('data-scrollbar-region-x', 'true');
  122 |   await expect(horizontal).not.toHaveAttribute('data-scrollbar-region-y');
  123 |   await moveInside(page, horizontal, 40, 25);
  124 |   await expectThumb(horizontal, 'x', true);
  125 |   await expectThumb(horizontal, 'y', false);
  126 |   await expectThumb(main, 'y', true);
  127 |   await moveInside(page, page.locator('#scroll-sidebar'), 40, 400);
  128 |   await expectThumb(main, 'y', false);
  129 |   await expectThumb(horizontal, 'x', false);
  130 | });
```