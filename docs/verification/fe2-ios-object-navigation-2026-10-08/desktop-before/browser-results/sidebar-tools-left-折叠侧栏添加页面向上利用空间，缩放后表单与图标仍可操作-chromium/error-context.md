# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: sidebar-tools.spec.ts >> left 折叠侧栏添加页面向上利用空间，缩放后表单与图标仍可操作
- Location: e2e/sidebar-tools.spec.ts:197:5

# Error details

```
Test timeout of 30000ms exceeded.
```

```
Error: locator.click: Test timeout of 30000ms exceeded.
Call log:
  - waiting for locator('#desktop-navigation').getByRole('button', { name: 'Collapse sidebar', exact: true })

```

# Page snapshot

```yaml
- generic [ref=e3]:
  - banner [ref=e4]:
    - button "Collapse sidebar" [expanded] [ref=e5] [cursor=pointer]:
      - img [ref=e6]
    - heading "Home" [level=1] [ref=e10]
    - button "More actions" [ref=e13] [cursor=pointer]:
      - img [ref=e14]
  - navigation "Home" [ref=e18]:
    - generic [ref=e19]:
      - img "SoloSoul" [ref=e20]
      - generic [ref=e21]: SoloSoul
    - generic [ref=e22]:
      - button "Home" [ref=e24] [cursor=pointer]:
        - img [ref=e25]
        - generic [ref=e28]: Home
      - generic [ref=e29]:
        - button "Identity" [ref=e31] [cursor=pointer]:
          - img [ref=e32]
          - generic [ref=e36]: Identity
        - button "Travel" [ref=e38] [cursor=pointer]:
          - img [ref=e39]
          - generic [ref=e41]: Travel
        - button "Financial" [ref=e43] [cursor=pointer]:
          - img [ref=e44]
          - generic [ref=e47]: Financial
        - button "Professional" [ref=e49] [cursor=pointer]:
          - img [ref=e50]
          - generic [ref=e53]: Professional
        - button "Documents" [ref=e55] [cursor=pointer]:
          - img [ref=e56]
          - generic [ref=e59]: Documents
      - button "Add Page" [ref=e62] [cursor=pointer]:
        - img [ref=e63]
        - generic [ref=e64]: Add Page
    - button "Tools" [ref=e66] [cursor=pointer]:
      - img [ref=e67]
      - generic [ref=e69]: Tools
    - generic [ref=e70]:
      - button "Lock Vault" [ref=e72] [cursor=pointer]:
        - img [ref=e73]
        - generic [ref=e76]: Lock Vault
      - button "Settings" [ref=e78] [cursor=pointer]:
        - img [ref=e79]
        - generic [ref=e82]: Settings
  - main [ref=e85]:
    - generic [ref=e86]:
      - generic [ref=e87]:
        - heading "Welcome back, E2E User" [level=2] [ref=e88]
        - paragraph [ref=e89]: Your personal data vault. All data encrypted and stored locally.
      - heading "Data Sections" [level=2] [ref=e90]
      - generic [ref=e91]:
        - button "Identity Personal info, ID cards, contacts" [ref=e92] [cursor=pointer]:
          - generic [ref=e93]:
            - img [ref=e94]
            - heading "Identity" [level=3] [ref=e98]
          - paragraph [ref=e99]: Personal info, ID cards, contacts
        - button "Travel Passports, visas, travel history" [ref=e100] [cursor=pointer]:
          - generic [ref=e101]:
            - img [ref=e102]
            - heading "Travel" [level=3] [ref=e104]
          - paragraph [ref=e105]: Passports, visas, travel history
        - button "Financial Bank accounts, cards, tax info" [ref=e106] [cursor=pointer]:
          - generic [ref=e107]:
            - img [ref=e108]
            - heading "Financial" [level=3] [ref=e111]
          - paragraph [ref=e112]: Bank accounts, cards, tax info
        - button "Professional Education, employment, skills" [ref=e113] [cursor=pointer]:
          - generic [ref=e114]:
            - img [ref=e115]
            - heading "Professional" [level=3] [ref=e118]
          - paragraph [ref=e119]: Education, employment, skills
        - button "Documents Documents imported from OCR scans" [ref=e120] [cursor=pointer]:
          - generic [ref=e121]:
            - img [ref=e122]
            - heading "Documents" [level=3] [ref=e125]
          - paragraph [ref=e126]: Documents imported from OCR scans
      - heading "Quick Access" [level=2] [ref=e127]
      - generic [ref=e128]:
        - button "Settings Manage account, theme, security, and preferences" [ref=e129] [cursor=pointer]:
          - generic [ref=e130]:
            - img [ref=e131]
            - heading "Settings" [level=3] [ref=e134]
          - paragraph [ref=e135]: Manage account, theme, security, and preferences
        - button "Trash View and manage deleted objects, restore or permanently purge" [ref=e136] [cursor=pointer]:
          - generic [ref=e137]:
            - img [ref=e138]
            - heading "Trash" [level=3] [ref=e141]
          - paragraph [ref=e142]: View and manage deleted objects, restore or permanently purge
        - button "Search Globally search objects, profiles, and help docs" [ref=e143] [cursor=pointer]:
          - generic [ref=e144]:
            - img [ref=e145]
            - heading "Search" [level=3] [ref=e148]
          - paragraph [ref=e149]: Globally search objects, profiles, and help docs
        - button "Templates Create, edit, and manage object templates" [ref=e150] [cursor=pointer]:
          - generic [ref=e151]:
            - img [ref=e152]
            - heading "Templates" [level=3] [ref=e156]
          - paragraph [ref=e157]: Create, edit, and manage object templates
        - button "Attachments Manage attachment uploads, downloads, and trash" [ref=e158] [cursor=pointer]:
          - generic [ref=e159]:
            - img [ref=e160]
            - heading "Attachments" [level=3] [ref=e162]
          - paragraph [ref=e163]: Manage attachment uploads, downloads, and trash
        - button "Photo Album Browse photos across all objects" [ref=e164] [cursor=pointer]:
          - generic [ref=e165]:
            - img [ref=e166]
            - heading "Photo Album" [level=3] [ref=e171]
          - paragraph [ref=e172]: Browse photos across all objects
        - button "Plugins Manage local plugin marketplace" [ref=e173] [cursor=pointer]:
          - generic [ref=e174]:
            - img [ref=e175]
            - heading "Plugins" [level=3] [ref=e177]
          - paragraph [ref=e178]: Manage local plugin marketplace
        - button "OCR Scan images for text, quickly extract and create objects" [ref=e179] [cursor=pointer]:
          - generic [ref=e180]:
            - img [ref=e181]
            - heading "OCR" [level=3] [ref=e186]
          - paragraph [ref=e187]: Scan images for text, quickly extract and create objects
        - button "Import / Export Export or import encrypted vault packages" [ref=e188] [cursor=pointer]:
          - generic [ref=e189]:
            - img [ref=e190]
            - heading "Import / Export" [level=3] [ref=e193]
          - paragraph [ref=e194]: Export or import encrypted vault packages
        - button "Device Sync Sync vault data with nearby devices over the local network" [ref=e195] [cursor=pointer]:
          - generic [ref=e196]:
            - img [ref=e197]
            - heading "Device Sync" [level=3] [ref=e202]
          - paragraph [ref=e203]: Sync vault data with nearby devices over the local network
        - button "Help Quick guide to features, security and usage" [ref=e204] [cursor=pointer]:
          - generic [ref=e205]:
            - img [ref=e206]
            - heading "Help" [level=3] [ref=e208]
          - paragraph [ref=e209]: Quick guide to features, security and usage
        - button "AI Chat Chat with the AI assistant for suggestions and help" [ref=e210] [cursor=pointer]:
          - generic [ref=e211]:
            - img [ref=e212]
            - heading "AI Chat" [level=3] [ref=e214]
          - paragraph [ref=e215]: Chat with the AI assistant for suggestions and help
```

