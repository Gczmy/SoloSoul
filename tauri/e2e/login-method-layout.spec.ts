import { expect, test, type Page } from '@playwright/test';
import { setupTauriMock } from './fixtures/auth';
import { inflateSync } from 'node:zlib';

type Platform = 'android' | 'ios' | 'macos' | 'windows';

async function openLogin(
  page: Page,
  platform: Platform,
  options: { accountName?: string; theme?: 'light' | 'dark' } = {},
) {
  await setupTauriMock(page);
  await page.addInitScript(
    ({ mockPlatform, accountName, theme }) => {
      type Invoke = (command: string, args?: unknown, options?: unknown) => Promise<unknown>;
      const target = window as typeof window & {
        __TAURI_INTERNALS__: { invoke: Invoke };
      };
      const originalInvoke = target.__TAURI_INTERNALS__.invoke;
      // 通用 fixture 的这两个返回值写在 switch 中，需在 IPC 边界覆盖以展示真实三方式 UI。
      target.__TAURI_INTERNALS__.invoke = async (command, args, options) => {
        if (command === 'vault_list_accounts' && accountName) {
          return [{ id: 'e2e-account', name: accountName }];
        }
        if (command === 'biometric_check_availability') {
          return { available: true, configured: true, biometryType: 'touchId' };
        }
        if (command === 'pin_check_availability') {
          return { configured: true, locked: false };
        }
        return originalInvoke(command, args, options);
      };
      const prefs = {
        theme,
        language: 'en-US',
        hasSeenOnboarding: true,
        reduceMotion: true,
        autoLockTimeoutMinutes: 0,
      };
      Object.assign(window, {
        __MOCK_PLATFORM__: mockPlatform,
        __E2E_MOCKS__: {
          ui_get_preferences: () => prefs,
          user_data_get_preferences: () => prefs,
          vault_check_directory: () => true,
          vault_get_directory: () => ({ directoryType: 'local', valid: true }),
        },
      });
      localStorage.setItem('i18nextLng', 'en-US');
    },
    { mockPlatform: platform, accountName: options.accountName, theme: options.theme ?? 'light' },
  );
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto('/login');
  await expect(page.locator('[data-login-card]')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Touch ID', exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: 'PIN', exact: true })).toBeVisible();
  await page.evaluate(() => document.fonts.ready);
}

// 长账户名不能由卡片 overflow:hidden 静默截掉；检查实际文字行而非 CSS 属性。
for (const platform of ['macos', 'windows', 'android', 'ios'] as const) {
  for (const theme of ['light', 'dark'] as const) {
    test(`${platform} ${theme}: long account name stays readable in short login`, async ({
      page,
    }) => {
      const mobile = platform === 'android' || platform === 'ios';
      await page.setViewportSize({ width: mobile ? 320 : 800, height: mobile ? 568 : 600 });
      const accountName =
        'FE2_LongAccountName_ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789';
      await openLogin(page, platform, { accountName, theme });
      const card = page.locator('[data-login-card]');
      const name = card.locator('[data-login-account]').getByText(accountName, { exact: true });
      await expect(page.locator('html')).toHaveAttribute('data-theme', theme);
      await page.getByRole('button', { name: 'Master password', exact: true }).click();

      for (const largerText of [false, true]) {
        if (largerText) {
          // 前端字号放大样本，不冒称系统字体设置或原生验收。
          await card.evaluate((element) => element.style.setProperty('--text-body', '24px'));
        }
        await expect(name).toHaveText(accountName);
        const text = await name.evaluate((element) => {
          const boundary = element.parentElement!.getBoundingClientRect();
          const range = document.createRange();
          range.selectNodeContents(element);
          return {
            boundary: { left: boundary.left, right: boundary.right },
            lines: [...range.getClientRects()].map((rect) => ({
              left: rect.left,
              right: rect.right,
            })),
          };
        });
        expect(text.lines.length, '连续英文应自然换行，完整保留名称').toBeGreaterThan(1);
        for (const line of text.lines) {
          expect(line.left).toBeGreaterThanOrEqual(text.boundary.left - 0.5);
          expect(line.right).toBeLessThanOrEqual(text.boundary.right + 0.5);
        }
        const controls = [
          card.getByPlaceholder('Enter password'),
          card.locator('[data-login-password-submit]'),
          card.locator('[data-login-quick-links] button').last(),
          card.getByRole('button', { name: 'PIN', exact: true }),
        ];
        for (const control of controls) {
          await control.scrollIntoViewIfNeeded();
          await expect(control).toBeInViewport();
          expect(
            await control.evaluate((element) => {
              const rect = element.getBoundingClientRect();
              const hit = document.elementFromPoint(
                rect.x + rect.width / 2,
                rect.y + rect.height / 2,
              );
              return hit !== null && element.contains(hit);
            }),
            '输入、解锁、恢复和方式切换在滚动后不能被卡片裁剪或遮挡',
          ).toBe(true);
        }
        expect(
          await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth),
        ).toBe(true);
      }
      await page.screenshot({
        path: test.info().outputPath(`${platform}-${theme}-long-account.png`),
        animations: 'disabled',
      });
    });
  }
}

