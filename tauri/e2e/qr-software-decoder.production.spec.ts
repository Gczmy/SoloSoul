import { expect, test } from '@playwright/test';
import { login, setupTauriMock } from './fixtures/auth';

interface QrFixtureState {
  nativeProbes: number;
  frames: number;
  canvas: HTMLCanvasElement | null;
  streams: MediaStream[];
  timer?: number;
}

for (const theme of ['light', 'dark'] as const) {
  test(`android ${theme} decodes public QR pixels without native BarcodeDetector`, async ({
    page,
  }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await setupTauriMock(page);
    await page.addInitScript(
      ({ theme }) => {
        const prefs = {
          theme,
          language: 'en-US',
          hasSeenOnboarding: true,
          autoLockTimeoutMinutes: 0,
        };
        const state: QrFixtureState = { nativeProbes: 0, frames: 0, canvas: null, streams: [] };
        Object.assign(window, {
          __MOCK_PLATFORM__: 'android',
          __RF1177_QR__: state,
          __E2E_MOCKS__: {
            ui_get_preferences: () => prefs,
            user_data_get_preferences: () => prefs,
            sync_get_status: () => ({
              isDiscovering: false,
              syncEnabled: true,
              autoSyncEnabled: false,
              localFingerprint: '0'.repeat(64),
              connectedPeers: [],
            }),
            sync_listen_addr: () => '127.0.0.1:42069',
            sync_list_conflicts: () => [],
            mdns_discover: () => [],
            sync_generate_qr_payload: () =>
              JSON.stringify({
                t: 'sync',
                a: '127.0.0.1:42069',
                f: '0'.repeat(64),
                n: 'RF1177 Public device',
              }),
          },
        });
        class NativeBarcodeProbe {
          constructor() {
            state.nativeProbes += 1;
            throw new Error('RF1177 Android fixture must not probe native BarcodeDetector');
          }
          static async getSupportedFormats() {
            state.nativeProbes += 1;
            throw new Error('RF1177 Android fixture must not query native formats');
          }
        }
        Object.defineProperty(window, 'BarcodeDetector', {
          configurable: true,
          value: NativeBarcodeProbe,
        });
        Object.defineProperty(navigator, 'mediaDevices', {
          configurable: true,
          value: {
            enumerateDevices: async () => [
              {
                deviceId: 'rf1177-public-camera',
                kind: 'videoinput',
                label: 'Public QR pixels',
                groupId: 'public',
              },
            ],
            getUserMedia: async () => {
              if (!state.canvas)
                throw new DOMException('Public pixel source not prepared', 'NotReadableError');
              const stream = state.canvas.captureStream(10);
              state.streams.push(stream);
              return stream;
            },
          },
        });
        localStorage.setItem('i18nextLng', 'en-US');
      },
      { theme },
    );

    await login(page);
    await expect(page.locator('html')).toHaveAttribute('data-platform', 'android');
    await expect(page.locator('html')).toHaveAttribute('data-theme', theme);
    await page.locator('.android-navigation a[href="/settings"]').click();
    await page.getByRole('main').getByText('Device Sync', { exact: true }).click();
    await expect(page).toHaveURL('/sync');
    await page.getByRole('button', { name: 'Show QR', exact: true }).click();
    const card = page.locator('[data-ui-card="floating"]');
    await expect(card.locator('svg[height="200"]')).toBeVisible();
    await card.locator('svg[height="200"]').evaluate(async (node) => {
      const state = (window as unknown as { __RF1177_QR__: QrFixtureState }).__RF1177_QR__;
      const svg = node.cloneNode(true) as SVGElement;
      svg.setAttribute('width', '448');
      svg.setAttribute('height', '448');
      const image = new Image();
      const url = URL.createObjectURL(
        new Blob([new XMLSerializer().serializeToString(svg)], { type: 'image/svg+xml' }),
      );
      try {
        image.src = url;
        await image.decode();
        const canvas = document.createElement('canvas');
        canvas.width = canvas.height = 512;
        const context = canvas.getContext('2d')!;
        const paint = () => {
          context.imageSmoothingEnabled = false;
          context.fillStyle = '#fff';
          context.fillRect(0, 0, 512, 512);
          context.drawImage(image, 32, 32, 448, 448);
          state.frames += 1;
        };
        paint();
        state.canvas = canvas;
        state.timer = window.setInterval(paint, 100);
      } finally {
        URL.revokeObjectURL(url);
      }
    });
    await card.locator('button[aria-label="Close"]').click();
    await expect(card).toHaveCount(0);
    await page.getByRole('button', { name: 'Scan QR', exact: true }).click();
    // A native probe is a precise regression; successful confirmation requires
    // the actual html5-qrcode/ZXing decoder to consume the generated QR pixels.
    await page.waitForFunction(() => {
      const state = (window as unknown as { __RF1177_QR__: QrFixtureState }).__RF1177_QR__;
      return (
        state.nativeProbes > 0 ||
        [...document.querySelectorAll('button')].some((e) => e.textContent?.trim() === 'Sync Now')
      );
    });
    const stats = await page.evaluate(() => {
      const state = (window as unknown as { __RF1177_QR__: QrFixtureState }).__RF1177_QR__;
      return {
        nativeProbes: state.nativeProbes,
        frames: state.frames,
        streams: state.streams.length,
      };
    });
    expect(stats.nativeProbes).toBe(0);
    expect(stats.frames).toBeGreaterThan(0);
    expect(stats.streams).toBeGreaterThan(0);
    await expect(
      card.getByText('Start syncing with RF1177 Public device?', { exact: true }),
    ).toBeVisible();
    await expect(card.getByText('127.0.0.1:42069', { exact: true })).toBeVisible();
    await expect(card.getByText('0'.repeat(64), { exact: true })).toBeVisible();
    await expect(card.getByRole('button', { name: 'Sync Now', exact: true })).toBeVisible();
    await expect
      .poll(() =>
        page.evaluate(() => {
          const state = (window as unknown as { __RF1177_QR__: QrFixtureState }).__RF1177_QR__;
          return state.streams.every((stream) =>
            stream.getTracks().every((track) => track.readyState === 'ended'),
          );
        }),
      )
      .toBe(true);
    await page.screenshot({ path: test.info().outputPath('software-qr-decoded.png') });
    await card.locator('button[aria-label="Close"]').click();
    await expect(card).toHaveCount(0);
    await page.evaluate(() => {
      const state = (window as unknown as { __RF1177_QR__: QrFixtureState }).__RF1177_QR__;
      window.clearInterval(state.timer);
      state.streams.forEach((stream) => stream.getTracks().forEach((track) => track.stop()));
    });
  });
}
