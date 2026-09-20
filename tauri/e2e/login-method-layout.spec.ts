import { expect, test, type Page } from '@playwright/test';
import { setupTauriMock } from './fixtures/auth';

type Platform = 'android' | 'ios' | 'macos' | 'windows';

async function openLogin(page: Page, platform: Platform) {
  await setupTauriMock(page);
  await page.addInitScript((mockPlatform) => {
    type Invoke = (command: string, args?: unknown, options?: unknown) => Promise<unknown>;
    const target = window as typeof window & {
      __TAURI_INTERNALS__: { invoke: Invoke };
    };
    const originalInvoke = target.__TAURI_INTERNALS__.invoke;
    // 通用 fixture 的这两个返回值写在 switch 中，需在 IPC 边界覆盖以展示真实三方式 UI。
    target.__TAURI_INTERNALS__.invoke = async (command, args, options) => {
      if (command === 'biometric_check_availability') {
        return { available: true, configured: true, biometryType: 'touchId' };
      }
      if (command === 'pin_check_availability') {
        return { configured: true, locked: false };
      }
      return originalInvoke(command, args, options);
    };
    const prefs = {
      theme: 'light',
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
  }, platform);
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto('/login');
  await expect(page.locator('[data-login-card]')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Touch ID', exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: 'PIN', exact: true })).toBeVisible();
  await page.evaluate(() => document.fonts.ready);
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