for (const platform of ['macos', 'windows', 'android', 'ios'] as const) {
  for (const theme of ['light', 'dark'] as const) {
    test(`${platform} ${theme}: keyboard login method focus is visible and activates`, async ({
      page,
      browserName,
    }) => {
      await openLogin(page, platform, { theme });
      for (const [label, region] of [
        ['Master password', 'password'],
        ['PIN', 'pin'],
        ['Touch ID', 'biometric'],
      ]) {
        const method = page.getByRole('button', { name: label, exact: true });
        await page.locator('[data-login-brand]').click();
        const before = await method.evaluate((element) => ({
          shadow: getComputedStyle(element).boxShadow,
          width: element.getBoundingClientRect().width,
          height: element.getBoundingClientRect().height,
        }));
        // 通过真实键盘顺序进入，不用 focus() 掩盖缺失的键盘焦点反馈。
        for (let step = 0; step < 20; step++) {
          // Playwright WebKit 默认采用 Safari 的“仅文本控件”Tab策略，Option-Tab遍历按钮。
          await page.keyboard.press(browserName === 'webkit' ? 'Alt+Tab' : 'Tab');
          if (await method.evaluate((element) => element === document.activeElement)) break;
        }
        await expect(method).toBeFocused();
        const focused = await method.evaluate((element) => {
          const style = getComputedStyle(element);
          const rect = element.getBoundingClientRect();
          return {
            keyboard: element.matches(':focus-visible'),
            outline: style.outlineStyle !== 'none' && parseFloat(style.outlineWidth) > 0,
            shadow: style.boxShadow,
            width: rect.width,
            height: rect.height,
          };
        });
        expect(focused.keyboard).toBe(true);
        expect(
          focused.outline || (focused.shadow !== 'none' && focused.shadow !== before.shadow),
          '键盘焦点必须区别于普通选中态，不能只有内部焦点状态',
        ).toBe(true);
        expect(focused.width).toBeCloseTo(before.width, 1);
        expect(focused.height).toBeCloseTo(before.height, 1);
        await page.keyboard.press('Enter');
        await expect(page.locator(`[data-login-method-region="${region}"]`)).toBeVisible();
      }
      // 浏览器高对比标记只验证网页回退规则，不代表原生系统设置验收。
      await page.locator('html').evaluate((element) => {
        element.setAttribute('data-high-contrast', 'true');
      });
      await page.locator('[data-login-brand]').click();
      const biometric = page.getByRole('button', { name: 'Touch ID', exact: true });
      for (let step = 0; step < 20; step++) {
        await page.keyboard.press(browserName === 'webkit' ? 'Alt+Tab' : 'Tab');
        if (await biometric.evaluate((element) => element === document.activeElement)) break;
      }
      await expect(biometric).toBeFocused();
      const ring = await biometric.evaluate((element) => {
        const style = getComputedStyle(element);
        return {
          style: style.outlineStyle,
          width: parseFloat(style.outlineWidth),
          color: style.outlineColor,
        };
      });
      expect(ring.style).toBe('solid');
      expect(ring.width).toBeGreaterThan(0);
      expect(ring.color).toBe(
        await page
          .locator('[data-login-brand] h1')
          .evaluate((element) => getComputedStyle(element).color),
      );
      if (platform === 'windows' && browserName === 'chromium') {
        // 强制颜色必须优先于应用高对比标记，使用系统焦点颜色。
        await page.emulateMedia({ forcedColors: 'active' });
        const highlight = await page.evaluate(() => {
          const probe = document.createElement('span');
          probe.style.color = 'Highlight';
          document.body.appendChild(probe);
          const color = getComputedStyle(probe).color;
          probe.remove();
          return color;
        });
        await expect(biometric).toHaveCSS('outline-color', highlight);
      }
    });
  }
}

const cases: { platform: Platform; width: number; height: number }[] = [
  { platform: 'android', width: 390, height: 844 },
  { platform: 'android', width: 320, height: 844 },
  { platform: 'ios', width: 390, height: 844 },
  { platform: 'macos', width: 1100, height: 800 },
  { platform: 'windows', width: 1100, height: 800 },
];

