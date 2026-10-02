import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { ThemeConfig } from '@/types';
import { getSchemeById } from './themeSchemes';

const { invoke, syncNativeAppearance, platform, listen } = vi.hoisted(() => ({
  invoke: vi.fn(),
  syncNativeAppearance: vi.fn(),
  platform: vi.fn(),
  listen: vi.fn(),
}));

vi.mock('./ipcClient', () => ({ invokeCommand: invoke }));
vi.mock('./nativeWindow', () => ({ syncNativeAppearance }));
vi.mock('@tauri-apps/plugin-os', () => ({ platform }));
vi.mock('@tauri-apps/api/event', () => ({ listen }));

const SYSTEM_QUERY = '(prefers-color-scheme: dark)';
const LIGHT_SCHEME = 'clean-slate';
const DARK_SCHEME = 'obsidian-black';

let applyTheme: typeof import('./theme').applyTheme;
let rootAttributes: Array<[string, string]>;

function config(overrides: Partial<ThemeConfig> = {}): ThemeConfig {
  return {
    preset: 'system',
    accentColor: 'ocean',
    backgroundType: 'solid',
    backgroundValue: '',
    defaultLightTheme: LIGHT_SCHEME,
    defaultDarkTheme: DARK_SCHEME,
    ...overrides,
  };
}

function mediaTheme(mode: 'light' | 'dark') {
  vi.mocked(window.matchMedia).mockImplementation(
    (query) =>
      ({
        matches: mode === 'dark',
        media: query,
        onchange: null,
        addListener: vi.fn(),
        removeListener: vi.fn(),
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
        dispatchEvent: vi.fn(),
      }) as MediaQueryList,
  );
}

function systemTheme(response: () => Promise<string>) {
  invoke.mockImplementation((command: string) => {
    if (command === 'get_system_theme') return response();
    if (command === 'set_status_bar_style') return Promise.resolve();
    return Promise.reject(new Error('Unexpected theme command'));
  });
}

function systemCalls() {
  return invoke.mock.calls.filter(([command]) => command === 'get_system_theme');
}

function expectStatus(mode: 'light' | 'dark') {
  expect
    .soft(invoke.mock.calls.filter(([command]) => command === 'set_status_bar_style'))
    .toEqual([['set_status_bar_style', { payload: { style: mode } }]]);
}

function expectDesktopMode(mode: 'light' | 'dark') {
  const root = document.documentElement;
  const scheme = getSchemeById(mode === 'dark' ? DARK_SCHEME : LIGHT_SCHEME)!;
  const background = scheme.variables['--bg-base'];
  const rgb = [1, 3, 5].map((offset) => Number.parseInt(background.slice(offset, offset + 2), 16));
  expect.soft(root.dataset.theme).toBe(mode);
  expect.soft(root.style.getPropertyValue('--bg-base')).toBe(background);
  expect.soft(syncNativeAppearance).toHaveBeenCalledExactlyOnceWith({
    red: rgb[0],
    green: rgb[1],
    blue: rgb[2],
  });
  expectStatus(mode);
}

beforeEach(async () => {
  rootAttributes = Array.from(document.documentElement.attributes, ({ name, value }) => [
    name,
    value,
  ]);
  for (const name of ['style', 'data-theme', 'data-accent', 'data-platform']) {
    document.documentElement.removeAttribute(name);
  }
  vi.resetModules();
  invoke.mockReset();
  syncNativeAppearance.mockReset().mockResolvedValue(undefined);
  platform.mockReset().mockReturnValue('windows');
  listen.mockReset().mockResolvedValue(vi.fn());
  vi.spyOn(window, 'matchMedia').mockClear();
  mediaTheme('light');
  systemTheme(() => Promise.resolve('dark'));
  // 平台识别、色板、强调色和 Android 材质均运行真实实现；只替换平台 API 边界。
  await (await import('./platform')).initPlatform();
  applyTheme = (await import('./theme')).applyTheme;
});

