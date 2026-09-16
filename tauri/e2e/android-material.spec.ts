import { test, expect } from '@playwright/test';
import { setupTauriMock, login } from './fixtures/auth';

test.beforeEach(async ({ page }) => {
  await setupTauriMock(page);
  await page.addInitScript(() => {
    const defaults = {
      theme: 'light',
      accentColor: 'ocean',
      reduceMotion: false,
      language: 'en-US',
      hasSeenOnboarding: true,
      autoLockTimeoutMinutes: 0,
    };
    const prefs = () => ({
      ...defaults,
      ...JSON.parse(localStorage.getItem('android-test-prefs') || '{}'),
    });
    const objects = [
      {
        id: 'object-a',
        name: 'Passport',
        typeId: 'identity',
        updatedAt: '2026-09-15T09:00:00Z',
        sensitivityLevel: 'critical',
      },
      {
        id: 'object-b',
        name: 'Savings',
        typeId: 'financial',
        updatedAt: '2026-09-14T09:00:00Z',
        sensitivityLevel: 'sensitive',
      },
    ].map((obj) => ({
      ...obj,
      accountId: 'e2e-account',
      createdAt: obj.updatedAt,
      properties: {
        secret: 'PRIVATE-TEST-VALUE',
        __fields: { secret: { name: 'Secret', type: 'text' } },
      },
      propertyLabels: { secret: 'critical' },
    }));
    Object.assign(window, {
      __MOCK_PLATFORM__: 'android',
      __E2E_MOCKS__: {
        ui_get_preferences: prefs,
        user_data_get_preferences: prefs,
        user_data_update_preference: ({ payload }: { payload: { preferences: object } }) => {
          localStorage.setItem(
            'android-test-prefs',
            JSON.stringify({ ...prefs(), ...payload.preferences }),
          );
        },
        object_list: ({ filter }: { filter?: { typeId?: string; parentId?: string } }) =>
          filter?.typeId
            ? objects.filter((obj) => obj.typeId === filter.typeId)
            : filter?.parentId
              ? []
              : objects,
        object_get: ({ objectId }: { objectId: string }) =>
          objects.find((obj) => obj.id === objectId),
        object_create: ({ input }: { input: { id: string; name: string; typeId: string } }) =>
          input,
        vault_check_directory: () => true,
        ocr_get_model_status: () => ({ installed: true, bundled: true }),
        sync_list_conflicts: () => [],
        object_trash_list: () => [],
        attachment_count_stats: () => ({ attachmentCount: 0, photoCount: 0 }),
      },
    });
    localStorage.setItem('i18nextLng', 'en-US');
  });
  await login(page);
});

test('Android navigation stays mounted and icons keep their geometry', async ({ page }) => {
  await expect(page.locator('html')).toHaveAttribute('data-platform', 'android');
  const icons = await page.locator('.android-nav-icon svg').elementHandles();
  const boxes = await Promise.all(icons.map((icon) => icon.boundingBox()));
  for (const path of ['/workspace', '/tools', '/settings', '/']) {
    await page.locator(`.android-navigation a[href="${path}"]`).click();
    await expect(page.locator(`.android-navigation a[href="${path}"]`)).toHaveAttribute(
      'aria-current',
      'page',
    );
    for (const [index, icon] of icons.entries()) {
      expect(await icon.evaluate((node) => node.isConnected)).toBe(true);
      expect(await icon.boundingBox()).toEqual(boxes[index]);
    }
  }
  await expect(page.getByTestId('android-home')).toBeVisible();
});

test('responsive surfaces and navigation rail have no horizontal overflow', async ({ page }) => {
  for (const width of [320, 360, 390, 430, 768, 1024]) {
    await page.setViewportSize({ width, height: 844 });
    await expect(page.locator('.android-navigation')).toBeVisible();
    const dimensions = await page.evaluate(() => ({
      root: document.documentElement.scrollWidth,
      viewport: innerWidth,
      content: document.querySelector('main')!.scrollWidth,
      available: document.querySelector('main')!.clientWidth,
    }));
    expect(dimensions.root).toBeLessThanOrEqual(dimensions.viewport);
    expect(dimensions.content).toBeLessThanOrEqual(dimensions.available);
    const nav = await page.locator('.android-navigation').boundingBox();
    expect(width >= 768 ? nav!.width : nav!.height).toBe(width >= 768 ? 88 : 80);
  }
  await page.setViewportSize({ width: 390, height: 844 });
  await page.screenshot({ path: test.info().outputPath('android-home.png') });
});

