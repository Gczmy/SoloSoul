import { expect, test, type Locator, type Page } from '@playwright/test';
import { login, setupTauriMock } from './fixtures/auth';

const longAddress = '[2001:db8:1234:5678:90ab:cdef:1234:5678]:42069';
const fingerprint = '0123456789abcdef'.repeat(4);
const deviceName = 'PublicDevice'.repeat(5);
const pin = '123456';
type PixelState = { canvas: HTMLCanvasElement | null; timer?: number; streams: MediaStream[] };

async function measure(card: Locator, values: string[], stage: string) {
  const geometry = await card.evaluate((node, values) => {
    const cardRect = node.getBoundingClientRect();
    const fields = values.flatMap((value) =>
      [...node.querySelectorAll('span, div, p')]
        .filter((e) => e.textContent === value && e.children.length === 0)
        .map((e) => {
          const range = document.createRange();
          range.selectNodeContents(e);
          const parent = e.parentElement!.getBoundingClientRect();
          const rect = e.getBoundingClientRect();
          const style = getComputedStyle(e);
          return {
            value,
            tag: e.tagName,
            parentText: e.parentElement!.textContent,
            field: { left: rect.left, right: rect.right, width: rect.width },
            parent: { left: parent.left, right: parent.right, width: parent.width },
            lines: [...range.getClientRects()].map((r) => ({ left: r.left, right: r.right })),
            ellipsis: style.textOverflow === 'ellipsis',
            fontSize: style.fontSize,
          };
        }),
    );
    return {
      card: { left: cardRect.left, right: cardRect.right },
      clientWidth: node.clientWidth,
      scrollWidth: node.scrollWidth,
      fields,
    };
  }, values);
  await test.info().attach(`${stage}-geometry`, {
    body: JSON.stringify(geometry, null, 2),
    contentType: 'application/json',
  });
  expect
    .soft(geometry.scrollWidth - geometry.clientWidth, `${stage}: card overflow`)
    .toBeLessThanOrEqual(1);
  for (const value of values) {
    const fields = geometry.fields.filter((f) => f.value === value);
    expect.soft(fields.length, `${stage}: full field ${value}`).toBeGreaterThan(0);
    for (const field of fields) {
      expect.soft(field.ellipsis, `${stage}: full value must remain readable`).toBe(false);
      for (const line of field.lines) {
        expect
          .soft(line.left, `${stage}: text left`)
          .toBeGreaterThanOrEqual(Math.max(geometry.card.left, field.parent.left) - 1);
        expect
          .soft(line.right, `${stage}: text right`)
          .toBeLessThanOrEqual(Math.min(geometry.card.right, field.parent.right) + 1);
      }
    }
  }
}

async function screenshot(page: Page, name: string) {
  await page.screenshot({ path: test.info().outputPath(`${name}.png`) });
}

