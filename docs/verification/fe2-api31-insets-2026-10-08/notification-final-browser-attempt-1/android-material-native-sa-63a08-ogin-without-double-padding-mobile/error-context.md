# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: android-material.spec.ts >> native safe-area lengths protect chrome, sheets and login without double padding
- Location: e2e/android-material.spec.ts:389:1

# Error details

```
Error: expect(received).toBeGreaterThanOrEqual(expected)

Expected: >= 20
Received:    0
```

# Page snapshot

```yaml
- generic [ref=e3]:
  - banner [ref=e4]:
    - generic [ref=e5]:
      - img "SoloSoul" [ref=e6]
      - heading "SoloSoul" [level=1] [ref=e7]
    - generic [ref=e8]:
      - button "Lock Vault" [ref=e9] [cursor=pointer]:
        - img [ref=e10]
      - button "Account management" [ref=e14] [cursor=pointer]:
        - img [ref=e15]
      - group "More actions" [ref=e19]:
        - button "Guide" [ref=e20] [cursor=pointer]:
          - img [ref=e21]
  - navigation "Main navigation" [ref=e24]:
    - link "Home" [ref=e25] [cursor=pointer]:
      - /url: /
      - img [ref=e27]
      - generic [ref=e30]: Home
    - link "Objects" [ref=e31] [cursor=pointer]:
      - /url: /workspace
      - img [ref=e33]
      - generic [ref=e37]: Objects
    - link "Tools" [ref=e38] [cursor=pointer]:
      - /url: /tools
      - img [ref=e40]
      - generic [ref=e44]: Tools
    - link "Settings" [ref=e45] [cursor=pointer]:
      - /url: /settings
      - img [ref=e47]
      - generic [ref=e50]: Settings
  - button "New" [ref=e51] [cursor=pointer]:
    - img [ref=e52]
    - generic [ref=e53]: New
  - generic [ref=e55]:
    - button "Backup reminder must avoid landscape system navigation Back Up Now Close" [ref=e58] [cursor=pointer]:
      - generic [ref=e59]: Backup reminder must avoid landscape system navigation
      - generic [ref=e60]:
        - button "Back Up Now" [ref=e61]
        - button "Close" [ref=e62]: x
    - main [ref=e63]:
      - generic [ref=e64]:
        - generic [ref=e65]:
          - heading "Welcome back, E2E User" [level=2] [ref=e66]
          - paragraph [ref=e67]: A place for every part of your life.
        - generic [ref=e69]:
          - paragraph [ref=e70]: My vault
          - generic [ref=e71]:
            - strong [ref=e72]: "2"
            - generic [ref=e73]: objects
          - paragraph [ref=e74]: Local vault · Your data, your control
        - generic [ref=e75]:
          - generic [ref=e76]:
            - generic [ref=e77]:
              - heading "Data Sections" [level=2] [ref=e78]
              - button "View all" [ref=e79] [cursor=pointer]:
                - text: View all
                - img [ref=e80]
            - generic [ref=e82]:
              - button "Identity 1 object" [ref=e84] [cursor=pointer]:
                - img [ref=e85]
                - strong [ref=e89]: Identity
                - generic [ref=e90]: 1 object
              - button "Travel 0 objects" [ref=e92] [cursor=pointer]:
                - img [ref=e93]
                - strong [ref=e95]: Travel
                - generic [ref=e96]: 0 objects
              - button "Financial 1 object" [ref=e98] [cursor=pointer]:
                - img [ref=e99]
                - strong [ref=e102]: Financial
                - generic [ref=e103]: 1 object
              - button "Professional 0 objects" [ref=e105] [cursor=pointer]:
                - img [ref=e106]
                - strong [ref=e109]: Professional
                - generic [ref=e110]: 0 objects
              - button "Documents 0 objects" [ref=e112] [cursor=pointer]:
                - img [ref=e113]
                - strong [ref=e116]: Documents
                - generic [ref=e117]: 0 objects
          - generic [ref=e118]:
            - generic [ref=e119]:
              - heading "Recently updated" [level=2] [ref=e120]
              - button "Photo Album" [ref=e121] [cursor=pointer]:
                - img [ref=e122]
            - generic [ref=e127]:
              - button "Passport Identity · Sep 15" [ref=e128] [cursor=pointer]:
                - img [ref=e130]
                - generic [ref=e134]:
                  - strong [ref=e135]: Passport
                  - generic [ref=e136]: Identity · Sep 15
                - img [ref=e137]
              - button "Savings Financial · Sep 14" [ref=e139] [cursor=pointer]:
                - img [ref=e141]
                - generic [ref=e145]:
                  - strong [ref=e146]: Savings
                  - generic [ref=e147]: Financial · Sep 14
                - img [ref=e148]
```

