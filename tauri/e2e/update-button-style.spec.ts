import { expect, test, type Page } from '@playwright/test';
import { login } from './fixtures/auth';
import { measureControlContrast } from './fixtures/contrast';

type Platform = 'android' | 'macos' | 'windows';
type Theme = 'light' | 'dark';
type UpdateKind = 'available' | 'downloaded' | 'error';
type Scenario = { platform: Platform; theme: Theme; customAccent?: string };

async function setup(page: Page, scenario: Scenario) {
  await page.addInitScript({ path: 'e2e/fixtures/tauriMock.js' });
  await page.addInitScript(({ platform, theme, customAccent }) => {
    const prefs = {
      theme,
      accentColor: customAccent ? 'custom' : 'ocean',
      customAccentHex: customAccent ?? '',
      hasSeenOnboarding: true,
      language: 'en-US',
      reduceMotion: true,
      autoLockTimeoutMinutes: 0,
      sidebarPosition: 'left',
    };
    const titlebarHeight = platform === 'macos' ? 52 : platform === 'windows' ? 40 : 0;
    const material =
      platform === 'macos' ? 'liquid-glass' : platform === 'windows' ? 'mica' : 'solid';
    let downloadAttempt = 0;
    const simulateDownload = () => {
      if (++downloadAttempt === 1) throw new Error('A simulated download failure.');
      return 2;
    };
    Object.assign(window, {
      __MOCK_PLATFORM__: platform,
      __E2E_MOCKS__: {
        ui_get_preferences: () => prefs,
        user_data_get_preferences: () => prefs,
        vault_check_directory: () => true,
        set_titlebar_color: () => ({
          material,
          platform,
          titlebarHeight,
          trafficLightsRight: platform === 'macos' ? 79 : 0,
          reduceMotion: true,
          highContrast: false,
        }),
        get_window_layout: () => ({ platform, titlebarHeight, trafficLightsRight: 0 }),
        desktop_prepare_update: () => ({
          rid: 1,
          currentVersion: '2.13.0',
          version: '9.9.9',
          body: 'A local style fixture.',
          rawJson: {},
        }),
        android_check_update: () => ({
          currentVersion: '2.13.0',
          latestVersion: '9.9.9',
          downloadUrl: 'https://example.test/app.apk',
          releaseNotes: 'A local style fixture.',
          mandatory: false,
          checksum: 'verified',
        }),
        android_is_apk_downloaded: () => false,
        create_update_download: () => 3,
        android_download_apk: simulateDownload,
        desktop_download_update: simulateDownload,
        android_install_apk: () => {
          throw new Error('Style verification must not install an update');
        },
        desktop_install_update: () => {
          throw new Error('Style verification must not install an update');
        },
        get_app_info: () => ({
          appName: 'SoloSoul',
          version: '2.13.0',
          os: platform,
          arch: 'aarch64',
        }),
        ocr_get_model_status: () => ({ installed: true, bundled: true }),
        sync_list_conflicts: () => [],
      },
    });
    localStorage.setItem('i18nextLng', 'en-US');
  }, scenario);
  await page.emulateMedia({ reducedMotion: 'reduce', colorScheme: scenario.theme });
  await page.setViewportSize(
    scenario.platform === 'android' ? { width: 390, height: 844 } : { width: 1100, height: 800 },
  );
  await login(page);
  await expect(page.locator('html')).toHaveAttribute('data-theme', scenario.theme);
  await expect(
    page.locator('[data-notification-banner="update"]').getByRole('button', { name: 'Update Now' }),
  ).toBeVisible();
}

async function setUpdateKind(page: Page, kind: UpdateKind) {
  if (kind === 'available') return;
  // 第一次模拟下载失败，重试模拟成功；驱动真实任务流，永不点击安装按钮。
  await page
    .locator('[data-notification-banner="update"]')
    .getByRole('button', { name: kind === 'error' ? 'Update Now' : 'Retry', exact: true })
    .click();
}

const scenarios: Scenario[] = (['android', 'macos', 'windows'] as const).flatMap((platform) =>
  (['light', 'dark'] as const).map((theme) => ({ platform, theme })),
);
scenarios.push(
  { platform: 'windows', theme: 'light', customAccent: '#ffffff' },
  { platform: 'windows', theme: 'dark', customAccent: '#17382A' },
  { platform: 'windows', theme: 'light', customAccent: '#777777' },
);

for (const scenario of scenarios) {
  const { platform, theme, customAccent } = scenario;
  test(`${platform} ${theme} ${customAccent ?? 'ocean'}: update actions retain contrast and platform shape`, async ({
    page,
  }) => {
    await setup(page, scenario);
    const banner = page.locator('[data-notification-banner="update"]');
    const primary = banner.locator('[data-ui-button="primary"]');
    const states: { kind: UpdateKind; label: string }[] = [
      { kind: 'available', label: 'Update Now' },
      { kind: 'error', label: 'Retry' },
      { kind: 'downloaded', label: 'Install Update' },
    ];
    const measurements = [];
    for (const { kind, label } of states) {
      await setUpdateKind(page, kind);
      await expect(primary).toHaveAccessibleName(label);
      await page.mouse.move(0, 0);
      const idle = await measureControlContrast(primary, '[data-notification-banner="update"]');
      measurements.push({ kind, state: 'idle', ...idle });
      expect(idle.backgroundImage, '此测试只按实色按钮计算颜色对比').toBe('none');
      expect(idle.foregroundContrast, `${kind} 文字对比`).toBeGreaterThanOrEqual(4.5);
      expect(idle.iconContrast, `${kind} 图标对比`).toBeGreaterThanOrEqual(4.5);
      expect(
        Math.max(idle.surfaceContrast, idle.borderContrast),
        `${kind} 按钮边界可区分`,
      ).toBeGreaterThanOrEqual(3);
      if (customAccent) {
        const expected = [1, 3, 5].map((offset) =>
          parseInt(customAccent.slice(offset, offset + 2), 16),
        );
        expect(idle.background, '必须实际验证自定义强调色，而非默认色回退').toEqual(expected);
      }
      if (platform === 'android') {
        expect(idle.width).toBeGreaterThanOrEqual(48);
        expect(idle.height).toBeGreaterThanOrEqual(48);
        expect(idle.borderRadius).toBeGreaterThanOrEqual(idle.height / 2 - 0.5);
      } else {
        await primary.hover();
        await expect
          .poll(async () => {
            const color = await measureControlContrast(
              primary,
              '[data-notification-banner="update"]',
            );
            return Math.min(color.foregroundContrast, color.iconContrast);
          })
          .toBeGreaterThanOrEqual(4.5);
        const hover = await measureControlContrast(primary, '[data-notification-banner="update"]');
        measurements.push({ kind, state: 'hover', ...hover });
        expect(hover.foregroundContrast, `${kind} hover文字对比`).toBeGreaterThanOrEqual(4.5);
        expect(hover.iconContrast, `${kind} hover图标对比`).toBeGreaterThanOrEqual(4.5);
        expect(
          Math.max(hover.surfaceContrast, hover.borderContrast),
          `${kind} hover边界可区分`,
        ).toBeGreaterThanOrEqual(3);
      }
      await banner.screenshot({
        path: test.info().outputPath(`${kind}.png`),
        animations: 'disabled',
      });
    }
    await test.info().attach('contrast-measurements', {
      body: JSON.stringify(measurements, null, 2),
      contentType: 'application/json',
    });
  });
}