for (const platform of ['android', 'windows', 'macos'] as const) {
  for (const theme of ['light', 'dark'] as const) {
    for (const textScale of [1, 2]) {
      for (const variant of platform === 'android' && textScale === 2
        ? ['long', 'native-address']
        : ['long']) {
        const address = variant === 'native-address' ? '10.0.2.15:35965' : longAddress;
        test(`${platform} ${theme} ${textScale}x ${variant} QR metadata remains fully readable`, async ({
          page,
        }) => {
          await page.setViewportSize({ width: platform === 'android' ? 320 : 900, height: 640 });
          await setupTauriMock(page);
          await page.addInitScript(
            ({ platform, theme, address, fingerprint, deviceName, pin }) => {
              const prefs = {
                theme,
                language: 'en-US',
                hasSeenOnboarding: true,
                autoLockTimeoutMinutes: 0,
              };
              const state: PixelState = { canvas: null, streams: [] };
              Object.assign(window, {
                __MOCK_PLATFORM__: platform,
                __RF1178_PIXELS__: state,
                __RF1178_COPIES__: [],
                __RF1178_CANCELS__: 0,
                __E2E_MOCKS__: {
                  vault_check_directory: () => true,
                  ui_get_preferences: () => prefs,
                  user_data_get_preferences: () => prefs,
                  sync_get_status: () => ({
                    isDiscovering: false,
                    syncEnabled: true,
                    autoSyncEnabled: false,
                    localFingerprint: fingerprint,
                    connectedPeers: [],
                  }),
                  sync_listen_addr: () => address,
                  sync_list_conflicts: () => [],
                  mdns_discover: () => [],
                  sync_generate_qr_payload: () =>
                    JSON.stringify({ t: 'sync', a: address, f: fingerprint, n: deviceName }),
                  recovery_host_start: () => ({
                    displayAddr: address,
                    bindAddr: address,
                    pin,
                    fingerprint,
                    nonce: 'public-layout',
                    qrPayload: JSON.stringify({ t: 'rec', a: address, p: pin }),
                  }),
                  recovery_host_cancel: () => {
                    (window as unknown as { __RF1178_CANCELS__: number }).__RF1178_CANCELS__ += 1;
                  },
                },
              });
              // This layout fixture supplies no native decoder capability. The real
              // existing JavaScript decoder consumes pixels; this is not native camera proof.
              Reflect.deleteProperty(window, 'BarcodeDetector');
              Object.defineProperty(navigator, 'clipboard', {
                configurable: true,
                value: {
                  writeText: async (value: string) => {
                    (window as unknown as { __RF1178_COPIES__: string[] }).__RF1178_COPIES__.push(
                      value,
                    );
                  },
                },
              });
              Object.defineProperty(navigator, 'mediaDevices', {
                configurable: true,
                value: {
                  enumerateDevices: async () => [
                    {
                      deviceId: 'public-pixels',
                      kind: 'videoinput',
                      label: 'Public pixels',
                      groupId: 'public',
                    },
                  ],
                  getUserMedia: async () => {
                    if (!state.canvas)
                      throw new DOMException('Pixels not prepared', 'NotReadableError');
                    const stream = state.canvas.captureStream(10);
                    state.streams.push(stream);
                    return stream;
                  },
                },
              });
              localStorage.setItem('i18nextLng', 'en-US');
            },
            { platform, theme, address, fingerprint, deviceName, pin },
          );
          await login(page);
          await expect(page.locator('html')).toHaveAttribute('data-theme', theme);
          await expect(page.locator('html')).toHaveAttribute(
            platform === 'android' ? 'data-platform' : 'data-desktop-platform',
            platform,
          );
          if (platform === 'android')
            await page.locator('.android-navigation a[href="/settings"]').click();
          else await page.getByRole('button', { name: 'Settings', exact: true }).first().click();
          await page.getByRole('main').getByText('Device Sync', { exact: true }).click();
          await expect(page).toHaveURL('/sync');
          // Explicit token scaling simulates large browser text only. It does not
          // replace the parent task's real Android system-font-scale matrix.
          const typography = await page.evaluate((scale) => {
            const root = document.documentElement;
            const computed = getComputedStyle(root);
            const tokens = [...computed]
              .filter((k) => k.startsWith('--text-'))
              .map((key) => [key, computed.getPropertyValue(key).trim()])
              .filter(([, value]) => /^(?:\d+(?:\.\d+)?|\.\d+)(px|rem)$/.test(value));
            const rootSize = parseFloat(computed.fontSize);
            const body = computed.getPropertyValue('--text-body-sm').trim();
            const expectedBodySize =
              parseFloat(body) * (body.endsWith('rem') ? rootSize : 1) * scale;
            for (const [key, value] of tokens)
              root.style.setProperty(
                key,
                `${parseFloat(value) * (value.endsWith('rem') ? rootSize : 1) * scale}px`,
              );
            return {
              tokens,
              expectedBodySize,
              actualToken: root.style.getPropertyValue('--text-body-sm'),
            };
          }, textScale);
          expect(typography.tokens.length).toBeGreaterThan(5);
          await test.info().attach('typography', {
            body: JSON.stringify(typography),
            contentType: 'application/json',
          });
          await page.getByRole('button', { name: 'Show QR', exact: true }).click();
          const card = page.locator('[data-ui-card="floating"]');
          const svg = card.locator('svg[height="200"]');
          await expect(svg).toBeVisible();
          expect(
            await card
              .getByText(address, { exact: true })
              .first()
              .evaluate((node) => parseFloat(getComputedStyle(node).fontSize)),
          ).toBeCloseTo(typography.expectedBodySize, 1);
          await measure(card, [deviceName, address, fingerprint], 'sync-display');
          await svg.evaluate(async (node) => {
            const state = (window as unknown as { __RF1178_PIXELS__: PixelState })
              .__RF1178_PIXELS__;
            const clone = node.cloneNode(true) as SVGElement;
            clone.setAttribute('width', '448');
            clone.setAttribute('height', '448');
            const image = new Image();
            const url = URL.createObjectURL(
              new Blob([new XMLSerializer().serializeToString(clone)], { type: 'image/svg+xml' }),
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
              };
              paint();
              state.canvas = canvas;
              state.timer = window.setInterval(paint, 100);
            } finally {
              URL.revokeObjectURL(url);
            }
          });
          await card.getByText(fingerprint, { exact: true }).scrollIntoViewIfNeeded();
          await screenshot(page, 'sync-long-metadata');
          await card.getByRole('button', { name: 'Recovery QR', exact: true }).click();
          await expect(card.getByText(pin, { exact: true })).toBeVisible();
          await card
            .getByRole('button', { name: 'No camera? Enter details manually', exact: true })
            .click();
          await measure(card, [address, pin], 'recovery-expanded');
          await card.getByRole('button', { name: 'Copy', exact: true }).first().click();
          await card.getByRole('button', { name: 'Copy', exact: true }).last().click();
          expect(
            await page.evaluate(
              () => (window as unknown as { __RF1178_COPIES__: string[] }).__RF1178_COPIES__,
            ),
          ).toEqual([address, pin]);
          await screenshot(page, 'recovery-long-metadata');
          await page.setViewportSize({ width: platform === 'android' ? 640 : 1280, height: 280 });
          await measure(card, [address, pin], 'recovery-landscape');
          const close = card.locator('button[aria-label="Close"]');
          await close.scrollIntoViewIfNeeded();
          await expect(close).toBeInViewport({ ratio: 1 });
          await close.click();
          await expect(card).toHaveCount(0);
          await expect
            .poll(() =>
              page.evaluate(
                () => (window as unknown as { __RF1178_CANCELS__: number }).__RF1178_CANCELS__,
              ),
            )
            .toBe(1);
          if (variant === 'native-address') {
            await page.evaluate(() =>
              window.clearInterval(
                (window as unknown as { __RF1178_PIXELS__: PixelState }).__RF1178_PIXELS__.timer,
              ),
            );
            return;
          }
          await page.setViewportSize({ width: platform === 'android' ? 320 : 900, height: 640 });
          await page.getByRole('button', { name: 'Scan QR', exact: true }).click();
          await expect(card.getByRole('button', { name: 'Sync Now', exact: true })).toBeVisible();
          await measure(card, [address, fingerprint], 'scan-confirmation');
          const prompt = card.locator('p').filter({ hasText: deviceName });
          await expect(prompt).toContainText(deviceName);
          const promptWidth = await prompt.evaluate((node) => ({
            width: node.clientWidth,
            scroll: node.scrollWidth,
          }));
          expect
            .soft(promptWidth.scroll - promptWidth.width, 'confirmation device name overflow')
            .toBeLessThanOrEqual(1);
          await card.getByText(fingerprint, { exact: true }).scrollIntoViewIfNeeded();
          await screenshot(page, 'scan-long-metadata');
          await card.getByRole('button', { name: 'Cancel', exact: true }).click();
          await expect(card.getByRole('button', { name: 'Sync Now', exact: true })).toHaveCount(0);
          await close.click();
          await expect(card).toHaveCount(0);
          await expect
            .poll(() =>
              page.evaluate(() => {
                const state = (window as unknown as { __RF1178_PIXELS__: PixelState })
                  .__RF1178_PIXELS__;
                return (
                  state.streams.length > 0 &&
                  state.streams.every((s) => s.getTracks().every((t) => t.readyState === 'ended'))
                );
              }),
            )
            .toBe(true);
          await page.evaluate(() =>
            window.clearInterval(
              (window as unknown as { __RF1178_PIXELS__: PixelState }).__RF1178_PIXELS__.timer,
            ),
          );
        });
      }
    }
  }
}