for (const { platform, width, height } of cases) {
  test(`${platform} ${width}px: login methods stay compact and PIN stays inside the card`, async ({
    page,
  }) => {
    await page.setViewportSize({ width, height });
    await openLogin(page, platform);
    const card = page.locator('[data-login-card]');
    const brand = card.locator('[data-login-brand]');
    const logo = (await brand.locator('img').boundingBox())!;
    const title = (await brand.locator('h1').boundingBox())!;
    expect(logo.x + logo.width).toBeLessThanOrEqual(title.x);
    expect(logo.width).toBe(48);
    expect(title.y).toBeGreaterThanOrEqual(logo.y - 2);
    const heights: number[] = [];
    const methods = [
      { label: 'Master password', region: 'password' },
      { label: 'Touch ID', region: 'biometric' },
      { label: 'PIN', region: 'pin' },
    ];

    for (const { label, region } of methods) {
      await page.getByRole('button', { name: label, exact: true }).click();
      await expect(page.locator(`[data-login-method-region="${region}"]`)).toBeVisible();
      // 同一真实登录卡片切换方法，等两帧再读几何，排除字体/样式尚未提交的测量。
      await page.evaluate(
        () =>
          new Promise<void>((resolve) =>
            requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
          ),
      );
      heights.push((await card.boundingBox())!.height);

      if (region === 'password') {
        const field = await card.locator('.interactive-password-field').boundingBox();
        const submit = await card.locator('[data-login-password-submit]').boundingBox();
        const gap = submit!.y - field!.y - field!.height;
        expect(gap, '保留错误行后，密码框与解锁按钮之间仍应紧凑').toBeGreaterThanOrEqual(8);
        expect(gap).toBeLessThanOrEqual(32);
        const account = (await card.locator('[data-login-account]').boundingBox())!;
        const links = (await card
          .locator('[data-login-quick-links] button')
          .first()
          .boundingBox())!;
        expect(
          field!.y - account.y - account.height,
          '账户与输入框之间避免大块空白',
        ).toBeLessThanOrEqual(40);
        expect(
          links.y - submit!.y - submit!.height,
          '解锁与创建账户之间避免叠加留白',
        ).toBeLessThanOrEqual(36);
      }

      if (region === 'pin') {
        await expect(card.locator('[data-pin-digit]')).toHaveCount(6);
        const geometry = await card.evaluate((element) => {
          const rect = element.getBoundingClientRect();
          const style = getComputedStyle(element);
          const input = element.querySelector('[data-pin-input]')!.getBoundingClientRect();
          const digits = [...element.querySelectorAll('[data-pin-digit]')].map((digit) => {
            const box = digit.getBoundingClientRect();
            return { left: box.left, right: box.right, width: box.width };
          });
          return {
            contentLeft:
              rect.left + parseFloat(style.paddingLeft) + parseFloat(style.borderLeftWidth),
            contentRight:
              rect.right - parseFloat(style.paddingRight) - parseFloat(style.borderRightWidth),
            inputLeft: input.left,
            inputRight: input.right,
            viewport: window.innerWidth,
            documentWidth: document.documentElement.scrollWidth,
            digits,
          };
        });
        expect(geometry.inputLeft).toBeGreaterThanOrEqual(geometry.contentLeft - 0.5);
        expect(geometry.inputRight).toBeLessThanOrEqual(geometry.contentRight + 0.5);
        expect(geometry.documentWidth).toBeLessThanOrEqual(geometry.viewport);
        for (const digit of geometry.digits) {
          expect(digit.width).toBeGreaterThan(0);
          expect(digit.left).toBeGreaterThanOrEqual(geometry.inputLeft - 0.5);
          expect(digit.right).toBeLessThanOrEqual(geometry.inputRight + 0.5);
          expect(digit.left).toBeGreaterThanOrEqual(0);
          expect(digit.right).toBeLessThanOrEqual(geometry.viewport);
        }
      }

      await page.screenshot({
        path: test.info().outputPath(`${platform}-${width}-${region}.png`),
        animations: 'disabled',
      });
    }

    expect(
      Math.max(...heights) - Math.min(...heights),
      '正常三方式的卡片应等高',
    ).toBeLessThanOrEqual(2);

    // 错误必须可读；不把正常态等高约束套到错误态，允许长文案撑开卡片。
    await page.getByRole('button', { name: 'Master password', exact: true }).click();
    await card.locator('[data-login-password-submit]').click();
    const error = card.locator('[data-login-method-region="password"] [role="alert"]');
    await expect(error).toBeVisible();
    await expect(error).not.toBeEmpty();
    await expect(card.locator('[data-login-password-submit]')).toBeVisible();
    await test.info().attach('normal-card-heights', {
      body: JSON.stringify({
        platform,
        width,
        methods: methods.map((method) => method.region),
        heights,
      }),
      contentType: 'application/json',
    });
  });
}

