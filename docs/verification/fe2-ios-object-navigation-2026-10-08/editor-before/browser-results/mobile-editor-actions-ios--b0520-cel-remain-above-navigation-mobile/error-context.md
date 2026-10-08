# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: mobile-editor-actions.spec.ts >> ios 390px: editor save and cancel remain above navigation
- Location: e2e/mobile-editor-actions.spec.ts:6:5

# Error details

```
Error: expect(received).toBeLessThanOrEqual(expected)

Expected: <= 584
Received:    628
```

# Page snapshot

```yaml
- generic [ref=e3]:
  - banner [ref=e4]:
    - generic [ref=e5]:
      - button "Back" [ref=e6] [cursor=pointer]:
        - img [ref=e7]
      - heading "New Object" [level=1] [ref=e9]
  - navigation "Home" [ref=e10]:
    - button "Home" [ref=e11] [cursor=pointer]:
      - img [ref=e12]
      - generic [ref=e15]: Home
    - button "Settings" [ref=e16] [cursor=pointer]:
      - img [ref=e17]
      - generic [ref=e20]: Settings
    - button "Add Page" [ref=e23] [cursor=pointer]:
      - img [ref=e24]
      - generic [ref=e25]: Add Page
    - button "Expand" [ref=e26] [cursor=pointer]:
      - img [ref=e27]
      - generic [ref=e29]: Expand
    - button "Lock Vault" [ref=e30] [cursor=pointer]:
      - img [ref=e31]
      - generic [ref=e34]: Lock Vault
  - main [ref=e37]:
    - generic [ref=e38]:
      - generic [ref=e39]:
        - heading "Object Type" [level=3] [ref=e40]
        - paragraph [ref=e41]: "Save to: Identity"
        - generic [ref=e42]:
          - button "Public identity" [pressed] [ref=e43] [cursor=pointer]
          - button "Manage Templates" [ref=e44] [cursor=pointer]:
            - img [ref=e45]
            - text: Manage Templates
      - generic [ref=e50]:
        - generic [ref=e53]: Object Name
        - textbox "Object Name" [ref=e55]:
          - /placeholder: Enter name
      - generic [ref=e56]:
        - heading "Properties" [level=3] [ref=e57]
        - generic [ref=e58]:
          - generic [ref=e60]:
            - generic [ref=e62]:
              - img [ref=e63]
              - generic [ref=e65]: Public field 0
              - 'generic "Sensitivity: Public" [ref=e67]':
                - img [ref=e68]
                - text: Public
            - textbox "Public field 0 Public" [ref=e70]
          - generic [ref=e72]:
            - generic [ref=e74]:
              - img [ref=e75]
              - generic [ref=e77]: Public field 1
              - 'generic "Sensitivity: Public" [ref=e79]':
                - img [ref=e80]
                - text: Public
            - textbox "Public field 1 Public" [ref=e82]
          - generic [ref=e84]:
            - generic [ref=e86]:
              - img [ref=e87]
              - generic [ref=e89]: Public field 2
              - 'generic "Sensitivity: Public" [ref=e91]':
                - img [ref=e92]
                - text: Public
            - textbox "Public field 2 Public" [ref=e94]
          - generic [ref=e96]:
            - generic [ref=e98]:
              - img [ref=e99]
              - generic [ref=e101]: Public field 3
              - 'generic "Sensitivity: Public" [ref=e103]':
                - img [ref=e104]
                - text: Public
            - textbox "Public field 3 Public" [ref=e106]
          - generic [ref=e108]:
            - generic [ref=e110]:
              - img [ref=e111]
              - generic [ref=e113]: Public field 4
              - 'generic "Sensitivity: Public" [ref=e115]':
                - img [ref=e116]
                - text: Public
            - textbox "Public field 4 Public" [ref=e118]
          - generic [ref=e120]:
            - generic [ref=e122]:
              - img [ref=e123]
              - generic [ref=e125]: Public field 5
              - 'generic "Sensitivity: Public" [ref=e127]':
                - img [ref=e128]
                - text: Public
            - textbox "Public field 5 Public" [ref=e130]
          - generic [ref=e132]:
            - generic [ref=e134]:
              - img [ref=e135]
              - generic [ref=e137]: Public field 6
              - 'generic "Sensitivity: Public" [ref=e139]':
                - img [ref=e140]
                - text: Public
            - textbox "Public field 6 Public" [ref=e142]
          - generic [ref=e144]:
            - generic [ref=e146]:
              - img [ref=e147]
              - generic [ref=e149]: Public field 7
              - 'generic "Sensitivity: Public" [ref=e151]':
                - img [ref=e152]
                - text: Public
            - textbox "Public field 7 Public" [ref=e154]
      - generic [ref=e155]:
        - button "Cancel" [ref=e156] [cursor=pointer]
        - button "Save" [ref=e157] [cursor=pointer]
```