describe('RF201 移动端真实系统来源与单一事件源', () => {
  async function mobileTheme(os: 'android' | 'ios') {
    vi.resetModules();
    platform.mockReturnValue(os);
    await (await import('./platform')).initPlatform();
    return import('./theme');
  }

  it.each([
    ['android', 'light'],
    ['android', 'dark'],
    ['ios', 'light'],
    ['ios', 'dark'],
  ] as const)('%s 初始 %s 使用 WebView，忽略相反的 IPC 值', async (os, mode) => {
    const theme = await mobileTheme(os);
    mediaTheme(mode);
    systemTheme(() => Promise.resolve(mode === 'dark' ? 'light' : 'dark'));
    expect(await theme.getSystemTheme()).toBe(mode);
    expect(systemCalls()).toHaveLength(0);
  });

  it.each(['android', 'ios'] as const)('%s 切色只注册 media 监听且释放同一回调', async (os) => {
    const theme = await mobileTheme(os);
    const callbacks = new Set<(event: MediaQueryListEvent) => void>();
    const mq = {
      matches: false,
      media: SYSTEM_QUERY,
      addEventListener: vi.fn((_: string, callback: (e: MediaQueryListEvent) => void) => {
        callbacks.add(callback);
      }),
      removeEventListener: vi.fn((_: string, callback: (e: MediaQueryListEvent) => void) => {
        callbacks.delete(callback);
      }),
    } as unknown as MediaQueryList;
    vi.mocked(window.matchMedia).mockReturnValue(mq);
    const changed = vi.fn();
    const dispose = await theme.listenForSystemTheme(changed);
    expect(listen).not.toHaveBeenCalled();
    expect(callbacks.size).toBe(1);
    for (const matches of [true, false]) {
      for (const callback of callbacks) callback({ matches } as MediaQueryListEvent);
    }
    expect(changed.mock.calls).toEqual([['dark'], ['light']]);
    dispose();
    expect(callbacks.size).toBe(0);
    expect(mq.removeEventListener).toHaveBeenCalledWith(
      'change',
      vi.mocked(mq.addEventListener).mock.calls[0][1],
    );
    expect(systemCalls()).toHaveLength(0);
  });

  it('桌面仍由原生事件驱动，注册失败才回退 media', async () => {
    const theme = await import('./theme');
    const nativeDispose = vi.fn();
    listen.mockResolvedValueOnce(nativeDispose);
    const changed = vi.fn();
    const dispose = await theme.listenForSystemTheme(changed);
    expect(listen).toHaveBeenCalledWith('system-theme-changed', expect.any(Function));
    expect(window.matchMedia).not.toHaveBeenCalled();
    const callback = listen.mock.calls[0][1];
    callback({ payload: 'dark' });
    expect(changed).toHaveBeenCalledWith('dark');
    dispose();
    expect(nativeDispose).toHaveBeenCalledOnce();
    listen.mockRejectedValueOnce(new Error('IPC unavailable'));
    await theme.listenForSystemTheme(changed);
    expect(window.matchMedia).toHaveBeenCalledWith(SYSTEM_QUERY);
  });
});

afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
  for (const attribute of Array.from(document.documentElement.attributes)) {
    document.documentElement.removeAttribute(attribute.name);
  }
  for (const [name, value] of rootAttributes) {
    document.documentElement.setAttribute(name, value);
  }
});

