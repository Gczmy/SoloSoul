import { expect, test, type Page } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { THEME_SCHEMES, getSchemeById, type ThemeScheme } from '../src/lib/themeSchemes';
import { login, setupTauriMock } from './fixtures/auth';
import { measureControlContrast } from './fixtures/contrast';

const settingsMessages = JSON.parse(
  readFileSync(new URL('../src/locales/en-US/settings.json', import.meta.url), 'utf8'),
);
const authMessages = JSON.parse(
  readFileSync(new URL('../src/locales/en-US/auth.json', import.meta.url), 'utf8'),
);

type Platform = 'android' | 'windows' | 'macos';
type Contrast = 'no-preference' | 'more';

async function prepare(
  page: Page,
  platform: Platform,
  scheme: ThemeScheme,
  biometricLogin = false,
) {
  await page.setViewportSize({ width: platform === 'android' ? 390 : 1000, height: 844 });
  await setupTauriMock(page);
  await page.addInitScript(
    ({ platform, scheme, biometricLogin }) => {
      const prefs = {
        theme: scheme.mode,
        defaultLightTheme: scheme.mode === 'light' ? scheme.id : 'warm-stone',
        defaultDarkTheme: scheme.mode === 'dark' ? scheme.id : 'warm-stone-dark',
        language: 'en-US',
        hasSeenOnboarding: true,
        autoLockTimeoutMinutes: 0,
        autoLockOnBackground: false,
      };
      Object.assign(window, {
        __MOCK_PLATFORM__: platform,
        __E2E_MOCKS__: {
          ui_get_preferences: () => prefs,
          user_data_get_preferences: () => prefs,
          vault_check_directory: () => true,
        },
      });
      type Invoke = (command: string, args?: unknown, options?: unknown) => Promise<unknown>;
      const target = window as typeof window & { __TAURI_INTERNALS__: { invoke: Invoke } };
      const original = target.__TAURI_INTERNALS__.invoke;
      // 在 IPC 边界模拟可用性，只展示实际警告分支，不声称产生了系统/PIN 锁定。
      target.__TAURI_INTERNALS__.invoke = async (command, args, options) => {
        if (command === 'pin_check_availability')
          return { configured: true, locked: true, remainingAttempts: 0, lockedUntil: null };
        if (command === 'biometric_check_availability') {
          if (localStorage.getItem('rf1182-weak-warning') === 'true')
            return {
              available: true,
              configured: false,
              strongAvailable: true,
              weakAvailable: true,
            };
          return {
            available: false,
            configured: biometricLogin,
            biometryType: 'touchId',
            lockout: true,
          };
        }
        return original(command, args, options);
      };
      localStorage.setItem('i18nextLng', 'en-US');
    },
    { platform, scheme: { mode: scheme.mode, id: scheme.id }, biometricLogin },
  );
}

async function themeReady(page: Page, scheme: ThemeScheme) {
  await expect(page.locator('html')).toHaveAttribute('data-theme', scheme.mode);
  await expect
    .poll(() =>
      page.evaluate(() =>
        getComputedStyle(document.documentElement).getPropertyValue('--bg-elevated').trim(),
      ),
    )
    .toBe(scheme.variables['--bg-elevated']);
}

async function warningContrast(
  page: Page,
  message: string,
  surface: string,
  kind: string,
  mode: 'light' | 'dark',
  contrast: Contrast,
) {
  const warning = page.getByText(message, { exact: true });
  await expect(warning).toBeVisible();
  await warning.scrollIntoViewIfNeeded();
  const colors = await measureControlContrast(warning, surface);
  expect.soft(colors.foregroundContrast, `${kind} actual warning body`).toBeGreaterThanOrEqual(4.5);
  if (contrast === 'more')
    expect
      .soft(colors.foreground, `${kind} follows high-contrast foreground`)
      .toEqual(mode === 'dark' ? [255, 255, 255] : [17, 17, 17]);
  await test
    .info()
    .attach(`${kind}-contrast`, { body: JSON.stringify(colors), contentType: 'application/json' });
}

async function security(page: Page, platform: Platform) {
  if (platform === 'android') await page.locator('.android-navigation a[href="/settings"]').click();
  else await page.getByRole('button', { name: 'Settings', exact: true }).first().click();
  await page.getByRole('main').getByText('Security Settings', { exact: true }).click();
  await expect(page).toHaveURL('/settings/security');
}

for (const platform of ['android', 'windows', 'macos'] as const) {
  for (const scheme of THEME_SCHEMES.map(
    (s) => getSchemeById(s.id, platform === 'android' ? 'shared' : 'desktop')!,
  )) {
    for (const contrast of ['no-preference', 'more'] as const) {
      test(`${platform} ${scheme.id} ${contrast} security warning bodies follow readable semantics`, async ({
        page,
      }) => {
        await page.emulateMedia({ contrast });
        await prepare(page, platform, scheme);
        await login(page);
        await themeReady(page, scheme);
        await security(page, platform);
        for (const [kind, message] of [
          ['never', settingsMessages.auto_lock_never_warning],
          ['master', settingsMessages.master_password_warning],
          ['pin', settingsMessages.pin_locked_warning],
          ['biometric', settingsMessages.biometric_lockout_desc],
        ])
          await warningContrast(
            page,
            message,
            '[data-ui-card="default"]',
            kind,
            scheme.mode,
            contrast,
          );
        await page.evaluate(() => localStorage.setItem('rf1182-weak-warning', 'true'));
        await security(page, platform);
        await warningContrast(
          page,
          settingsMessages.biometric_weak_unsupported,
          '[data-ui-card="default"]',
          'weak-biometric',
          scheme.mode,
          contrast,
        );
      });

      test(`${platform} ${scheme.id} ${contrast} login lockout warning stays readable`, async ({
        page,
      }) => {
        await page.emulateMedia({ contrast });
        await prepare(page, platform, scheme, true);
        await page.goto('/login');
        await themeReady(page, scheme);
        await page.getByRole('button', { name: 'Touch ID', exact: true }).click();
        await expect(page.locator('[data-login-method-region="biometric"]')).toBeVisible();
        await warningContrast(
          page,
          settingsMessages.biometric_lockout_desc,
          '[data-login-card]',
          'login-biometric',
          scheme.mode,
          contrast,
        );
      });

      test(`${platform} ${scheme.id} ${contrast} bootstrap warning stays readable`, async ({
        page,
      }) => {
        await page.emulateMedia({ contrast });
        await prepare(page, platform, scheme);
        await page.goto('/bootstrap?mode=create');
        await themeReady(page, scheme);
        await warningContrast(
          page,
          authMessages.master_password_warning,
          '[class*="bootstrapCard"]',
          'bootstrap-master',
          scheme.mode,
          contrast,
        );
      });
    }
  }
}
