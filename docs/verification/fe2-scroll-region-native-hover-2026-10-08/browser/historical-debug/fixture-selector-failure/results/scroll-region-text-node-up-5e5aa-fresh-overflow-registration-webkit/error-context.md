# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: scroll-region.spec.ts >> text node updates and viewport resizing refresh overflow registration
- Location: e2e/scroll-region.spec.ts:270:1

# Error details

```
Error: expect(locator).not.toHaveAttribute() failed

Locator:  locator('#scroll-main')
Expected: not have attribute
Received: have attribute
Timeout:  5000ms

Call log:
  - Expect "not toHaveAttribute" with timeout 5000ms
  - waiting for locator('#scroll-main')
    14 × locator resolved to <main id="scroll-main" data-scrollbar-region-y="true">…</main>
       - unexpected value "attribute present"

```

```yaml
- main:
  - button "Main"
  - text: Horizontal content
  - textbox: Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line Long line
  - text: Main content Short text
```

# Test source

```ts
  198 |   await page.locator('#scroll-overlay').evaluate((el) => {
  199 |     el.style.display = 'block';
  200 |   });
  201 |   await expectThumb(main, 'y', false);
  202 |   await page.locator('#scroll-overlay').evaluate((el) => {
  203 |     el.style.display = 'none';
  204 |   });
  205 |   await expectThumb(main, 'y', true);
  206 | });
  207 | 
  208 | for (const boundary of ['role', 'aria-modal', 'data-macos-glass-backdrop']) {
  209 |   test(`nested ${boundary} boundary suppresses hover outside dialog`, async ({ page }) => {
  210 |     const main = page.locator('#scroll-main');
  211 |     await main.evaluate((el, boundary) => {
  212 |       const dialog = document.createElement('div');
  213 |       dialog.id = 'nested-dialog';
  214 |       dialog.setAttribute(
  215 |         boundary,
  216 |         boundary === 'role' ? 'dialog' : boundary === 'aria-modal' ? 'true' : '',
  217 |       );
  218 |       dialog.style.cssText =
  219 |         'position:fixed;left:350px;top:150px;width:220px;height:220px;padding:20px;z-index:10000';
  220 |       const body = document.createElement('div');
  221 |       body.id = 'dialog-scroll';
  222 |       body.style.cssText = 'height:160px;overflow:auto';
  223 |       const content = document.createElement('div');
  224 |       content.style.height = '1000px';
  225 |       content.textContent = 'Dialog content';
  226 |       body.append(content);
  227 |       dialog.append(body);
  228 |       el.append(dialog);
  229 |     }, boundary);
  230 |     const dialog = page.locator('#nested-dialog');
  231 |     const body = page.locator('#dialog-scroll');
  232 |     await expect(body).toHaveAttribute('data-scrollbar-region-y', 'true');
  233 |     await moveInside(page, body, 40, 40);
  234 |     expect(await main.evaluate((el) => el.matches(':hover'))).toBe(true);
  235 |     await expectThumb(body, 'y', true);
  236 |     await expectThumb(main, 'y', false);
  237 |     await moveInside(page, dialog, 5, 5);
  238 |     await expectThumb(body, 'y', false);
  239 |     await expectThumb(main, 'y', false);
  240 |   });
  241 | }
  242 | 
  243 | test('content becoming scrollable updates each axis without moving pointer', async ({ page }) => {
  244 |   const main = page.locator('#scroll-main');
  245 |   const content = main.locator('> div').last();
  246 |   await content.evaluate((el) => {
  247 |     el.style.height = '20px';
  248 |   });
  249 |   await expect(main).not.toHaveAttribute('data-scrollbar-region-y');
  250 |   await moveInside(page, main, 500, 300);
  251 |   await expectThumb(main, 'y', false);
  252 |   await content.evaluate((el) => {
  253 |     el.style.height = '2000px';
  254 |     el.style.width = '1800px';
  255 |   });
  256 |   await expect(main).toHaveAttribute('data-scrollbar-region-x', 'true');
  257 |   await expect(main).toHaveAttribute('data-scrollbar-region-y', 'true');
  258 |   await expectThumb(main, 'x', true);
  259 |   await expectThumb(main, 'y', true);
  260 |   await content.evaluate((el) => {
  261 |     el.style.width = 'auto';
  262 |     el.style.height = '20px';
  263 |   });
  264 |   await expect(main).not.toHaveAttribute('data-scrollbar-region-x');
  265 |   await expect(main).not.toHaveAttribute('data-scrollbar-region-y');
  266 |   await expectThumb(main, 'x', false);
  267 |   await expectThumb(main, 'y', false);
  268 | });
  269 | 
  270 | test('text node updates and viewport resizing refresh overflow registration', async ({ page }) => {
  271 |   await page.locator('#scroll-main').evaluate((el) => {
  272 |     const region = document.createElement('div');
  273 |     region.id = 'dynamic-text-scroll';
  274 |     region.style.cssText =
  275 |       'position:fixed;left:500px;top:300px;width:160px;height:80px;overflow:auto';
  276 |     region.append(document.createTextNode('Short text'));
  277 |     el.append(region);
  278 |   });
  279 |   const region = page.locator('#dynamic-text-scroll');
  280 |   await moveInside(page, region, 40, 40);
  281 |   await expect(region).not.toHaveAttribute('data-scrollbar-region-y');
  282 |   await region.evaluate((el) => {
  283 |     el.firstChild!.textContent = 'Long text with spaces. '.repeat(100);
  284 |   });
  285 |   await expect(region).toHaveAttribute('data-scrollbar-region-y', 'true');
  286 |   await expectThumb(region, 'y', true);
  287 |   await expectThumb(page.locator('#scroll-main'), 'y', false);
  288 |   await region.evaluate((el) => {
  289 |     el.firstChild!.textContent = 'Short text';
  290 |   });
  291 |   await expect(region).not.toHaveAttribute('data-scrollbar-region-y');
  292 |   await page
  293 |     .locator('#scroll-main > div')
  294 |     .last()
  295 |     .evaluate((el) => {
  296 |       el.style.height = '20px';
  297 |     });
> 298 |   await expect(page.locator('#scroll-main')).not.toHaveAttribute('data-scrollbar-region-y');
      |                                                  ^ Error: expect(locator).not.toHaveAttribute() failed
  299 |   await page.setViewportSize({ width: 1000, height: 200 });
  300 |   await expect(page.locator('#scroll-main')).toHaveAttribute('data-scrollbar-region-y', 'true');
  301 | });
  302 | 
  303 | test('textarea input changes yield vertical axis and input/select remain excluded', async ({
  304 |   page,
  305 | }) => {
  306 |   const main = page.locator('#scroll-main');
  307 |   const textarea = page.locator('#text-scroll');
  308 |   await moveInside(page, textarea, 40, 40);
  309 |   await expectThumb(textarea, 'y', true);
  310 |   await expectThumb(main, 'y', false);
  311 |   await textarea.evaluate((el) => {
  312 |     (el as HTMLTextAreaElement).value = 'Short text';
  313 |     el.dispatchEvent(new Event('input', { bubbles: true }));
  314 |   });
  315 |   await expect(textarea).not.toHaveAttribute('data-scrollbar-region-y');
  316 |   await expectThumb(textarea, 'y', false);
  317 |   await expectThumb(main, 'y', true);
  318 |   await textarea.evaluate((el) => {
  319 |     (el as HTMLTextAreaElement).value = 'Long line\n'.repeat(40);
  320 |     el.dispatchEvent(new Event('input', { bubbles: true }));
  321 |   });
  322 |   await expect(textarea).toHaveAttribute('data-scrollbar-region-y', 'true');
  323 |   await expectThumb(textarea, 'y', true);
  324 |   await main.evaluate((el) => {
  325 |     const input = document.createElement('input');
  326 |     input.id = 'overflow-input';
  327 |     input.value = 'Long input text '.repeat(100);
  328 |     input.style.cssText = 'display:block;width:100px;overflow:auto';
  329 |     const select = document.createElement('select');
  330 |     select.id = 'overflow-select';
  331 |     select.multiple = true;
  332 |     select.style.cssText = 'display:block;height:50px;overflow:auto';
  333 |     select.innerHTML = '<option>Option</option>'.repeat(20);
  334 |     el.prepend(input, select);
  335 |   });
  336 |   for (const id of ['overflow-input', 'overflow-select']) {
  337 |     const control = page.locator(`#${id}`);
  338 |     await moveInside(page, control, 10, 10);
  339 |     await expect(control).not.toHaveAttribute('data-scrollbar-region-x');
  340 |     await expect(control).not.toHaveAttribute('data-scrollbar-region-y');
  341 |     await expectThumb(control, 'x', false);
  342 |     await expectThumb(control, 'y', false);
  343 |     await expectThumb(main, 'y', true);
  344 |   }
  345 | });
  346 | 
  347 | test('platform registration works before desktop material initialization', async ({ page }) => {
  348 |   const root = page.locator('html');
  349 |   await root.evaluate((el) => {
  350 |     el.removeAttribute('data-desktop-platform');
  351 |     el.setAttribute('data-platform', 'windows');
  352 |   });
  353 |   await expect(root).not.toHaveAttribute('data-desktop-platform');
  354 |   await expect(root).toHaveAttribute('data-scrollbar-region-mode', 'hover');
  355 |   await moveInside(page, page.locator('#scroll-main'), 500, 300);
  356 |   await expectThumb(page.locator('#scroll-main'), 'y', true);
  357 |   await root.evaluate((el) => el.setAttribute('data-platform', 'macos'));
  358 |   await expect(root).toHaveAttribute('data-scrollbar-region-mode', 'hover');
  359 |   await expectThumb(page.locator('#scroll-main'), 'y', true);
  360 | });
  361 | 
  362 | test('continuous main, sidebar, tools and chrome transitions paint light and dark thumbs', async ({
  363 |   page,
  364 | }, testInfo) => {
  365 |   await page.evaluate(() => {
  366 |     const chrome = document.createElement('div');
  367 |     chrome.id = 'chrome-space';
  368 |     chrome.style.cssText =
  369 |       'position:fixed;top:0;left:0;right:0;height:16px;z-index:10000;background:white';
  370 |     document.body.append(chrome);
  371 |   });
  372 |   const samples: {
  373 |     path: string;
  374 |     x: number;
  375 |     y: number;
  376 |     active: boolean;
  377 |     theme: string;
  378 |     region: string;
  379 |   }[] = [];
  380 |   for (const theme of ['light', 'dark']) {
  381 |     await page.evaluate((theme) => {
  382 |       document.documentElement.style.setProperty(
  383 |         '--text-primary',
  384 |         theme === 'dark' ? '#fff' : '#111',
  385 |       );
  386 |       document.documentElement.style.setProperty(
  387 |         '--bg-base',
  388 |         theme === 'dark' ? '#202020' : '#fff',
  389 |       );
  390 |     }, theme);
  391 |     for (let cycle = 0; cycle < 3; cycle++) {
  392 |       for (const region of ['main', 'sidebar', 'main', 'tools', 'main', 'chrome']) {
  393 |         await moveInside(
  394 |           page,
  395 |           page.locator(
  396 |             region === 'chrome'
  397 |               ? '#chrome-space'
  398 |               : region === 'tools'
```