describe('RF110 applyTheme 单次解析与各外观输出一致性', () => {
  it.each([
    ['dark', 'light'],
    ['light', 'dark'],
  ] as const)('IPC=%s、WebView=%s 时所有桌面输出使用 IPC 结果', async (backend, webview) => {
    mediaTheme(webview);
    systemTheme(() => Promise.resolve(backend));
    const input = Object.freeze(config());

    await applyTheme(input);

    expectDesktopMode(backend);
    expect.soft(systemCalls()).toHaveLength(1);
    expect.soft(window.matchMedia).not.toHaveBeenCalled();
    expect(input.resolvedSystemTheme).toBeUndefined();
  });

  it.each(['light', 'dark'] as const)(
    '调用方已提供 resolvedSystemTheme=%s 时不再查询任何系统来源',
    async (resolved) => {
      mediaTheme(resolved === 'dark' ? 'light' : 'dark');
      systemTheme(() => Promise.resolve(resolved === 'dark' ? 'light' : 'dark'));

      await applyTheme(config({ resolvedSystemTheme: resolved }));

      expectDesktopMode(resolved);
      expect(systemCalls()).toHaveLength(0);
      expect(window.matchMedia).not.toHaveBeenCalled();
    },
  );

  it.each([
    ['warm-stone-light', 'light'],
    ['warm-stone-dark', 'dark'],
  ] as const)('显式 %s 优先于相反的系统提示并使用 %s 色板', async (preset, mode) => {
    const opposite = mode === 'dark' ? 'light' : 'dark';
    mediaTheme(opposite);
    systemTheme(() => Promise.resolve(opposite));

    await applyTheme(config({ preset, resolvedSystemTheme: opposite }));

    expectDesktopMode(mode);
    expect(systemCalls()).toHaveLength(0);
    expect(window.matchMedia).not.toHaveBeenCalled();
  });

  it.each(['light', 'dark'] as const)(
    'IPC 拒绝后仅一次读取 WebView=%s，并复用该结果',
    async (fallback) => {
      mediaTheme(fallback);
      systemTheme(() => Promise.reject(new Error('Synthetic unavailable theme IPC')));

      await applyTheme(config());

      expectDesktopMode(fallback);
      expect(systemCalls()).toHaveLength(1);
      expect(window.matchMedia).toHaveBeenCalledExactlyOnceWith(SYSTEM_QUERY);
    },
  );

  it('IPC 超时仅回退一次，迟到结果不改变已完成的外观', async () => {
    vi.useFakeTimers();
    mediaTheme('dark');
    let completeLate!: (value: string) => void;
    systemTheme(
      () =>
        new Promise<string>((resolve) => {
          completeLate = resolve;
        }),
    );

    const pending = applyTheme(config());
    await vi.advanceTimersByTimeAsync(599);
    expect(window.matchMedia).not.toHaveBeenCalled();
    expect(syncNativeAppearance).not.toHaveBeenCalled();
    expect(document.documentElement.dataset.theme).toBeUndefined();

    await vi.advanceTimersByTimeAsync(1);
    await pending;
    expectDesktopMode('dark');
    expect.soft(systemCalls()).toHaveLength(1);
    expect.soft(window.matchMedia).toHaveBeenCalledExactlyOnceWith(SYSTEM_QUERY);

    completeLate('light');
    await Promise.resolve();
    await Promise.resolve();
    expectDesktopMode('dark');
    expect(systemCalls()).toHaveLength(1);
    expect(window.matchMedia).toHaveBeenCalledExactlyOnceWith(SYSTEM_QUERY);
  });

  it('保留自定义强调色的真实 hover/前景计算，切回预设后移除内联颜色', async () => {
    await applyTheme(
      config({
        preset: 'warm-stone-dark',
        accentColor: 'custom',
        customAccentHex: '#112233',
      }),
    );

    const root = document.documentElement;
    expect(root.dataset.accent).toBe('custom');
    expect(root.style.getPropertyValue('--accent-primary')).toBe('#112233');
    expect(root.style.getPropertyValue('--accent-hover')).toBe('rgb(15, 30, 45)');
    expect(root.style.getPropertyValue('--accent-primary-text')).toBe('#ffffff');
    expect(root.style.getPropertyValue('--accent-hover-text')).toBe('#ffffff');

    invoke.mockClear();
    syncNativeAppearance.mockClear();
    await applyTheme(config({ preset: 'warm-stone-light', accentColor: 'forest' }));

    expectDesktopMode('light');
    expect(root.dataset.accent).toBe('forest');
    expect(root.style.getPropertyValue('--accent-primary')).toBe('');
    expect(root.style.getPropertyValue('--accent-hover')).toBe('');
    expect(systemCalls()).toHaveLength(0);
    expect(window.matchMedia).not.toHaveBeenCalled();
  });

  it.each([
    ['light', '#F9F9F3', '#4C6846'],
    ['dark', '#181D1B', '#B9D2A7'],
  ] as const)(
    'Android %s 保留真实 Material surface 和 forest 强调色',
    async (mode, surface, accent) => {
      // 重载真实平台缓存；不替换 applyAndroidMaterial 或主题解析算法。
      vi.resetModules();
      platform.mockReturnValue('android');
      await (await import('./platform')).initPlatform();
      const androidApplyTheme = (await import('./theme')).applyTheme;
      mediaTheme(mode === 'dark' ? 'light' : 'dark');

      await androidApplyTheme(config({ resolvedSystemTheme: mode, accentColor: 'forest' }));

      const root = document.documentElement;
      expect(root.dataset.platform).toBe('android');
      expect(root.dataset.theme).toBe(mode);
      expect(root.dataset.accent).toBe('forest');
      expect(root.style.getPropertyValue('--bg-base')).toBe(surface);
      expect(root.style.getPropertyValue('--accent-primary')).toBe(accent);
      expect(root.style.getPropertyValue('--md-on-primary')).toBe(
        mode === 'dark' ? '#20351A' : '#FFFFFF',
      );
      expect(root.style.getPropertyValue('--shadow-card')).toBe('none');
      expectStatus(mode);
      expect(systemCalls()).toHaveLength(0);
      expect(window.matchMedia).not.toHaveBeenCalled();
    },
  );
});