# Test source

```ts
  328 |   await page.setViewportSize({ width: 320, height: 640 });
  329 |   await page.locator('.android-navigation a[href="/workspace"]').click();
  330 |   await page.getByRole('button', { name: /^Passport/ }).click();
  331 |   await page
  332 |     .getByTestId('object-detail-modal')
  333 |     .getByRole('button', { name: 'Edit', exact: true })
  334 |     .click();
  335 |   await expect(page).toHaveURL(/\/editor/);
  336 |   await page.evaluate(async () => {
  337 |     document.documentElement.style.fontSize = '20px';
  338 |     const source = '/src/stores/uiStore.ts';
  339 |     const { useUiStore } = await import(source);
  340 |     useUiStore
  341 |       .getState()
  342 |       .showToast({ message: 'Backup reminder', type: 'warning', duration: 60000 });
  343 |   });
  344 |   const save = page.getByRole('button', { name: 'Save', exact: true });
  345 |   await save.scrollIntoViewIfNeeded();
  346 |   const toastRect = (await page.locator('[data-toast-container]').boundingBox())!;
  347 |   const saveRect = (await save.boundingBox())!;
  348 |   expect(toastRect.y + toastRect.height).toBeLessThanOrEqual(saveRect.y);
  349 |   expect(saveRect.y + saveRect.height).toBeLessThanOrEqual(640);
  350 |   expect(
  351 |     await save.evaluate((node) => {
  352 |       const r = node.getBoundingClientRect();
  353 |       const hit = document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2);
  354 |       return node === hit || node.contains(hit);
  355 |     }),
  356 |   ).toBe(true);
  357 |   expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  358 |   await page.screenshot({ path: test.info().outputPath('toast-editor-320-large-text.png') });
  359 | });
  360 | 
  361 | test('appearance persists across reload and a locked vault drops cached names', async ({
  362 |   page,
  363 | }) => {
  364 |   await page.locator('.android-navigation a[href="/settings"]').click();
  365 |   await page.getByText('Theme & Appearance', { exact: true }).click();
  366 |   await page.getByRole('button', { name: 'Dark', exact: true }).click();
  367 |   await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
  368 |   await page.getByRole('button', { name: 'Forest', exact: true }).click();
  369 |   await page.getByRole('checkbox').check();
  370 |   await expect(page.locator('html')).toHaveAttribute('data-user-reduce-motion', 'true');
  371 |   await page.reload();
  372 |   await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
  373 |   // 重载重新进入锁定页，解锁后读取已保存的账户偏好。
  374 |   await login(page);
  375 |   await expect(page.locator('html')).toHaveAttribute('data-accent', 'forest');
  376 |   await expect(page.locator('html')).toHaveAttribute('data-user-reduce-motion', 'true');
  377 |   await page.getByRole('button', { name: /Lock Vault/i }).click();
  378 |   await expect(page.getByTestId('android-home')).toHaveCount(0);
  379 |   await expect(page.getByText('Passport', { exact: true })).toHaveCount(0);
  380 |   await expect(page.locator('.android-sheet')).toHaveCount(0);
  381 | });
  382 | 
  383 | async function selectGlass(page: import('@playwright/test').Page, label: string) {
  384 |   await page.locator('.android-navigation a[href="/settings"]').click();
  385 |   await page.getByText('Theme & Appearance', { exact: true }).click();
  386 |   await page.getByRole('button', { name: label, exact: true }).click();
  387 | }
  388 | 
  389 | test('native safe-area lengths protect chrome, sheets and login without double padding', async ({
  390 |   page,
  391 | }) => {
  392 |   // 浏览器只验证消费原生长度的 CSS；实际 Android 栏、IME 和重载由独立原生场景验证。
  393 |   await expect(page.getByTestId('android-home')).toBeVisible();
  394 |   await page.evaluate(async () => {
  395 |     const source = '/src/stores/uiStore.ts';
  396 |     const { useUiStore } = await import(source);
  397 |     useUiStore.getState().showToast({
  398 |       message: 'Backup reminder must avoid landscape system navigation',
  399 |       type: 'warning',
  400 |       duration: 60000,
  401 |       action: { label: 'Back Up Now', onClick: () => {} },
  402 |     });
  403 |   });
  404 |   await expect(page.locator('[data-shell-notifications] [data-toast-container]')).toBeVisible();
  405 |   for (const width of [320, 390, 844]) {
  406 |     const height = width === 844 ? 390 : 844;
  407 |     await page.setViewportSize({ width, height });
  408 |     await page.evaluate(() => {
  409 |       const root = document.documentElement;
  410 |       for (const [edge, value] of Object.entries({ top: 24, right: 32, bottom: 28, left: 20 }))
  411 |         root.style.setProperty(`--android-native-safe-area-${edge}`, `${value}px`);
  412 |     });
  413 |     const controls = page.locator(
  414 |       '.android-appbar-leading, .android-appbar-actions button, .android-navigation a, [data-shell-notifications] button',
  415 |     );
  416 |     const boxes = await controls.evaluateAll((nodes) =>
  417 |       nodes
  418 |         .filter(
  419 |           (node) => !node.closest('[inert]') && getComputedStyle(node).visibility !== 'hidden',
  420 |         )
  421 |         .map((node) => {
  422 |           const r = node.getBoundingClientRect();
  423 |           return { left: r.left, right: r.right, top: r.top, bottom: r.bottom };
  424 |         }),
  425 |     );
  426 |     expect(boxes.length).toBeGreaterThanOrEqual(8);
  427 |     for (const r of boxes) {
> 428 |       expect(r.left).toBeGreaterThanOrEqual(20);
      |                      ^ Error: expect(received).toBeGreaterThanOrEqual(expected)
  429 |       expect(r.right).toBeLessThanOrEqual(width - 32);
  430 |       expect(r.top).toBeGreaterThanOrEqual(24);
  431 |       expect(r.bottom).toBeLessThanOrEqual(height - 28);
  432 |     }
  433 |     const appbar = (await page.locator('.android-appbar').boundingBox())!;
  434 |     expect(appbar.height).toBe(88);
  435 |     const main = (await page.locator('[data-shell-main]').boundingBox())!;
  436 |     const reminder = (await page.locator('[data-toast-container]').boundingBox())!;
  437 |     expect(reminder.x).toBeCloseTo(main.x + 16 + (width >= 768 ? 0 : 20), 1);
  438 |     await page.screenshot({ path: test.info().outputPath(`safe-home-${width}.png`) });
  439 |   }
  440 |   await page.setViewportSize({ width: 390, height: 844 });
  441 |   await page.locator('.android-fab').click();
  442 |   const sheet = page.locator('.android-sheet');
  443 |   await expect(sheet).toBeVisible();
  444 |   await expect
  445 |     .poll(() => sheet.evaluate((e) => e.getAnimations().every((a) => a.playState !== 'running')))
  446 |     .toBe(true);
  447 |   const box = (await sheet.boundingBox())!;
  448 |   expect(box.x).toBeGreaterThanOrEqual(20);
  449 |   expect(box.x + box.width).toBeLessThanOrEqual(358);
  450 |   expect(box.y).toBeGreaterThanOrEqual(24);
  451 |   expect(box.y + box.height).toBeLessThanOrEqual(816);
  452 |   await page.screenshot({ path: test.info().outputPath('safe-sheet.png') });
  453 |   await page.keyboard.press('Escape');
  454 |   await page.getByRole('button', { name: /Lock Vault/i }).click();
  455 |   await expect(page.locator('[data-login-card]')).toBeVisible();
  456 |   const layout = page.locator('[data-auth-layout]');
  457 |   const loginLayout = (await layout.boundingBox())!;
  458 |   const loginReminder = (await page.locator('[data-toast-container]').boundingBox())!;
  459 |   // 认证壳已避让安全区，通知只保留自己的 16px 留白。
  460 |   expect(loginReminder.x).toBeCloseTo(loginLayout.x + 20 + 16, 1);
  461 |   expect(
  462 |     await layout.evaluate((node) => {
  463 |       const s = getComputedStyle(node);
  464 |       return [s.paddingTop, s.paddingRight, s.paddingBottom, s.paddingLeft];
  465 |     }),
  466 |   ).toEqual(['24px', '32px', '28px', '20px']);
  467 |   await page.screenshot({ path: test.info().outputPath('safe-login.png') });
  468 |   await page.locator('[data-login-method-region="password"] input').focus();
  469 |   await page.setViewportSize({ width: 390, height: 400 });
  470 |   await expect.poll(() => layout.evaluate((e) => e.getBoundingClientRect().height)).toBe(400);
  471 |   await page.locator('[data-login-method-region="password"] input').focus();
  472 |   await page.evaluate(
  473 |     () =>
  474 |       new Promise<void>((resolve) =>
  475 |         requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
  476 |       ),
  477 |   );
  478 |   for (const selector of [
  479 |     '[data-login-method-region="password"] input',
  480 |     '[data-login-password-submit]',
  481 |   ]) {
  482 |     const control = page.locator(selector);
  483 |     const r = (await control.boundingBox())!;
  484 |     expect(r.y).toBeGreaterThanOrEqual(24);
  485 |     expect(r.y + r.height).toBeLessThanOrEqual(372);
  486 |     expect(
  487 |       await control.evaluate((e) => {
  488 |         const box = e.getBoundingClientRect();
  489 |         const hit = document.elementFromPoint(box.x + box.width / 2, box.y + box.height / 2);
  490 |         return e === hit || e.contains(hit);
  491 |       }),
  492 |     ).toBe(true);
  493 |   }
  494 |   await page.screenshot({ path: test.info().outputPath('safe-login-short.png') });
  495 |   // OS 已经排除系统栏时，原生交付 0；不残留上一种布局的留白。
  496 |   await page.evaluate(() => {
  497 |     for (const edge of ['top', 'right', 'bottom', 'left'])
  498 |       document.documentElement.style.setProperty(`--android-native-safe-area-${edge}`, '0px');
  499 |   });
  500 |   expect(await layout.evaluate((node) => getComputedStyle(node).padding)).toBe('0px');
  501 | });
  502 | 
  503 | for (const [mode, scheme, accent, glass] of [
  504 |   ['Dark', 'forest-night', '#112233', 'Enhanced glass'],
  505 |   ['Light', 'clean-slate', '#ffee00', 'Local glass'],
  506 | ]) {
  507 |   test(`custom ${accent} keeps home actions and tools readable in ${mode}`, async ({ page }) => {
  508 |     await selectGlass(page, glass);
  509 |     await page.getByRole('button', { name: mode, exact: true }).click();
  510 |     await page.getByRole('combobox', { name: mode, exact: true }).selectOption(scheme);
  511 |     await page
  512 |       .getByRole('textbox', { name: 'Custom accent color (hex)', exact: true })
  513 |       .fill(accent);
  514 |     await page
  515 |       .locator('.android-custom-accent')
  516 |       .getByRole('button', { name: 'Save', exact: true })
  517 |       .click();
  518 |     await expect
  519 |       .poll(() =>
  520 |         page
  521 |           .locator('html')
  522 |           .evaluate((root) => getComputedStyle(root).getPropertyValue('--accent-primary').trim()),
  523 |       )
  524 |       .toBe(accent);
  525 |     for (const [path, selector, background] of [
  526 |       ['/', '.android-section-heading .android-text-button', '--bg-base'],
  527 |       ['/tools', '.android-tool > svg', '--bg-elevated'],
  528 |     ]) {
```