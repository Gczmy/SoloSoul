# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: sidebar-tools.spec.ts >> right 折叠侧栏打开搜索后能再次点击搜索及直接切换卡片
- Location: e2e/sidebar-tools.spec.ts:341:5

# Error details

```
Error: locator.click: Test ended.
Call log:
  - waiting for locator('#desktop-navigation').getByRole('button', { name: 'Collapse sidebar', exact: true })

```

# Test source

```ts
  247 |         await card
  248 |           .getByRole('textbox', { name: 'Page name', exact: true })
  249 |           .fill('Small window page');
  250 |         await confirm.click();
  251 |         await expect(page).toHaveURL(/\/workspace\/custom\//);
  252 |         await expect(card).toHaveCount(0);
  253 |       }
  254 |     });
  255 |   }
  256 | }
  257 | 
  258 | for (const position of ['top', 'bottom'] as const) {
  259 |   test(`${position} 横向导航添加页面打开后位置固定，缩放后表单和操作保持可见`, async ({ page }) => {
  260 |     await page.setViewportSize({ width: 1000, height: 600 });
  261 |     await setupSidebar(page, position);
  262 |     const tools = page.getByRole('button', { name: 'Tools', exact: true });
  263 |     const trigger = page.getByRole('button', { name: 'Add Page', exact: true });
  264 |     await tools.hover();
  265 |     await expect(tools).toHaveAttribute('aria-expanded', 'true');
  266 |     await trigger.click();
  267 |     const openingTriggerBounds = (await trigger.boundingBox())!;
  268 |     const card = page.locator('[data-add-page-popover]');
  269 |     const icons = card.locator('[data-icon-picker-scroll]');
  270 |     const name = card.getByRole('textbox', { name: 'Page name', exact: true });
  271 |     const confirm = card.getByRole('button', { name: 'Confirm', exact: true });
  272 |     await expect(name).toBeInViewport();
  273 |     await expect(confirm).toBeInViewport();
  274 |     await card.evaluate(async (element) => {
  275 |       await Promise.all(element.getAnimations().map((animation) => animation.finished));
  276 |     });
  277 |     const openingBounds = (await card.boundingBox())!;
  278 | 
  279 |     // 真实悬停离开工具区后，添加按钮随菜单收起向右移动；已打开的卡片不能跟随。
  280 |     // 经过按钮与卡片的间隙，避免 hover 瞬移到 React Portal 后仍被视为组件内部移动。
  281 |     await page.mouse.move(
  282 |       openingTriggerBounds.x + openingTriggerBounds.width / 2,
  283 |       position === 'top'
  284 |         ? openingTriggerBounds.y + openingTriggerBounds.height + 4
  285 |         : openingTriggerBounds.y - 4,
  286 |     );
  287 |     await icons.hover();
  288 |     await expect(tools).toHaveAttribute('aria-expanded', 'false');
  289 |     await tools.evaluate(async (element) => {
  290 |       await Promise.all(
  291 |         element
  292 |           .closest('header')!
  293 |           .getAnimations({ subtree: true })
  294 |           .map((animation) => animation.finished),
  295 |       );
  296 |     });
  297 |     expect((await trigger.boundingBox())!.x - openingTriggerBounds.x).toBeGreaterThan(100);
  298 |     expect(await card.boundingBox()).toEqual(openingBounds);
  299 |     await page.mouse.wheel(0, 300);
  300 |     await expect.poll(() => icons.evaluate((element) => element.scrollTop)).toBeGreaterThan(0);
  301 |     expect(await card.boundingBox()).toEqual(openingBounds);
  302 | 
  303 |     await name.fill('New collection');
  304 |     await card
  305 |       .getByRole('textbox', { name: 'Page description (optional)', exact: true })
  306 |       .fill('Draft');
  307 |     await icons.getByRole('button').last().click();
  308 |     await expect(name).toHaveValue('New collection');
  309 |     expect(await card.boundingBox()).toEqual(openingBounds);
  310 |     await card.getByRole('button', { name: 'Cancel', exact: true }).click();
  311 |     await expect(card).toHaveCount(0);
  312 | 
  313 |     // 关闭重开应取新锚点；底栏卡片沿用居中布局，上栏卡片靠近当前添加按钮。
  314 |     await trigger.click();
  315 |     await card.evaluate(async (element) => {
  316 |       await Promise.all(element.getAnimations().map((animation) => animation.finished));
  317 |     });
  318 |     await expect(tools).toHaveAttribute('aria-expanded', 'false');
  319 |     const reopenedBounds = (await card.boundingBox())!;
  320 |     if (position === 'top') expect(reopenedBounds.x).toBeGreaterThan(openingBounds.x + 100);
  321 |     else expect(reopenedBounds).toEqual(openingBounds);
  322 | 
  323 |     // 固定打开锚点不应阻止窗口缩小时避让视口边缘。
  324 |     await page.setViewportSize({ width: 800, height: 480 });
  325 |     await expect(name).toBeInViewport();
  326 |     await expect(confirm).toBeInViewport();
  327 |     const smallBounds = (await card.boundingBox())!;
  328 |     expect(smallBounds.x).toBeGreaterThanOrEqual(12);
  329 |     expect(smallBounds.x + smallBounds.width).toBeLessThanOrEqual(788);
  330 |     expect(smallBounds.y).toBeGreaterThanOrEqual(56);
  331 |     expect(smallBounds.y + smallBounds.height).toBeLessThanOrEqual(464);
  332 |     await name.fill(`${position} collection`);
  333 |     await confirm.click();
  334 |     await expect(page).toHaveURL(/\/workspace\/custom\//);
  335 |     await expect(card).toHaveCount(0);
  336 |   });
  337 | }
  338 | 
  339 | for (const position of ['left', 'right'] as const) {
  340 |   for (const collapsed of [false, true]) {
  341 |     test(`${position} ${collapsed ? '折叠' : '展开'}侧栏打开搜索后能再次点击搜索及直接切换卡片`, async ({
  342 |       page,
  343 |     }) => {
  344 |       await setupSidebar(page, position);
  345 |       const nav = page.locator('#desktop-navigation');
  346 |       if (collapsed)
> 347 |         await nav.getByRole('button', { name: 'Collapse sidebar', exact: true }).click();
      |                                                                                  ^ Error: locator.click: Test ended.
  348 |       const tools = nav.getByRole('button', { name: 'Tools', exact: true });
  349 |       const search = nav.getByRole('button', { name: 'Search', exact: true });
  350 |       const input = page.getByPlaceholder('Search objects, profiles...');
  351 |       await tools.hover();
  352 |       await search.click();
  353 |       await expect(input).toBeFocused();
  354 | 
  355 |       // 按实际坐标点击，捕获遮罩吞掉侧栏点击的回归，而非等待遮罩消失。
  356 |       const bounds = (await search.boundingBox())!;
  357 |       await page.mouse.click(bounds.x + bounds.width / 2, bounds.y + bounds.height / 2);
  358 |       await expect(input).toHaveCount(0);
  359 |       await expect(tools).toHaveAttribute('aria-expanded', 'true');
  360 | 
  361 |       await search.click();
  362 |       await expect(input).toBeFocused();
  363 |       await nav.getByRole('button', { name: 'Plugins', exact: true }).click({ timeout: 3000 });
  364 |       await expect(input).toHaveCount(0);
  365 |       await expect(page.getByRole('dialog', { name: 'Plugins', exact: true })).toBeVisible();
  366 |       await expect(tools).toHaveAttribute('aria-expanded', 'true');
  367 |       await page.keyboard.press('Escape');
  368 |       await page.mouse.move(640, 650);
  369 |       await expect(tools).toHaveAttribute('aria-expanded', 'false');
  370 |     });
  371 |   }
  372 | }
  373 | 
  374 | test('工具菜单随原生玻璃与辅助功能切换材质', async ({ page }) => {
  375 |   await setupSidebar(page, 'left');
  376 |   const root = page.locator('html');
  377 |   const tools = page.getByRole('button', { name: 'Tools', exact: true });
  378 |   const menu = page.locator('[data-sidebar-tools]');
  379 |   await tools.hover();
  380 |   for (const material of ['liquid-glass', 'vibrancy']) {
  381 |     await root.evaluate(
  382 |       (element, value) => element.setAttribute('data-native-material', value),
  383 |       material,
  384 |     );
  385 |     await expect(menu).toHaveCSS('background-color', 'rgba(0, 0, 0, 0)');
  386 |     await expect(menu).toHaveCSS('backdrop-filter', 'none');
  387 |   }
  388 |   await root.evaluate((element) => element.setAttribute('data-native-material', 'solid'));
  389 |   await expect(menu).not.toHaveCSS('background-color', 'rgba(0, 0, 0, 0)');
  390 |   await root.evaluate((element) => {
  391 |     element.setAttribute('data-native-material', 'liquid-glass');
  392 |     element.setAttribute('data-high-contrast', 'true');
  393 |   });
  394 |   await expect(menu).not.toHaveCSS('background-color', 'rgba(0, 0, 0, 0)');
  395 |   await root.evaluate((element) => element.setAttribute('data-high-contrast', 'false'));
  396 |   await expect(menu).toHaveCSS('background-color', 'rgba(0, 0, 0, 0)');
  397 |   // 浏览器的辅助功能偏好也必须覆盖原生玻璃分支，不能被选择器优先级吞掉。
  398 |   const cdp = await page.context().newCDPSession(page);
  399 |   await cdp.send('Emulation.setEmulatedMedia', {
  400 |     features: [{ name: 'prefers-reduced-transparency', value: 'reduce' }],
  401 |   });
  402 |   await expect(menu).not.toHaveCSS('background-color', 'rgba(0, 0, 0, 0)');
  403 |   await cdp.send('Emulation.setEmulatedMedia', { features: [] });
  404 |   await cdp.detach();
  405 |   await page.emulateMedia({ forcedColors: 'active' });
  406 |   await expect(menu).not.toHaveCSS('background-color', 'rgba(0, 0, 0, 0)');
  407 | });
  408 | 
  409 | test('小窗口工具列表利用剩余高度，卡片打开期间保持菜单且不遮挡固定操作', async ({ page }) => {
  410 |   await page.setViewportSize({ width: 800, height: 600 });
  411 |   await setupSidebar(page, 'left');
  412 |   const nav = page.locator('#desktop-navigation');
  413 |   const tools = nav.getByRole('button', { name: 'Tools', exact: true });
  414 |   const list = nav.locator('[data-sidebar-tools]');
  415 |   await tools.hover();
  416 |   const bounds = (await list.boundingBox())!;
  417 |   expect(bounds.height).toBeGreaterThan(300);
  418 |   expect(bounds.y).toBeGreaterThanOrEqual(100);
  419 |   expect(bounds.y + bounds.height).toBeLessThanOrEqual((await tools.boundingBox())!.y);
  420 |   await list.getByRole('button', { name: 'AI Chat', exact: true }).scrollIntoViewIfNeeded();
  421 |   await list.getByRole('button', { name: 'AI Chat', exact: true }).click();
  422 |   const chat = page.locator('[data-ai-quick-chat="open"]');
  423 |   await expect(chat).toBeVisible();
  424 |   await expect(chat.getByRole('button', { name: 'Close', exact: true })).toBeVisible();
  425 |   await chat.hover();
  426 |   await expect(tools).toHaveAttribute('aria-expanded', 'true');
  427 |   expect((await chat.boundingBox())!.x).toBeGreaterThanOrEqual(232);
  428 |   await chat.getByRole('button', { name: 'Close', exact: true }).click();
  429 |   await expect(chat).toHaveCount(0);
  430 |   await expect(tools).toHaveAttribute('aria-expanded', 'false');
  431 |   await tools.hover();
  432 |   await expect(list.getByRole('button', { name: 'AI Chat', exact: true })).toBeInViewport();
  433 |   await nav.getByRole('button', { name: 'Settings', exact: true }).click();
  434 |   await expect(page).toHaveURL(/\/settings$/);
  435 |   await expect(tools).toHaveAttribute('aria-expanded', 'false');
  436 | });
  437 | 
  438 | test('工具入口支持点击、键盘和 Escape，关闭后菜单退出 Tab 顺序', async ({ page, browserName }) => {
  439 |   // macOS WebKit 默认使用 Option+Tab 访问按钮，与系统键盘导航偏好保持一致。
  440 |   const tabKey = browserName === 'webkit' ? 'Alt+Tab' : 'Tab';
  441 |   await setupSidebar(page, 'left', 'windows');
  442 |   const nav = page.locator('#desktop-navigation');
  443 |   const tools = nav.getByRole('button', { name: 'Tools', exact: true });
  444 |   const list = nav.locator('[data-sidebar-tools]');
  445 |   await tools.click();
  446 |   await page.mouse.move(640, 400);
  447 |   await expect(tools).toHaveAttribute('aria-expanded', 'true');
```