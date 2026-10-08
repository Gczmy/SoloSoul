# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: android-material.spec.ts >> Android reminders occupy layout and follow the active sheet without covering actions
- Location: e2e/android-material.spec.ts:251:1

# Error details

```
Error: locator.click: Error: strict mode violation: getByTestId('object-detail-modal').getByRole('button', { name: 'Back Up Now', exact: true }) resolved to 2 elements:
    1) <button class="_actionBtn_1p7fy_110">Back Up Now</button> aka getByRole('button', { name: 'Back Up Now' }).nth(1)
    2) <button class="_actionBtn_1p7fy_110">Back Up Now</button> aka getByRole('button', { name: 'Back Up Now' }).nth(3)

Call log:
  - waiting for getByTestId('object-detail-modal').getByRole('button', { name: 'Back Up Now', exact: true })

```

# Page snapshot

```yaml
- generic [ref=e3]:
  - banner [ref=e4]:
    - generic [ref=e5]:
      - button "Back" [ref=e6] [cursor=pointer]:
        - img [ref=e7]
      - heading "Objects" [level=1] [ref=e9]
    - generic [ref=e10]:
      - button "Lock Vault" [ref=e11] [cursor=pointer]:
        - img [ref=e12]
      - button "More actions" [ref=e17] [cursor=pointer]:
        - img [ref=e18]
  - navigation "Main navigation" [ref=e22]:
    - link "Home" [ref=e23] [cursor=pointer]:
      - /url: /
      - img [ref=e25]
      - generic [ref=e28]: Home
    - link "Objects" [ref=e29] [cursor=pointer]:
      - /url: /workspace
      - img [ref=e31]
      - generic [ref=e35]: Objects
    - link "Tools" [ref=e36] [cursor=pointer]:
      - /url: /tools
      - img [ref=e38]
      - generic [ref=e42]: Tools
    - link "Settings" [ref=e43] [cursor=pointer]:
      - /url: /settings
      - img [ref=e45]
      - generic [ref=e48]: Settings
  - button "New" [ref=e49] [cursor=pointer]:
    - img [ref=e50]
    - generic [ref=e51]: New
  - main [ref=e54]:
    - generic [ref=e56]:
      - generic "Object categories" [ref=e57]:
        - button "All" [pressed] [ref=e58] [cursor=pointer]:
          - img [ref=e59]
          - text: All
        - button "Identity" [ref=e61] [cursor=pointer]:
          - img [ref=e62]
          - text: Identity
        - button "Travel" [ref=e66] [cursor=pointer]:
          - img [ref=e67]
          - text: Travel
        - button "Financial" [ref=e69] [cursor=pointer]:
          - img [ref=e70]
          - text: Financial
        - button "Professional" [ref=e73] [cursor=pointer]:
          - img [ref=e74]
          - text: Professional
        - button "Documents" [ref=e77] [cursor=pointer]:
          - img [ref=e78]
          - text: Documents
      - generic [ref=e82]:
        - generic:
          - img
        - textbox "Search objects…" [ref=e83]
      - generic [ref=e84]:
        - generic [ref=e85]: 2 objects
        - combobox "Sort objects" [ref=e86]:
          - option "Recently updated" [selected]
          - option "By name"
      - generic [ref=e87]:
        - generic [ref=e88]:
          - 'button "Passport Identity · 未关联模板 Sensitivity: Critical" [active] [ref=e89] [cursor=pointer]':
            - img [ref=e91]
            - generic [ref=e94]:
              - strong [ref=e95]: Passport
              - generic [ref=e96]: Identity · 未关联模板
              - 'generic "Sensitivity: Critical" [ref=e98]':
                - img [ref=e99]
          - button "Actions for Passport" [ref=e102] [cursor=pointer]:
            - img [ref=e103]
        - generic [ref=e107]:
          - 'button "Savings Financial · 未关联模板 Sensitivity: Critical" [ref=e108] [cursor=pointer]':
            - img [ref=e110]
            - generic [ref=e113]:
              - strong [ref=e114]: Savings
              - generic [ref=e115]: Financial · 未关联模板
              - 'generic "Sensitivity: Critical" [ref=e117]':
                - img [ref=e118]
          - button "Actions for Savings" [ref=e121] [cursor=pointer]:
            - img [ref=e122]
      - dialog "Passport" [ref=e127]:
        - generic [ref=e128]:
          - generic [ref=e129]:
            - img [ref=e131]
            - generic [ref=e134]:
              - heading "Passport" [level=2] [ref=e135]
              - generic [ref=e136]: "Identity · created: 2026-09-15 · updated: 2026-09-15"
          - button "Close" [ref=e137] [cursor=pointer]:
            - img [ref=e138]
        - generic [ref=e141]:
          - generic [ref=e142]:
            - generic:
              - button "Backup reminder with a long message for a narrow phone Back Up Now Close" [ref=e143] [cursor=pointer]:
                - generic [ref=e144]: Backup reminder with a long message for a narrow phone
                - generic [ref=e145]:
                  - button "Back Up Now" [ref=e146]
                  - button "Close" [ref=e147]: x
              - button "It has been a while since your last backup. Please go to Settings > Backup & Restore to create one. Back Up Now Close" [ref=e148] [cursor=pointer]:
                - generic [ref=e149]: It has been a while since your last backup. Please go to Settings > Backup & Restore to create one.
                - generic [ref=e150]:
                  - button "Back Up Now" [ref=e151]
                  - button "Close" [ref=e152]: x
          - generic [ref=e155]:
            - generic [ref=e156]:
              - generic [ref=e157]:
                - img [ref=e158]
                - generic [ref=e160]: Secret
                - 'generic "Sensitivity: Critical" [ref=e161]':
                  - img [ref=e162]
                  - text: Critical
              - generic [ref=e165]:
                - button [ref=e166] [cursor=pointer]:
                  - img [ref=e167]
                - button [ref=e170] [cursor=pointer]:
                  - img [ref=e171]
            - generic [ref=e174]: ••••••••
        - generic [ref=e175]:
          - button "Attachments (0)" [ref=e176] [cursor=pointer]:
            - generic [ref=e177]: Attachments
            - generic [ref=e178]: "0"
          - button "Edit" [ref=e179] [cursor=pointer]:
            - generic [ref=e180]: Edit
          - button "Actions for Passport" [ref=e181] [cursor=pointer]:
            - img [ref=e182]
```

