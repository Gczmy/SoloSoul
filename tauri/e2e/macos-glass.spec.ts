import { expect, test, type Locator, type Page } from '@playwright/test';
import { login, setupTauriMock } from './fixtures/auth';

async function setupMac(page: Page, theme: 'light' | 'dark') {
  await setupTauriMock(page);
  await page.addInitScript((theme) => {
    const preferences = {
      theme,
      accentColor: 'ocean',
      language: 'en-US',
      hasSeenOnboarding: true,
      sidebarPosition: 'left',
      autoLockTimeoutMinutes: 0,
    };
    const objects = Array.from({ length: 8 }, (_, index) => ({
      id: `glass-object-${index}`,
      name: index ? `Travel document ${index}` : 'Passport',
      typeId: 'identity',
      accountId: 'e2e-account',
      sensitivityLevel: 'critical',
      createdAt: '2026-09-16T09:00:00Z',
      updatedAt: '2026-09-16T09:00:00Z',
      properties: {
        secret: 'PRIVATE-TEST-VALUE',
        __fields: { secret: { name: 'Secret', type: 'text' } },
      },
      propertyLabels: { secret: 'critical' },
    }));
    Object.assign(window, {
      __MOCK_PLATFORM__: 'macos',
      __E2E_MOCKS__: {
        ui_get_preferences: () => preferences,
        user_data_get_preferences: () => preferences,
        vault_check_directory: () => true,
        ocr_get_model_status: () => ({ installed: true, bundled: true }),
        ocr_list_available_tiers: () => [],
        ocr_get_active_tier: () => 'small',
        sync_list_conflicts: () => [],
        set_titlebar_color: () => ({
          material: 'liquid-glass',
          platform: 'macos',
          reduceMotion: false,
          highContrast: false,
          titlebarHeight: 52,
          trafficLightsRight: 79,
        }),
        get_window_layout: () => ({
          platform: 'macos',
          titlebarHeight: 52,
          trafficLightsRight: 79,
        }),
        object_list: ({ filter }: { filter?: { typeId?: string; parentId?: string } }) =>
          filter?.parentId
            ? []
            : objects.filter((obj) => !filter?.typeId || obj.typeId === filter.typeId),
        object_get: ({ objectId }: { objectId: string }) =>
          objects.find((obj) => obj.id === objectId),
        attachment_count_stats: () => ({ attachmentCount: 0, photoCount: 0 }),
        snapshot_list: () => [],
      },
    });
    localStorage.setItem('i18nextLng', 'en-US');
  }, theme);
  await login(page);
  await expect(page.locator('html')).toHaveAttribute('data-theme', theme);
}

async function expectContentSurface(surface: Locator) {
  await expect(surface).toBeVisible();
  await expect(surface).toHaveCSS('backdrop-filter', 'none');
  await expect(surface).toHaveCSS('background-image', 'none');
}

async function expectFlatControl(control: Locator) {
  await expectContentSurface(control);
  await expect(control).toHaveCSS('box-shadow', 'none');
}