test('page sheets keep focus, stay within the viewport and preserve create/edit routes', async ({
  page,
}) => {
  await page.locator('.android-fab').click();
  await expect(page.getByRole('dialog')).toBeVisible();
  await expect(page.locator('#root')).toHaveJSProperty('inert', true);
  await page.keyboard.press('Shift+Tab');
  await expect(page.getByRole('button', { name: 'Close', exact: true })).toBeFocused();
  await page.keyboard.press('Escape');
  await expect(page.getByRole('dialog')).toHaveCount(0);
  await expect(page.locator('.android-fab')).toBeFocused();
  await page.locator('.android-fab').click();
  await page.goBack();
  await expect(page.getByRole('dialog')).toHaveCount(0);
  await expect(page).toHaveURL('/');
  await page.locator('.android-fab').click();
  await page.getByRole('button', { name: /Add Page|New Page/i }).click();
  const input = page.getByRole('textbox').first();
  await expect(input).toBeFocused();
  await input.fill('Reading');
  await page.getByRole('button', { name: 'notebook', exact: true }).click();
  await expect(page.getByRole('dialog')).toBeVisible();
  await page.locator('.android-page-form button[type="submit"]').click();
  await expect(page).toHaveURL(/\/workspace\/custom\//);
  await expect(page.getByRole('dialog')).toHaveCount(0);
  await expect(page.locator('header h1')).toHaveText('Reading');

  await page.locator('.android-navigation a[href="/"]').click();
  await page.evaluate(() => {
    Object.assign(window, { __pageUpdates: [] });
    (window as any).__E2E_MOCKS__.object_update = (args: unknown) => {
      (window as any).__pageUpdates.push(args);
    };
  });
  await page.getByRole('button', { name: 'Edit page: Reading', exact: true }).click();
  const editor = page.getByRole('dialog', { name: 'Edit page: Reading', exact: true });
  await expect(editor).toBeVisible();
  await expect(page.locator('#root')).toHaveJSProperty('inert', true);
  // 右列卡片的菜单不能沿锚点溢出；旋转屏幕后整个编辑器仍在可视区域内。
  for (const viewport of [
    { width: 320, height: 640 },
    { width: 844, height: 390 },
    { width: 390, height: 844 },
  ]) {
    await page.setViewportSize(viewport);
    await expect
      .poll(() =>
        editor.evaluate((panel) => {
          const rect = panel.getBoundingClientRect();
          return (
            rect.left >= 12 &&
            rect.right <= innerWidth - 12 &&
            rect.top >= 24 &&
            rect.bottom <= innerHeight - 6 &&
            panel.scrollWidth <= panel.clientWidth
          );
        }),
      )
      .toBe(true);
    await editor.getByRole('button', { name: 'Save', exact: true }).scrollIntoViewIfNeeded();
    await expect(editor.getByRole('button', { name: 'Save', exact: true })).toBeInViewport();
  }
  await page.screenshot({ path: test.info().outputPath('edit-page-portrait.png') });

  // 模拟软键盘压缩 visualViewport，走实际键盘避让监听而非直接改面板坐标。
  await editor.getByRole('textbox').first().fill('Reading room');
  await editor.getByRole('textbox').nth(1).fill('Books and notes');
  await page.evaluate(() => {
    Object.defineProperty(window.visualViewport, 'height', {
      configurable: true,
      get: () => innerHeight - 300,
    });
    window.visualViewport!.dispatchEvent(new Event('resize'));
  });
  await expect
    .poll(() =>
      editor.evaluate((panel) => {
        const rect = panel.getBoundingClientRect();
        return rect.top >= 24 && rect.bottom <= window.visualViewport!.height - 6;
      }),
    )
    .toBe(true);
  await editor.getByRole('button', { name: 'star', exact: true }).click();
  await editor.getByRole('button', { name: 'Save', exact: true }).scrollIntoViewIfNeeded();
  await page.screenshot({ path: test.info().outputPath('edit-page-keyboard.png') });
  await editor.getByRole('button', { name: 'Save', exact: true }).click();
  await expect(editor).toHaveCount(0);
  await expect(page).toHaveURL('/');
  expect(await page.evaluate(() => (window as any).__pageUpdates)).toEqual([
    expect.objectContaining({
      input: {
        name: 'Reading room',
        properties: { description: 'Books and notes' },
        iconName: 'star',
      },
    }),
  ]);
  await page.evaluate(() => {
    Reflect.deleteProperty(window.visualViewport!, 'height');
    window.visualViewport!.dispatchEvent(new Event('resize'));
  });
  const editAgain = page.getByRole('button', { name: 'Edit page: Reading room', exact: true });
  await editAgain.click();
  await page.getByRole('dialog').getByRole('textbox').first().fill('Discarded draft');
  await page.getByRole('button', { name: 'Cancel', exact: true }).click();
  await expect(page.getByRole('dialog')).toHaveCount(0);
  await editAgain.click();
  await expect(page.getByRole('dialog').getByRole('textbox').first()).toHaveValue('Reading room');
  await page.goBack();
  await expect(page.getByRole('dialog')).toHaveCount(0);
  await expect(page).toHaveURL('/');
  await expect(page.locator('#root')).toHaveJSProperty('inert', false);
  expect(await page.evaluate(() => (window as any).__pageUpdates.length)).toBe(1);
});

test('all objects, filters, search, details and contextual creation reuse business routes', async ({
  page,
}) => {
  await page.locator('.android-navigation a[href="/workspace"]').click();
  await expect(page.getByTestId('workspace-object-card')).toHaveCount(2);
  const identity = page.locator('.android-chip').filter({ hasText: 'Identity' });
  const width = (await identity.boundingBox())!.width;
  await identity.click();
  await expect(identity).toHaveAttribute('aria-pressed', 'true');
  expect((await identity.boundingBox())!.width).toBe(width);
  await expect(page.getByTestId('workspace-object-card')).toHaveCount(1);
  await page.locator('.android-fab').click();
  await page.getByRole('button', { name: /New object/ }).click();
  await expect(page).toHaveURL('/editor?section=identity');
  await page.locator('.android-navigation a[href="/workspace"]').click();
  await page.getByRole('textbox').fill('Savings');
  await expect(page.getByTestId('workspace-object-card')).toHaveCount(1);
  await page.getByRole('button', { name: /^Savings/ }).click();
  await expect(page.getByTestId('object-detail-modal')).toBeVisible();
  await expect(page.getByText('PRIVATE-TEST-VALUE', { exact: true })).toHaveCount(0);
});

test('appearance persists across reload and a locked vault drops cached names', async ({
  page,
}) => {
  await page.locator('.android-navigation a[href="/settings"]').click();
  await page.getByText('Theme & Appearance', { exact: true }).click();
  await page.getByRole('button', { name: 'Dark', exact: true }).click();
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
  await page.getByRole('button', { name: 'Sage', exact: true }).click();
  await page.getByRole('checkbox').check();
  await expect(page.locator('html')).toHaveAttribute('data-user-reduce-motion', 'true');
  await page.reload();
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
  // 重载重新进入锁定页，解锁后读取已保存的账户偏好。
  await login(page);
  await expect(page.locator('html')).toHaveAttribute('data-accent', 'forest');
  await expect(page.locator('html')).toHaveAttribute('data-user-reduce-motion', 'true');
  await page.getByRole('button', { name: /Lock Vault/i }).click();
  await expect(page.getByTestId('android-home')).toHaveCount(0);
  await expect(page.getByText('Passport', { exact: true })).toHaveCount(0);
  await expect(page.locator('.android-sheet')).toHaveCount(0);
});

async function selectGlass(page: import('@playwright/test').Page, label: string) {
  await page.locator('.android-navigation a[href="/settings"]').click();
  await page.getByText('Theme & Appearance', { exact: true }).click();
  await page.getByRole('button', { name: label, exact: true }).click();
}

test('glass switches retain navigation geometry, forced colors use solid surfaces', async ({
  page,
}) => {
  await expect(page.locator('html')).toHaveAttribute('data-android-glass', 'local');
  const before = await page.locator('.android-navigation').boundingBox();
  expect(
    await page.locator('.android-appbar').evaluate((el) => getComputedStyle(el).backdropFilter),
  ).toContain('blur');
  await selectGlass(page, 'Off');
  await expect(page.locator('html')).toHaveAttribute('data-android-glass', 'off');
  expect(await page.locator('.android-navigation').boundingBox()).toEqual(before);
  expect(
    await page.locator('.android-appbar').evaluate((el) => getComputedStyle(el).backdropFilter),
  ).toBe('none');
  await page.getByRole('button', { name: 'Enhanced glass', exact: true }).click();
  await page.emulateMedia({ forcedColors: 'active' });
  await expect(page.locator('html')).toHaveAttribute('data-android-glass', 'off');
  await page.emulateMedia({ forcedColors: 'none' });
  await expect(page.locator('html')).toHaveAttribute('data-android-glass', 'enhanced');
  await page.reload();
  await login(page);
  await expect(page.locator('html')).toHaveAttribute('data-android-glass', 'enhanced');
});

test('liquid artwork recovers context loss, stops when hidden and clears on lock', async ({
  page,
}) => {
  await selectGlass(page, 'Enhanced glass');
  await page.locator('.android-navigation a[href="/"]').click();
  const art = page.locator('.android-liquid-artwork');
  await expect(art).toHaveAttribute('data-liquid-ready', 'true');
  await page.screenshot({ path: test.info().outputPath('enhanced-home.png') });
  // 包装 drawArrays 计数验证实际绘制生命周期，未读取画布中的业务字段。
  await art.locator('canvas').evaluate((canvas) => {
    const gl = (canvas as HTMLCanvasElement).getContext('webgl')!;
    const draw = gl.drawArrays.bind(gl);
    Object.assign(window, {
      __glassFrames: 0,
      __glassContext: gl.getExtension('WEBGL_lose_context'),
    });
    gl.drawArrays = (...args) => {
      (window as any).__glassFrames++;
      draw(...args);
    };
  });
  await page.waitForTimeout(180);
  expect(await page.evaluate(() => (window as any).__glassFrames)).toBe(0);
  await page.evaluate(() => (window as any).__glassContext.loseContext());
  await expect(art).toHaveAttribute('data-liquid-ready', 'false');
  await page.evaluate(() => (window as any).__glassContext.restoreContext());
  await expect(art).toHaveAttribute('data-liquid-ready', 'true');
  await page.setViewportSize({ width: 390, height: 640 });
  await page.locator('[data-shell-content]').evaluate((el) => (el.scrollTop = el.scrollHeight));
  await page.waitForTimeout(180);
  expect(await art.evaluate((el) => el.getBoundingClientRect().bottom)).toBeLessThanOrEqual(0);
  const before = await page.evaluate(() => (window as any).__glassFrames);
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.waitForTimeout(180);
  expect(await page.evaluate(() => (window as any).__glassFrames)).toBe(before);
  await page.getByRole('button', { name: /Lock Vault/i }).click();
  await expect(art).toHaveCount(0);
});

test('enhanced glass follows the applied native theme when WebView media queries disagree', async ({
  page,
}) => {
  await selectGlass(page, 'Enhanced glass');
  await page.getByRole('button', { name: 'System', exact: true }).click();
  await page.locator('.android-navigation a[href="/"]').click();
  const art = page.locator('.android-liquid-artwork');
  await expect(art).toHaveAttribute('data-liquid-ready', 'true');

  for (const theme of ['dark', 'light'] as const) {
    // 原生深浅主题可能与 Android WebView 的 matchMedia 返回值相反。
    await page.emulateMedia({ colorScheme: theme === 'dark' ? 'light' : 'dark' });
    await page.evaluate(async (mode) => {
      await (window as any).__TAURI_INTERNALS__.invoke('plugin:event|emit', {
        event: 'system-theme-changed',
        // 测试桥直接转发 callback 数据，因此传入原生事件信封。
        payload: { event: 'system-theme-changed', id: 0, payload: mode },
      });
    }, theme);
    await expect(page.locator('html')).toHaveAttribute('data-theme', theme);

    const checkSurface = async () => {
      await expect
        .poll(() =>
          art.locator('canvas').evaluate((el) => {
            const gl = (el as HTMLCanvasElement).getContext('webgl')!;
            const program = gl.getParameter(gl.CURRENT_PROGRAM)!;
            const container = Array.from(
              gl.getUniform(program, gl.getUniformLocation(program, 'container')) as Float32Array,
            ).map((channel) => Math.round(channel * 255));
            const css = getComputedStyle(document.documentElement)
              .getPropertyValue('--md-primary-container')
              .trim();
            const expected = [1, 3, 5].map((offset) => parseInt(css.slice(offset, offset + 2), 16));
            return {
              dark: gl.getUniform(program, gl.getUniformLocation(program, 'dark')),
              sameContainer: container.every((value, index) => value === expected[index]),
            };
          }),
        )
        .toEqual({ dark: theme === 'dark' ? 1 : 0, sameContainer: true });
      await expect(page.locator('.android-overview-copy')).toContainText('My vault');
      await expect(page.locator('.android-overview-stat strong')).toHaveText('2');
    };
    await checkSurface();
    // 再次进入首页时也必须使用已解析的主题，不能退回 WebView 的媒体查询。
    await page.locator('.android-navigation a[href="/tools"]').click();
    await page.locator('.android-navigation a[href="/"]').click();
    await expect(art).toHaveAttribute('data-liquid-ready', 'true');
    await checkSurface();
    await page.screenshot({ path: test.info().outputPath(`native-theme-${theme}.png`) });
  }
});

test('native menu returns contextual actions and uses web fallback when unavailable', async ({
  page,
}) => {
  await selectGlass(page, 'Enhanced glass');
  await page.evaluate(() => {
    const mocks = (window as any).__E2E_MOCKS__;
    mocks.android_glass_capabilities = () => ({
      apiLevel: 36,
      windowBlur: true,
      webViewVersion: 'test',
    });
    mocks.android_show_glass_menu = ({ payload }: any) => ({
      requestId: payload.requestId,
      action: 'object',
    });
  });
  await page.locator('.android-navigation a[href="/workspace"]').click();
  await page.locator('.android-chip').filter({ hasText: 'Identity' }).click();
  const historyLength = await page.evaluate(() => history.length);
  await page.locator('.android-fab').click();
  await expect(page).toHaveURL('/editor?section=identity');
  expect(await page.evaluate(() => history.length)).toBe(historyLength + 1);
  await page.locator('.android-navigation a[href="/"]').click();
  await page.evaluate(() => {
    (window as any).__E2E_MOCKS__.android_show_glass_menu = ({ payload }: any) => ({
      requestId: payload.requestId,
      action: 'page',
    });
  });
  await page.locator('.android-fab').click();
  await expect(page.locator('.android-page-form')).toBeVisible();
  await expect(page.getByRole('textbox', { name: 'Page name', exact: true })).toBeFocused();
  await page.keyboard.press('Escape');
  await expect(page.locator('.android-create-options')).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(page.getByRole('dialog')).toHaveCount(0);
  await page.evaluate(() => {
    (window as any).__E2E_MOCKS__.android_glass_capabilities = () => ({ windowBlur: false });
  });
  await page.locator('.android-fab').click();
  await expect(page.locator('.android-create-options')).toBeVisible();
});

test('late native menu results cannot navigate after route change or lock', async ({ page }) => {
  await selectGlass(page, 'Enhanced glass');
  await page.locator('.android-navigation a[href="/"]').click();
  await page.evaluate(() => {
    const mocks = (window as any).__E2E_MOCKS__;
    mocks.android_glass_capabilities = () => ({ windowBlur: true });
    mocks.android_show_glass_menu = ({ payload }: any) =>
      new Promise((resolve) => {
        (window as any).__finishGlass = () =>
          resolve({ requestId: payload.requestId, action: 'object' });
      });
    mocks.android_close_glass_menu = () => {
      (window as any).__glassClosed = true;
    };
  });
  await page.locator('.android-fab').click();
  await expect
    .poll(() => page.evaluate(() => typeof (window as any).__finishGlass))
    .toBe('function');
  await page.locator('.android-navigation a[href="/settings"]').click();
  await expect.poll(() => page.evaluate(() => (window as any).__glassClosed)).toBe(true);
  await page.evaluate(() => (window as any).__finishGlass());
  await expect(page).toHaveURL('/settings');
  await page.getByRole('button', { name: /Lock Vault/i }).click();
  await expect(page).toHaveURL('/login');
});