# Test source

```ts
  221 |   await expect(page.getByRole('dialog').getByRole('textbox').first()).toHaveValue('Reading room');
  222 |   await page.goBack();
  223 |   await expect(page.getByRole('dialog')).toHaveCount(0);
  224 |   await expect(page).toHaveURL('/');
  225 |   await expect(page.locator('#root')).toHaveJSProperty('inert', false);
  226 |   expect(await page.evaluate(() => (window as any).__pageUpdates.length)).toBe(1);
  227 | });
  228 | 
  229 | test('all objects, filters, search, details and contextual creation reuse business routes', async ({
  230 |   page,
  231 | }) => {
  232 |   await page.locator('.android-navigation a[href="/workspace"]').click();
  233 |   await expect(page.getByTestId('workspace-object-card')).toHaveCount(2);
  234 |   const identity = page.locator('.android-chip').filter({ hasText: 'Identity' });
  235 |   const width = (await identity.boundingBox())!.width;
  236 |   await identity.click();
  237 |   await expect(identity).toHaveAttribute('aria-pressed', 'true');
  238 |   expect((await identity.boundingBox())!.width).toBe(width);
  239 |   await expect(page.getByTestId('workspace-object-card')).toHaveCount(1);
  240 |   await page.locator('.android-fab').click();
  241 |   await page.getByRole('button', { name: /New object/ }).click();
  242 |   await expect(page).toHaveURL('/editor?section=identity');
  243 |   await page.locator('.android-navigation a[href="/workspace"]').click();
  244 |   await page.getByRole('textbox').fill('Savings');
  245 |   await expect(page.getByTestId('workspace-object-card')).toHaveCount(1);
  246 |   await page.getByRole('button', { name: /^Savings/ }).click();
  247 |   await expect(page.getByTestId('object-detail-modal')).toBeVisible();
  248 |   await expect(page.getByText('PRIVATE-TEST-VALUE', { exact: true })).toHaveCount(0);
  249 | });
  250 | 
  251 | test('Android reminders occupy layout and follow the active sheet without covering actions', async ({
  252 |   page,
  253 | }) => {
  254 |   await page.setViewportSize({ width: 320, height: 640 });
  255 |   await page.evaluate(async () => {
  256 |     const source = '/src/stores/uiStore.ts';
  257 |     const { useUiStore } = await import(source);
  258 |     Object.assign(window, { __toastActions: 0 });
  259 |     useUiStore.getState().showToast({
  260 |       message: 'Backup reminder with a long message for a narrow phone',
  261 |       type: 'warning',
  262 |       duration: 60000,
  263 |       action: {
  264 |         label: 'Back Up Now',
  265 |         onClick: () => {
  266 |           (window as any).__toastActions += 1;
  267 |         },
  268 |       },
  269 |     });
  270 |   });
  271 |   const toast = page.locator('[data-toast-container]');
  272 |   await expect(page.locator('[data-shell-notifications] [data-toast-container]')).toBeVisible();
  273 |   const id = await toast.locator('[data-macos-glass="notification"]').getAttribute('class');
  274 |   await page.locator('.android-fab').click();
  275 |   const sheet = page.locator('.android-sheet');
  276 |   await expect(sheet.locator('[data-toast-container]')).toBeVisible();
  277 |   await expect(page.locator('[data-toast-container]')).toHaveCount(1);
  278 |   await expect(page.locator('#root')).toHaveJSProperty('inert', true);
  279 |   const action = sheet.getByRole('button', { name: /New object/ });
  280 |   await expect
  281 |     .poll(() =>
  282 |       sheet.evaluate((node) =>
  283 |         node.getAnimations().every((animation) => animation.playState !== 'running'),
  284 |       ),
  285 |     )
  286 |     .toBe(true);
  287 |   const reminder = (await toast.boundingBox())!;
  288 |   const actionRect = (await action.boundingBox())!;
  289 |   expect(reminder.y + reminder.height).toBeLessThanOrEqual(actionRect.y);
  290 |   await page.screenshot({ path: test.info().outputPath('toast-sheet-320.png') });
  291 |   await page.keyboard.press('Escape');
  292 |   await expect(page.locator('[data-shell-notifications] [data-toast-container]')).toBeVisible();
  293 |   expect(await toast.locator('[data-macos-glass="notification"]').getAttribute('class')).toBe(id);
  294 |   await page.locator('.android-navigation a[href="/workspace"]').click();
  295 |   await page.getByRole('button', { name: 'Actions for Passport', exact: true }).click();
  296 |   await expect(sheet.locator('[data-toast-container]')).toBeVisible();
  297 |   await expect
  298 |     .poll(() =>
  299 |       sheet.evaluate((node) =>
  300 |         node.getAnimations().every((animation) => animation.playState !== 'running'),
  301 |       ),
  302 |     )
  303 |     .toBe(true);
  304 |   const remove = sheet.getByRole('button', { name: 'Delete', exact: true });
  305 |   await remove.scrollIntoViewIfNeeded();
  306 |   const toastBox = (await toast.boundingBox())!;
  307 |   const deleteBox = (await remove.boundingBox())!;
  308 |   expect(toastBox.y + toastBox.height).toBeLessThanOrEqual(deleteBox.y);
  309 |   expect(
  310 |     await remove.evaluate((node) => {
  311 |       const r = node.getBoundingClientRect();
  312 |       const hit = document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2);
  313 |       return node === hit || node.contains(hit);
  314 |     }),
  315 |   ).toBe(true);
  316 |   await page.screenshot({ path: test.info().outputPath('toast-object-menu-320.png') });
  317 |   await page.keyboard.press('Escape');
  318 |   await page.getByRole('button', { name: /^Passport/ }).click();
  319 |   const detail = page.getByTestId('object-detail-modal');
  320 |   await expect(detail.locator('[data-toast-container]')).toBeVisible();
> 321 |   await detail.getByRole('button', { name: 'Back Up Now', exact: true }).click();
      |                                                                          ^ Error: locator.click: Error: strict mode violation: getByTestId('object-detail-modal').getByRole('button', { name: 'Back Up Now', exact: true }) resolved to 2 elements:
  322 |   expect(await page.evaluate(() => (window as any).__toastActions)).toBe(1);
  323 |   await expect(detail).toBeVisible();
  324 |   await expect(toast).toHaveCount(0);
  325 | });
  326 | 
  327 | test('Android editor save stays reachable with a reminder and enlarged text', async ({ page }) => {
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
```