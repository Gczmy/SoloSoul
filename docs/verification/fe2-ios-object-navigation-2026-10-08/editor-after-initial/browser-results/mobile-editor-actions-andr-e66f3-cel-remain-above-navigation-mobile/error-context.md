# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: mobile-editor-actions.spec.ts >> android 390px: editor save and cancel remain above navigation
- Location: e2e/mobile-editor-actions.spec.ts:6:5

# Error details

```
Error: expect(page).toHaveURL(expected) failed

Expected: "http://localhost:1420/workspace/identity"
Received: "http://localhost:1420/workspace?section=identity"
Timeout:  5000ms

Call log:
  - Expect "toHaveURL" with timeout 5000ms
    14 × unexpected value "http://localhost:1420/workspace?section=identity"

```

```yaml
- banner:
  - button "Back"
  - heading "Identity" [level=1]
  - button "Lock Vault"
  - button "More actions"
- navigation "Main navigation":
  - link "Home":
    - /url: /
  - link "Objects":
    - /url: /workspace
  - link "Tools":
    - /url: /tools
  - link "Settings":
    - /url: /settings
- button "New"
- main:
  - button "All"
  - button "Identity" [pressed]
  - button "Travel"
  - button "Financial"
  - button "Professional"
  - button "Documents"
  - textbox "Search objects…"
  - text: 0 objects
  - combobox "Sort objects":
    - option "Recently updated" [selected]
    - option "By name"
  - paragraph: No objects yet
```

# Test source

```ts
  1  | import { expect, test } from '@playwright/test';
  2  | import { login, setupTauriMock } from './fixtures/auth';
  3  | 
  4  | for (const platform of ['ios', 'android'] as const) {
  5  |   for (const width of [320, 390]) {
  6  |     test(`${platform} ${width}px: editor save and cancel remain above navigation`, async ({
  7  |       page,
  8  |     }) => {
  9  |       await page.setViewportSize({ width, height: 640 });
  10 |       await setupTauriMock(page);
  11 |       await page.addInitScript((mockPlatform) => {
  12 |         const prefs = {
  13 |           theme: 'light',
  14 |           language: 'en-US',
  15 |           hasSeenOnboarding: true,
  16 |           autoLockTimeoutMinutes: 0,
  17 |           backupReminderDays: 0,
  18 |         };
  19 |         Object.assign(window, {
  20 |           __MOCK_PLATFORM__: mockPlatform,
  21 |           __E2E_MOCKS__: {
  22 |             ui_get_preferences: () => prefs,
  23 |             user_data_get_preferences: () => prefs,
  24 |             vault_check_directory: () => true,
  25 |             template_list: () => [
  26 |               {
  27 |                 id: 'public-identity',
  28 |                 accountId: 'e2e-account',
  29 |                 name: 'Public identity',
  30 |                 category: 'identity',
  31 |                 createdAt: '2026-10-08T00:00:00Z',
  32 |                 properties: Array.from({ length: 8 }, (_, i) => ({
  33 |                   id: `field-${i}`,
  34 |                   name: `Public field ${i}`,
  35 |                   type: 'text',
  36 |                   sensitivityLevel: 'public',
  37 |                 })),
  38 |               },
  39 |             ],
  40 |             object_field_suggestions: () => [],
  41 |           },
  42 |         });
  43 |         localStorage.setItem('i18nextLng', 'en-US');
  44 |       }, platform);
  45 |       await login(page);
  46 |       await page
  47 |         .getByRole('button', { name: /^Identity/ })
  48 |         .first()
  49 |         .click();
  50 |       if (platform === 'android') {
  51 |         await page.locator('.android-fab').click();
  52 |         await page.getByRole('button', { name: /^New object/ }).click();
  53 |       } else {
  54 |         await page.getByRole('button', { name: '+', exact: true }).click();
  55 |       }
  56 |       await expect(page).toHaveURL('/editor?section=identity');
  57 |       await expect(page.getByLabel('Object Name', { exact: true })).toBeVisible();
  58 |       await page.evaluate(() => {
  59 |         document.documentElement.style.fontSize = '20px';
  60 |       });
  61 |       const cancel = page.getByRole('button', { name: 'Cancel', exact: true });
  62 |       const save = page.getByRole('button', { name: 'Save', exact: true });
  63 |       await cancel.scrollIntoViewIfNeeded();
  64 |       const nav = page.locator(
  65 |         platform === 'ios' ? '[data-testid="mobile-bottom-nav"]' : '.android-navigation',
  66 |       );
  67 |       const navRect = (await nav.boundingBox())!;
  68 |       await page.screenshot({ path: test.info().outputPath('editor-actions.png') });
  69 |       for (const button of [save, cancel]) {
  70 |         const rect = (await button.boundingBox())!;
  71 |         expect(rect.height).toBeGreaterThanOrEqual(44);
  72 |         expect(rect.y).toBeGreaterThanOrEqual(0);
  73 |         expect(rect.y + rect.height).toBeLessThanOrEqual(navRect.y);
  74 |         expect(
  75 |           await button.evaluate((node) => {
  76 |             const r = node.getBoundingClientRect();
  77 |             const hit = document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2);
  78 |             return node === hit || node.contains(hit);
  79 |           }),
  80 |         ).toBe(true);
  81 |       }
  82 |       await cancel.click();
> 83 |       await expect(page).toHaveURL('/workspace/identity');
     |                          ^ Error: expect(page).toHaveURL(expected) failed
  84 |     });
  85 |   }
  86 | }
  87 | 
```