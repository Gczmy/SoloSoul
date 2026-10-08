# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: sidebar-tools.spec.ts >> 添加页面在折叠侧栏复用导航的提示卡片，展开后不重复显示提示
- Location: e2e/sidebar-tools.spec.ts:176:1

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
    - group "More actions" [ref=e13]:
      - button "Guide" [ref=e14] [cursor=pointer]:
        - img [ref=e15]
        - generic [ref=e18]: Guide
  - navigation "Home" [ref=e19]:
    - generic [ref=e20]:
      - img "SoloSoul" [ref=e21]
      - generic [ref=e22]: SoloSoul
    - generic [ref=e23]:
      - button "Home" [ref=e25] [cursor=pointer]:
        - img [ref=e26]
        - generic [ref=e29]: Home
      - generic [ref=e30]:
        - button "Identity" [ref=e32] [cursor=pointer]:
          - img [ref=e33]
          - generic [ref=e37]: Identity
        - button "Travel" [ref=e39] [cursor=pointer]:
          - img [ref=e40]
          - generic [ref=e42]: Travel
        - button "Financial" [ref=e44] [cursor=pointer]:
          - img [ref=e45]
          - generic [ref=e48]: Financial
        - button "Professional" [ref=e50] [cursor=pointer]:
          - img [ref=e51]
          - generic [ref=e54]: Professional
        - button "Documents" [ref=e56] [cursor=pointer]:
          - img [ref=e57]
          - generic [ref=e60]: Documents
      - button "Add Page" [ref=e63] [cursor=pointer]:
        - img [ref=e64]
        - generic [ref=e65]: Add Page
    - button "Tools" [ref=e67] [cursor=pointer]:
      - img [ref=e68]
      - generic [ref=e70]: Tools
    - generic [ref=e71]:
      - button "Lock Vault" [ref=e73] [cursor=pointer]:
        - img [ref=e74]
        - generic [ref=e77]: Lock Vault
      - button "Settings" [ref=e79] [cursor=pointer]:
        - img [ref=e80]
        - generic [ref=e83]: Settings
  - main [ref=e86]:
    - generic [ref=e87]:
      - generic [ref=e88]:
        - heading "Welcome back, E2E User" [level=2] [ref=e89]
        - paragraph [ref=e90]: Your personal data vault. All data encrypted and stored locally.
      - heading "Data Sections" [level=2] [ref=e91]
      - generic [ref=e92]:
        - button "Identity Personal info, ID cards, contacts" [ref=e93] [cursor=pointer]:
          - generic [ref=e94]:
            - img [ref=e95]
            - heading "Identity" [level=3] [ref=e99]
          - paragraph [ref=e100]: Personal info, ID cards, contacts
        - button "Travel Passports, visas, travel history" [ref=e101] [cursor=pointer]:
          - generic [ref=e102]:
            - img [ref=e103]
            - heading "Travel" [level=3] [ref=e105]
          - paragraph [ref=e106]: Passports, visas, travel history
        - button "Financial Bank accounts, cards, tax info" [ref=e107] [cursor=pointer]:
          - generic [ref=e108]:
            - img [ref=e109]
            - heading "Financial" [level=3] [ref=e112]
          - paragraph [ref=e113]: Bank accounts, cards, tax info
        - button "Professional Education, employment, skills" [ref=e114] [cursor=pointer]:
          - generic [ref=e115]:
            - img [ref=e116]
            - heading "Professional" [level=3] [ref=e119]
          - paragraph [ref=e120]: Education, employment, skills
        - button "Documents Documents imported from OCR scans" [ref=e121] [cursor=pointer]:
          - generic [ref=e122]:
            - img [ref=e123]
            - heading "Documents" [level=3] [ref=e126]
          - paragraph [ref=e127]: Documents imported from OCR scans
      - heading "Quick Access" [level=2] [ref=e128]
      - generic [ref=e129]:
        - button "Settings Manage account, theme, security, and preferences" [ref=e130] [cursor=pointer]:
          - generic [ref=e131]:
            - img [ref=e132]
            - heading "Settings" [level=3] [ref=e135]
          - paragraph [ref=e136]: Manage account, theme, security, and preferences
        - button "Trash View and manage deleted objects, restore or permanently purge" [ref=e137] [cursor=pointer]:
          - generic [ref=e138]:
            - img [ref=e139]
            - heading "Trash" [level=3] [ref=e142]
          - paragraph [ref=e143]: View and manage deleted objects, restore or permanently purge
        - button "Search Globally search objects, profiles, and help docs" [ref=e144] [cursor=pointer]:
          - generic [ref=e145]:
            - img [ref=e146]
            - heading "Search" [level=3] [ref=e149]
          - paragraph [ref=e150]: Globally search objects, profiles, and help docs
        - button "Templates Create, edit, and manage object templates" [ref=e151] [cursor=pointer]:
          - generic [ref=e152]:
            - img [ref=e153]
            - heading "Templates" [level=3] [ref=e157]
          - paragraph [ref=e158]: Create, edit, and manage object templates
        - button "Attachments Manage attachment uploads, downloads, and trash" [ref=e159] [cursor=pointer]:
          - generic [ref=e160]:
            - img [ref=e161]
            - heading "Attachments" [level=3] [ref=e163]
          - paragraph [ref=e164]: Manage attachment uploads, downloads, and trash
        - button "Photo Album Browse photos across all objects" [ref=e165] [cursor=pointer]:
          - generic [ref=e166]:
            - img [ref=e167]
            - heading "Photo Album" [level=3] [ref=e172]
          - paragraph [ref=e173]: Browse photos across all objects
        - button "Plugins Manage local plugin marketplace" [ref=e174] [cursor=pointer]:
          - generic [ref=e175]:
            - img [ref=e176]
            - heading "Plugins" [level=3] [ref=e178]
          - paragraph [ref=e179]: Manage local plugin marketplace
        - button "OCR Scan images for text, quickly extract and create objects" [ref=e180] [cursor=pointer]:
          - generic [ref=e181]:
            - img [ref=e182]
            - heading "OCR" [level=3] [ref=e187]
          - paragraph [ref=e188]: Scan images for text, quickly extract and create objects
        - button "Import / Export Export or import encrypted vault packages" [ref=e189] [cursor=pointer]:
          - generic [ref=e190]:
            - img [ref=e191]
            - heading "Import / Export" [level=3] [ref=e194]
          - paragraph [ref=e195]: Export or import encrypted vault packages
        - button "Device Sync Sync vault data with nearby devices over the local network" [ref=e196] [cursor=pointer]:
          - generic [ref=e197]:
            - img [ref=e198]
            - heading "Device Sync" [level=3] [ref=e203]
          - paragraph [ref=e204]: Sync vault data with nearby devices over the local network
        - button "Help Quick guide to features, security and usage" [ref=e205] [cursor=pointer]:
          - generic [ref=e206]:
            - img [ref=e207]
            - heading "Help" [level=3] [ref=e209]
          - paragraph [ref=e210]: Quick guide to features, security and usage
        - button "AI Chat Chat with the AI assistant for suggestions and help" [ref=e211] [cursor=pointer]:
          - generic [ref=e212]:
            - img [ref=e213]
            - heading "AI Chat" [level=3] [ref=e215]
          - paragraph [ref=e216]: Chat with the AI assistant for suggestions and help