async function expectNavigationGlass(surface: Locator) {
  await expect(surface).toBeVisible();
  await expect(surface).toHaveCSS('backdrop-filter', /blur\(/);
  await expect(surface).toHaveCSS('background-image', 'none');
}

async function expectUnobstructed(surface: Locator) {
  await expect
    .poll(() =>
      surface.evaluate((element) => {
        const r = element.getBoundingClientRect();
        return [6, r.height / 2, r.height - 6].every((y) =>
          element.contains(document.elementFromPoint(r.x + r.width / 2, r.y + y)),
        );
      }),
    )
    .toBe(true);
}

for (const theme of ['light', 'dark'] as const) {
  test(`${theme} AppBar 平面按钮保持原生标题栏尺寸与操作`, async ({ page }) => {
    await setupMac(page, theme);
    const header = page.locator('[data-appbar]');
    const guide = header.getByRole('button', { name: 'Guide', exact: true });
    await expectFlatControl(guide);
    expect((await header.boundingBox())!.height).toBe(52);
    await guide.click();
    const guidePanel = page.locator('[data-page-guide-overlay] [role="dialog"]');
    await expectContentSurface(guidePanel);
    await expectUnobstructed(guidePanel);
    await page.screenshot({ path: test.info().outputPath('guide.png') });
    await page.keyboard.press('Escape');
    await page
      .locator('#desktop-navigation')
      .getByRole('button', { name: 'Identity', exact: true })
      .click();
    const create = header.getByRole('button', { name: '+ New', exact: true });
    await expectFlatControl(create);
    const before = await create.boundingBox();
    // 改变材质能力不能改变顶栏布局或原生按钮命中区。
    await page
      .locator('html')
      .evaluate((root) => root.setAttribute('data-native-material', 'solid'));
    expect(await create.boundingBox()).toEqual(before);
    await expect(create).toHaveCSS('backdrop-filter', 'none');
    await page
      .locator('html')
      .evaluate((root) => root.setAttribute('data-native-material', 'liquid-glass'));
    await page.screenshot({ path: test.info().outputPath('workspace.png') });
    await create.click();
    await expect(page).toHaveURL(/\/editor(?:\?|$)/);
    const back = header.getByRole('button', { name: 'Back', exact: true });
    await expectFlatControl(back);
    await back.click();
    await expect(page).toHaveURL(/\/workspace/);
  });

  test(`${theme} 侧栏浮层与对象二级对话框保持可点击和视口定位`, async ({ page }) => {
    await setupMac(page, theme);
    const nav = page.locator('#desktop-navigation');
    for (const [name, selector] of [
      [
        'Search',
        '[data-macos-glass="panel"]:has(input[placeholder="Search objects, profiles..."])',
      ],
      ['Plugins', '[role="dialog"][aria-label="Plugins"]'],
      ['OCR', '[data-ocr-quick-scan="open"]'],
      ['AI Chat', '[data-ai-quick-chat="open"]'],
    ]) {
      await nav.getByRole('button', { name: 'Tools', exact: true }).hover();
      await nav.getByRole('button', { name, exact: true }).click();
      const card = page.locator(selector);
      await expectContentSurface(card);
      await expectUnobstructed(card);
      if (name === 'Search') {
        await card.getByRole('textbox').click();
        await expect(card.getByRole('textbox')).toBeFocused();
        await page.screenshot({ path: test.info().outputPath('search.png') });
      }
      if (name === 'AI Chat')
        await expect(card.getByRole('button', { name: 'Close', exact: true })).toBeVisible();
      await page.keyboard.press('Escape');
      await expect(card).toHaveCount(0);
    }
    await page.mouse.move(640, 400);
    await expect(nav.getByRole('button', { name: 'Tools', exact: true })).toHaveAttribute(
      'aria-expanded',
      'false',
    );
    await nav.getByRole('button', { name: 'Identity', exact: true }).click();
    await page.getByRole('button', { name: /^Passport/ }).click();
    const detail = page.getByTestId('object-detail-modal');
    await expectContentSurface(detail);
    await expectUnobstructed(detail);
    await expect(page.getByText('PRIVATE-TEST-VALUE', { exact: true })).toHaveCount(0);
    await page.screenshot({ path: test.info().outputPath('object.png') });
    // 指南是详情内部触发的 Portal，不能被父卡片的滤镜截断或改成局部定位。
    await detail.getByRole('button', { name: 'Guide', exact: true }).click();
    const guide = page.locator('[data-page-guide-overlay] [role="dialog"]');
    await expectContentSurface(guide);
    await expectUnobstructed(guide);
    const rect = (await guide.boundingBox())!;
    expect(rect.x + rect.width / 2).toBeCloseTo(640, 0);
    await page.keyboard.press('Escape');
    await expect(detail).toBeVisible();
    await detail.getByRole('button', { name: 'Delete', exact: true }).click();
    const confirm = page
      .getByRole('dialog')
      .filter({ has: page.getByRole('button', { name: 'Cancel', exact: true }) });
    await expectContentSurface(confirm);
    await expectUnobstructed(confirm);
    await confirm.getByRole('button', { name: 'Cancel', exact: true }).click();
    await expect(confirm).toHaveCount(0);
    await expect(page.getByRole('button', { name: /^Passport/ })).toBeVisible();
  });
}

test('小窗口菜单与辅助功能回退保持键盘操作及清晰表面', async ({ page }) => {
  await setupMac(page, 'light');
  await page.setViewportSize({ width: 800, height: 600 });
  const more = page
    .locator('[data-appbar]')
    .getByRole('button', { name: 'More actions', exact: true });
  await expectFlatControl(more);
  await more.click();
  const menu = page.locator('#toolbar-secondary-actions');
  await expectNavigationGlass(menu);
  await expectUnobstructed(menu);
  const guide = menu.getByRole('button', { name: 'Guide', exact: true });
  await expect(guide).toBeFocused();
  // 菜单内部操作保留原有菜单样式，避免每项叠加滤镜。
  await expect(guide).toHaveCSS('backdrop-filter', 'none');
  await page.keyboard.press('Escape');
  await expect(more).toBeFocused();
  await page.locator('html').evaluate((root) => root.setAttribute('data-high-contrast', 'true'));
  await more.click();
  await expect(menu).toHaveCSS('backdrop-filter', 'none');
  await expectUnobstructed(menu);
  await page.locator('html').evaluate((root) => root.setAttribute('data-high-contrast', 'false'));
  await expectNavigationGlass(menu);
  await page.emulateMedia({ forcedColors: 'active' });
  await expect(menu).toHaveCSS('backdrop-filter', 'none');
});