// 错误提示是正文，不能复用为白字按钮准备的危险底色；标记回退不冒称系统验收。
for (const platform of ['macos', 'windows', 'android', 'ios'] as const) {
  for (const theme of ['light', 'dark'] as const) {
    test(`${platform} ${theme}: login errors have readable contrast`, async ({ page }) => {
      await openLogin(page, platform, { theme });
      await page.evaluate(() => {
        const app = window as unknown as { __E2E_MOCKS__: Record<string, unknown> };
        app.__E2E_MOCKS__.pin_unlock = () => {
          throw '__PIN_ERR__:incorrect';
        };
      });
      for (const highContrast of [false, true]) {
        await page.locator('html').evaluate((element, active) => {
          element.setAttribute('data-high-contrast', String(active));
        }, highContrast);
        await page.getByRole('button', { name: 'Master password', exact: true }).click();
        await page.locator('[data-login-password-submit]').click();
        const passwordError = page.locator('[data-login-method-region="password"] [role="alert"]');
        await expect(passwordError.filter({ hasText: 'Password is required' })).toBeVisible();
        // 切回主密码后原 PIN 错误仍在独立错误区显示，两个提示都必须可读。
        for (const error of await passwordError.all()) await checkContrast(error, highContrast);
        await page.getByRole('button', { name: 'PIN', exact: true }).click();
        await page.locator('[data-pin-input] input').fill('123456');
        const pinError = page
          .locator('[data-login-method-region="pin"]')
          .getByText('Incorrect PIN. Please try again.', { exact: true });
        await expect(pinError).toBeVisible();
        await checkContrast(pinError, highContrast);
      }
      async function checkContrast(error: ReturnType<Page['locator']>, highContrast: boolean) {
        const text = await error.evaluate((element) => getComputedStyle(element).color);
        const rect = (await error.boundingBox())!;
        const card = (await page.locator('[data-login-card]').boundingBox())!;
        // Android 登录卡片有透明叠层及 backdrop-filter。取错误左侧空白的实际绘制像素，
        // 不把 RGBA 原始通道当成最终背景，也不改成实色以绕过当前材质。
        const x = Math.floor(rect.x - 4);
        const y = Math.floor(rect.y + rect.height / 2);
        expect(x).toBeGreaterThan(card.x + 1);
        expect(y).toBeGreaterThan(card.y + 1);
        expect(y).toBeLessThan(card.y + card.height - 1);
        const pixel = await page.screenshot({ clip: { x, y, width: 1, height: 1 }, scale: 'css' });
        const background = onePixelRgb(pixel);
        const colors = { text, background };
        const luminance = (color: string) => {
          const parts = color.match(/[\d.]+/g)!.map(Number);
          expect(parts.length === 3 || parts[3] === 1, '测量的卡片背景必须不透明').toBe(true);
          const channels = parts.slice(0, 3).map((n) => {
            const c = n / 255;
            return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
          });
          return channels[0] * 0.2126 + channels[1] * 0.7152 + channels[2] * 0.0722;
        };
        const [low, high] = [luminance(colors.text), luminance(colors.background)].sort(
          (a, b) => a - b,
        );
        expect(
          (high + 0.05) / (low + 0.05),
          JSON.stringify({ platform, theme, highContrast, ...colors }),
        ).toBeGreaterThanOrEqual(4.5);
      }
    });
  }
}

/** Playwright 自产的1×1 PNG：首像素无左右/上方预测项，五种 PNG 行滤镜均无需反算。 */
function onePixelRgb(png: Buffer): string {
  expect(png.subarray(0, 8)).toEqual(Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]));
  const parts: Buffer[] = [];
  let channels = 0;
  for (let offset = 8; offset < png.length; ) {
    const size = png.readUInt32BE(offset);
    const type = png.toString('ascii', offset + 4, offset + 8);
    const data = png.subarray(offset + 8, offset + 8 + size);
    if (type === 'IHDR') {
      expect(data.readUInt32BE(0)).toBe(1);
      expect(data.readUInt32BE(4)).toBe(1);
      expect(data[8]).toBe(8);
      expect([2, 6]).toContain(data[9]);
      expect([...data.subarray(10)]).toEqual([0, 0, 0]);
      channels = data[9] === 6 ? 4 : 3;
    } else if (type === 'IDAT') parts.push(data);
    offset += size + 12;
  }
  const raw = inflateSync(Buffer.concat(parts), { maxOutputLength: channels + 1 });
  expect(raw.length).toBe(channels + 1);
  expect(raw[0]).toBeLessThanOrEqual(4);
  if (channels === 4) expect(raw[4]).toBe(255);
  return `rgb(${raw[1]}, ${raw[2]}, ${raw[3]})`;
}