# Test source

```ts
  104 |             const r = element.getBoundingClientRect();
  105 |             return element.contains(
  106 |               document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2),
  107 |             );
  108 |           }),
  109 |         )
  110 |         .toBe(true);
  111 |     }
  112 |     // 原生玻璃下不能透出重叠的导航文字；裁剪只影响绘制，不改变布局。
  113 |     expect(
  114 |       await identity.evaluate((element) => {
  115 |         const zone = element.closest('[class*="primaryZone"]')!;
  116 |         const clip = getComputedStyle(zone).clipPath;
  117 |         return clip !== 'none' && !clip.endsWith('0px 0px)');
  118 |       }),
  119 |     ).toBe(true);
  120 |     await list.getByRole('button', { name: 'Search', exact: true }).hover();
  121 |     await expect(list).toBeVisible();
  122 |     await page.screenshot({ path: test.info().outputPath(`tools-${position}.png`) });
  123 |     await page.mouse.move(640, 400);
  124 |     await expect(tools).toHaveAttribute('aria-expanded', 'false');
  125 |     await expect(list).toBeHidden();
  126 |     await identity.click();
  127 |     await expect(page).toHaveURL(/section=identity/);
  128 |   });
  129 | }
  130 | 
  131 | for (const position of ['left', 'right'] as const) {
  132 |   test(`${position} 折叠侧栏使用向上玻璃菜单，缩放和操作卡片不挤动导航`, async ({ page }) => {
  133 |     await setupSidebar(page, position);
  134 |     const nav = page.locator('#desktop-navigation');
  135 |     await nav.getByRole('button', { name: 'Collapse sidebar', exact: true }).click();
  136 |     const tools = nav.getByRole('button', { name: 'Tools', exact: true });
  137 |     const addPage = nav.getByRole('button', { name: 'Add Page', exact: true });
  138 |     const menu = nav.locator('[data-sidebar-tools]');
  139 |     const toolBounds = (await tools.boundingBox())!;
  140 |     const addPageBounds = await addPage.boundingBox();
  141 |     await tools.hover();
  142 |     await expect(menu).toBeVisible();
  143 |     await expect(menu).toHaveCSS('background-color', 'rgba(0, 0, 0, 0)');
  144 |     await expect(menu).toHaveCSS('backdrop-filter', 'none');
  145 |     expect(await tools.boundingBox()).toEqual(toolBounds);
  146 |     expect(await addPage.boundingBox()).toEqual(addPageBounds);
  147 |     const menuBounds = (await menu.boundingBox())!;
  148 |     const navBounds = (await nav.boundingBox())!;
  149 |     expect(menuBounds.height).toBeGreaterThan(400);
  150 |     expect(menuBounds.y + menuBounds.height).toBeLessThanOrEqual(toolBounds.y);
  151 |     expect(menuBounds.x).toBeGreaterThan(navBounds.x);
  152 |     expect(menuBounds.x + menuBounds.width).toBeLessThan(navBounds.x + navBounds.width);
  153 |     await page.screenshot({ path: test.info().outputPath(`compact-tools-${position}.png`) });
  154 | 
  155 |     // 保持菜单打开并缩小窗口，确认仍能滚动到最后一项并打开卡片。
  156 |     await page.setViewportSize({ width: 800, height: 600 });
  157 |     await tools.hover();
  158 |     await expect(menu).toBeVisible();
  159 |     expect((await menu.boundingBox())!.y).toBeGreaterThanOrEqual(100);
  160 |     await menu.getByRole('button', { name: 'AI Chat', exact: true }).click();
  161 |     const chat = page.locator('[data-ai-quick-chat="open"]');
  162 |     await expect(chat).toBeInViewport();
  163 |     const chatBounds = (await chat.boundingBox())!;
  164 |     if (position === 'left') expect(chatBounds.x).toBeGreaterThanOrEqual(96);
  165 |     else expect(chatBounds.x + chatBounds.width).toBeLessThanOrEqual(800 - 96);
  166 |     await chat.hover();
  167 |     await expect(tools).toHaveAttribute('aria-expanded', 'true');
  168 |     await chat.getByRole('button', { name: 'Close', exact: true }).click();
  169 |     await expect(tools).toHaveAttribute('aria-expanded', 'false');
  170 |     await expect(menu).toBeHidden();
  171 |     await nav.getByRole('button', { name: 'Identity', exact: true }).click();
  172 |     await expect(page).toHaveURL(/section=identity/);
  173 |   });
  174 | }
  175 | 
  176 | test('添加页面在折叠侧栏复用导航的提示卡片，展开后不重复显示提示', async ({ page }) => {
  177 |   await setupSidebar(page, 'left');
  178 |   const nav = page.locator('#desktop-navigation');
  179 |   const addPage = nav.getByRole('button', { name: 'Add Page', exact: true });
  180 |   const identity = nav.getByRole('button', { name: 'Identity', exact: true });
  181 |   await nav.getByRole('button', { name: 'Collapse sidebar', exact: true }).click();
  182 |   expect(await buttonAppearance(addPage)).toEqual(await buttonAppearance(identity));
  183 |   await identity.hover();
  184 |   const tooltip = page.locator('[role="tooltip"]');
  185 |   await expect(tooltip).toHaveText('Identity');
  186 |   const tooltipClass = await tooltip.getAttribute('class');
  187 |   await addPage.hover();
  188 |   await expect(tooltip).toHaveText('Add Page');
  189 |   await expect(tooltip).toHaveAttribute('class', tooltipClass!);
  190 |   await nav.getByRole('button', { name: 'Expand sidebar', exact: true }).click();
  191 |   await addPage.hover();
  192 |   await expect(tooltip).toHaveCount(0);
  193 | });
  194 | 
  195 | for (const position of ['left', 'right'] as const) {
  196 |   for (const collapsed of [false, true]) {
  197 |     test(`${position} ${collapsed ? '折叠' : '展开'}侧栏添加页面向上利用空间，缩放后表单与图标仍可操作`, async ({
  198 |       page,
  199 |     }) => {
  200 |       await page.setViewportSize({ width: 800, height: 600 });
  201 |       await setupSidebar(page, position);
  202 |       const nav = page.locator('#desktop-navigation');
  203 |       if (collapsed)
> 204 |         await nav.getByRole('button', { name: 'Collapse sidebar', exact: true }).click();
      |                                                                                  ^ Error: locator.click: Test timeout of 30000ms exceeded.
  205 |       const trigger = nav.getByRole('button', { name: 'Add Page', exact: true });
  206 |       const triggerBounds = (await trigger.boundingBox())!;
  207 |       await trigger.click();
  208 |       const card = page.locator('[data-add-page-popover]');
  209 |       const icons = card.locator('[data-icon-picker-scroll]');
  210 |       await expect(card).toBeVisible();
  211 |       await card.evaluate(async (element) => {
  212 |         await Promise.all(element.getAnimations().map((animation) => animation.finished));
  213 |       });
  214 |       const bounds = (await card.boundingBox())!;
  215 |       expect(bounds.y).toBeLessThan(triggerBounds.y - 100);
  216 |       expect(bounds.height).toBeGreaterThanOrEqual(450);
  217 |       expect(bounds.y).toBeGreaterThanOrEqual(64);
  218 |       expect(bounds.y + bounds.height).toBeLessThanOrEqual(584);
  219 |       expect((await icons.boundingBox())!.height).toBeGreaterThan(280);
  220 |       const confirm = card.getByRole('button', { name: 'Confirm', exact: true });
  221 |       await confirm.click();
  222 |       await expect(card.getByText('Page name is required', { exact: true })).toBeVisible();
  223 |       await expect(confirm).toBeInViewport();
  224 |       await page.screenshot({
  225 |         path: test.info().outputPath(`add-page-${position}-${collapsed}.png`),
  226 |       });
  227 | 
  228 |       await page.setViewportSize({ width: 800, height: 480 });
  229 |       await expect(card).toHaveCSS('height', '400px');
  230 |       const smallBounds = (await card.boundingBox())!;
  231 |       expect(smallBounds.y).toBeGreaterThanOrEqual(64);
  232 |       expect(smallBounds.y + smallBounds.height).toBeLessThanOrEqual(464);
  233 |       expect((await icons.boundingBox())!.height).toBeGreaterThan(180);
  234 |       const name = card.getByRole('textbox', { name: 'Page name', exact: true });
  235 |       await name.fill('New collection');
  236 |       await card
  237 |         .getByRole('textbox', { name: 'Page description (optional)', exact: true })
  238 |         .fill('Draft');
  239 |       const footerBounds = await confirm.boundingBox();
  240 |       await icons.getByRole('button').last().click();
  241 |       await expect(name).toHaveValue('New collection');
  242 |       expect(await confirm.boundingBox()).toEqual(footerBounds);
  243 |       await card.getByRole('button', { name: 'Cancel', exact: true }).click();
  244 |       await expect(card).toHaveCount(0);
  245 |       if (position === 'left' && collapsed) {
  246 |         await trigger.click();
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
```