```

# Test source

```ts
  81  |     const addPageBounds = await addPage.boundingBox();
  82  |     const identityBounds = await nav
  83  |       .getByRole('button', { name: 'Identity', exact: true })
  84  |       .boundingBox();
  85  |     await tools.hover();
  86  |     await expect(tools).toHaveAttribute('aria-expanded', 'true');
  87  |     await expect(list).toBeVisible();
  88  |     await expect(list).toHaveCSS('background-color', 'rgba(0, 0, 0, 0)');
  89  |     await expect(list).toHaveCSS('background-image', 'none');
  90  |     await expect(list).toHaveCSS('backdrop-filter', 'none');
  91  |     expect(await tools.boundingBox()).toEqual(toggleBounds);
  92  |     expect(await addPage.boundingBox()).toEqual(addPageBounds);
  93  |     expect(await nav.getByRole('button', { name: 'Identity', exact: true }).boundingBox()).toEqual(
  94  |       identityBounds,
  95  |     );
  96  |     const buttons = list.getByRole('button');
  97  |     await expect(buttons).toHaveCount(10);
  98  |     for (const button of await buttons.all()) {
  99  |       await expect(button).toBeInViewport();
  100 |       // aria展开和可见性先于裁剪动画完成；等待真实中心点可命中，不跳过交互检查。
  101 |       await expect
  102 |         .poll(() =>
  103 |           button.evaluate((element) => {
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
> 181 |   await nav.getByRole('button', { name: 'Collapse sidebar', exact: true }).click();
      |                                                                            ^ Error: locator.click: Test timeout of 30000ms exceeded.
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
  204 |         await nav.getByRole('button', { name: 'Collapse sidebar', exact: true }).click();
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
```