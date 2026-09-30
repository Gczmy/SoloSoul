import { expect, test } from '@playwright/test';
import { readFileSync, readdirSync } from 'node:fs';

const config = JSON.parse(readFileSync('src-tauri/tauri.conf.json', 'utf8'));
const productionCsp: string = config.app.security.csp;
const fixtureUrl = 'http://127.0.0.1:1423/__csp_ipc_fixture__';
const ipcUrl = 'http://ipc.localhost/set_titlebar_color';

// 使用真实浏览器 CSP 检查；只截获合成请求，不加载应用或接触原生 Vault。
async function mountCspFixture(page: import('@playwright/test').Page, csp: string) {
  const requests: string[] = [];
  await page.route('**/*', async (route) => {
    if (route.request().url() === fixtureUrl) {
      await route.fulfill({
        contentType: 'text/html',
        headers: { 'Content-Security-Policy': csp },
        body: '<!doctype html><title>CSP IPC fixture</title>',
      });
      return;
    }
    requests.push(route.request().url());
    await route.fulfill({
      contentType: 'application/json',
      headers: {
        'Access-Control-Allow-Origin': 'http://127.0.0.1:1423',
        'Access-Control-Allow-Headers': '*',
      },
      body: '{"fixture":"local-ipc"}',
    });
  });
  await page.addInitScript(() => {
    const violations: { directive: string; blockedUri: string }[] = [];
    Object.defineProperty(window, '__cspFixtureViolations', { value: violations });
    document.addEventListener('securitypolicyviolation', (event) => {
      violations.push({
        directive: event.effectiveDirective,
        blockedUri: event.blockedURI,
      });
    });
  });
  await page.goto(fixtureUrl);
  return requests;
}

async function readViolations(page: import('@playwright/test').Page) {
  return page.evaluate(() => {
    const fixtureWindow = window as unknown as {
      __cspFixtureViolations: { directive: string; blockedUri: string }[];
    };
    return fixtureWindow.__cspFixtureViolations.map((event) => ({
      directive: event.directive,
      origin: new URL(event.blockedUri).origin,
    }));
  });
}
test('生产 CSP 允许 Windows 本地 IPC 请求', async ({ page }) => {
  const requests = await mountCspFixture(page, productionCsp);
  const result = await page.evaluate(async (url) => {
    try {
      const response = await fetch(url, {
        method: 'POST',
        body: '{}',
        headers: { 'Content-Type': 'application/json' },
      });
      return { ok: response.ok, value: await response.json() };
    } catch {
      return { ok: false, value: null };
    }
  }, ipcUrl);
  expect(result).toEqual({ ok: true, value: { fixture: 'local-ipc' } });
  expect(requests).toEqual([ipcUrl]);
  expect(await readViolations(page)).toEqual([]);
});

test('去除本地 IPC 来源的旧 CSP 在请求发送前阻断连接', async ({ page }) => {
  const oldCsp = productionCsp.replace(/\s+ipc:/g, '').replace(/\s+http:\/\/ipc\.localhost/g, '');
  const requests = await mountCspFixture(page, oldCsp);
  const blocked = await page.evaluate(async (url) => {
    try {
      await fetch(url);
      return false;
    } catch {
      return true;
    }
  }, ipcUrl);
  expect(blocked).toBe(true);
  expect(requests).toEqual([]);
  await expect
    .poll(() => readViolations(page))
    .toEqual([{ directive: 'connect-src', origin: new URL(ipcUrl).origin }]);
});

test('生产 CSP 仍阻断外部地址、其他 localhost 和非 IPC 端口', async ({ page }) => {
  const requests = await mountCspFixture(page, productionCsp);
  const urls = [
    'https://example.invalid/collect',
    'http://127.0.0.1:11434/collect',
    'http://other.localhost/collect',
    'http://ipc.localhost:43210/collect',
    'http://ipc.localhost.attacker.invalid/collect',
  ];
  const blocked = await page.evaluate(async (targets) => {
    return Promise.all(
      targets.map(async (url) => {
        try {
          await fetch(url);
          return false;
        } catch {
          return true;
        }
      }),
    );
  }, urls);
  expect(blocked).toEqual(urls.map(() => true));
  expect(requests).toEqual([]);
  await expect
    .poll(() => readViolations(page))
    .toEqual(urls.map((url) => ({ directive: 'connect-src', origin: new URL(url).origin })));
});

test('各端配置继承受限 CSP 和自动 nonce/hash 注入', () => {
  const connect = productionCsp
    .split(';')
    .map((directive) => directive.trim().split(/\s+/))
    .find(([name]) => name === 'connect-src');
  expect(connect?.slice(1).sort()).toEqual(["'self'", 'ipc:', 'http://ipc.localhost'].sort());
  expect(config.app.security.dangerousDisableAssetCspModification).toBeUndefined();
  for (const name of readdirSync('src-tauri').filter((name) =>
    /^tauri\..+\.conf\.json$/.test(name),
  )) {
    const overlay = JSON.parse(readFileSync('src-tauri/' + name, 'utf8'));
    expect(overlay.app?.security?.csp, name).toBeUndefined();
    expect(overlay.app?.security?.dangerousDisableAssetCspModification, name).toBeUndefined();
  }
});