# Test source

```ts
  1  | import { expect, test } from '@playwright/test';
  2  | import { login, setupTauriMock } from './fixtures/auth';
  3  | 
  4  | for (const platform of ['ios', 'android'] as const) {
  5  |   for (const width of [320, 390]) {
  6  |     test(`${platform} ${width}px: editor save and cancel remain above navigation`, async ({ page }) => {
  7  |       await page.setViewportSize({ width, height: 640 });
  8  |       await setupTauriMock(page);
  9  |       await page.addInitScript((mockPlatform) => {
  10 |         const prefs = {
  11 |           theme: 'light',
  12 |           language: 'en-US',
  13 |           hasSeenOnboarding: true,
  14 |           autoLockTimeoutMinutes: 0,
  15 |           backupReminderDays: 0,
  16 |         };
  17 |         Object.assign(window, {
  18 |           __MOCK_PLATFORM__: mockPlatform,
  19 |           __E2E_MOCKS__: {
  20 |             ui_get_preferences: () => prefs,
  21 |             user_data_get_preferences: () => prefs,
  22 |             vault_check_directory: () => true,
  23 |             template_list: () => [
  24 |               {
  25 |                 id: 'public-identity',
  26 |                 accountId: 'e2e-account',
  27 |                 name: 'Public identity',
  28 |                 category: 'identity',
  29 |                 createdAt: '2026-10-08T00:00:00Z',
  30 |                 properties: Array.from({ length: 8 }, (_, i) => ({
  31 |                   id: `field-${i}`,
  32 |                   name: `Public field ${i}`,
  33 |                   type: 'text',
  34 |                   sensitivityLevel: 'public',
  35 |                 })),
  36 |               },
  37 |             ],
  38 |             object_field_suggestions: () => [],
  39 |           },
  40 |         });
  41 |         localStorage.setItem('i18nextLng', 'en-US');
  42 |       }, platform);
  43 |       await login(page);
  44 |       await page.getByRole('button', { name: /^Identity/ }).first().click();
  45 |       if (platform === 'android') {
  46 |         await page.locator('.android-fab').click();
  47 |         await page.getByRole('button', { name: /^New object/ }).click();
  48 |       } else {
  49 |         await page.getByRole('button', { name: '+', exact: true }).click();
  50 |       }
  51 |       await expect(page).toHaveURL('/editor?section=identity');
  52 |       await expect(page.getByLabel('Object Name', { exact: true })).toBeVisible();
  53 |       await page.evaluate(() => {
  54 |         document.documentElement.style.fontSize = '20px';
  55 |       });
  56 |       const cancel = page.getByRole('button', { name: 'Cancel', exact: true });
  57 |       const save = page.getByRole('button', { name: 'Save', exact: true });
  58 |       await cancel.scrollIntoViewIfNeeded();
  59 |       const nav = page.locator(
  60 |         platform === 'ios' ? '[data-testid="mobile-bottom-nav"]' : '.android-navigation',
  61 |       );
  62 |       const navRect = (await nav.boundingBox())!;
  63 |       await page.screenshot({ path: test.info().outputPath('editor-actions.png') });
  64 |       for (const button of [save, cancel]) {
  65 |         const rect = (await button.boundingBox())!;
  66 |         expect(rect.height).toBeGreaterThanOrEqual(44);
  67 |         expect(rect.y).toBeGreaterThanOrEqual(0);
> 68 |         expect(rect.y + rect.height).toBeLessThanOrEqual(navRect.y);
     |                                      ^ Error: expect(received).toBeLessThanOrEqual(expected)
  69 |         expect(
  70 |           await button.evaluate((node) => {
  71 |             const r = node.getBoundingClientRect();
  72 |             const hit = document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2);
  73 |             return node === hit || node.contains(hit);
  74 |           }),
  75 |         ).toBe(true);
  76 |       }
  77 |       await cancel.click();
  78 |       await expect(page).toHaveURL('/workspace/identity');
  79 |     });
  80 |   }
  81 | }
  82 | 
```