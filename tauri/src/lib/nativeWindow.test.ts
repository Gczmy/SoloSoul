import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const invoke = vi.hoisted(() => vi.fn());
const nativeEvents = vi.hoisted(() => ({
  listen: vi.fn(),
  unlisten: vi.fn(),
  layoutChanged: undefined as (() => void) | undefined,
}));
vi.mock('./ipcClient', () => ({ invokeCommand: invoke }));
vi.mock('@tauri-apps/api/event', () => ({ listen: nativeEvents.listen }));

beforeEach(() => {
  vi.resetModules();
  invoke.mockReset();
  nativeEvents.listen.mockReset();
  nativeEvents.unlisten.mockReset();
  nativeEvents.layoutChanged = undefined;
  nativeEvents.listen.mockImplementation(async (event: string, callback: () => void) => {
    if (event === 'native-window-layout-changed') nativeEvents.layoutChanged = callback;
    return nativeEvents.unlisten;
  });
  // 模拟旧版本留下的兼容模式选择，验证启动已不再读取它。
  vi.stubGlobal('localStorage', { getItem: vi.fn(() => 'true') });
  document.documentElement.style.setProperty('--startup-background', '#1c1c1e');
  delete document.documentElement.dataset.nativeMaterial;
  delete document.documentElement.dataset.desktopPlatform;
  invoke.mockImplementation(async (command: string) => {
    if (command === 'set_titlebar_color' || command === 'get_window_layout') {
      return {
        material: 'liquid-glass',
        platform: 'macos',
        reduceMotion: false,
        highContrast: false,
        titlebarHeight: 52,
        trafficLightsRight: 79,
      };
    }
  });
});

afterEach(() => {
  vi.unstubAllGlobals();
  document.documentElement.style.removeProperty('--startup-background');
  document.documentElement.style.removeProperty('--native-titlebar-height');
});

describe('原生窗口启动', () => {
  it('RF-1193 原生首帧先完成系统主题读取，再显示窗口', async () => {
    Reflect.set(window, '__TAURI_OS_PLUGIN_INTERNALS__', { platform: 'macos' });
    const originalStartup = window.__SOLOSOUL_STARTUP__;
    Reflect.set(window, '__SOLOSOUL_STARTUP__', {
      systemTheme: (mode: string) => {
        document.documentElement.style.setProperty(
          '--startup-background',
          mode === 'dark' ? '#212121' : '#ffffff',
        );
        return true;
      },
    });
    let complete!: (mode: string) => void;
    invoke.mockImplementation((command: string) =>
      command === 'get_system_theme'
        ? new Promise<string>((resolve) => {
            complete = resolve;
          })
        : Promise.resolve({ material: 'solid', platform: 'macos' }),
    );
    try {
      const { prepareStartupWindow } = await import('./nativeWindow');
      const pending = prepareStartupWindow();
      await vi.waitFor(() => expect(invoke).toHaveBeenCalledWith('get_system_theme'));
      expect(invoke.mock.calls.some(([command]) => command === 'set_titlebar_color')).toBe(false);
      complete('dark');
      await pending;
      expect(invoke).toHaveBeenCalledWith('set_titlebar_color', {
        color: { red: 33, green: 33, blue: 33 },
      });
    } finally {
      window.dispatchEvent(new Event('solosoul:theme-applied'));
      Reflect.set(window, '__SOLOSOUL_STARTUP__', originalStartup);
      Reflect.deleteProperty(window, '__TAURI_OS_PLUGIN_INTERNALS__');
    }
  });

  it('RF-1193 辅助功能刷新及迟到启动请求保留最近解析的原生颜色', async () => {
    const { syncNativeAppearance, refreshNativeAppearance } = await import('./nativeWindow');
    const color = { red: 24, green: 24, blue: 24 };
    await syncNativeAppearance(color);
    document.documentElement.style.setProperty('--bg-base', '#212121');
    document.documentElement.style.setProperty('--startup-background', '#ffffff');
    await refreshNativeAppearance();
    expect(invoke).toHaveBeenLastCalledWith('set_titlebar_color', { color });
    await refreshNativeAppearance(true);
    expect(invoke).toHaveBeenLastCalledWith('set_titlebar_color', { color });
  });

  it('接受 Windows Mica，并在系统关闭透明效果后切回实色', async () => {
    const { syncNativeAppearance } = await import('./nativeWindow');
    for (const material of ['mica', 'solid']) {
      invoke.mockResolvedValueOnce({
        material,
        platform: 'windows',
        reduceMotion: false,
        highContrast: material === 'solid',
      });
      await syncNativeAppearance({ red: 42, green: 38, blue: 32 });
      expect(document.documentElement.dataset.nativeMaterial).toBe(material);
      expect(document.documentElement.dataset.highContrast).toBe(String(material === 'solid'));
      expect(document.documentElement.style.getPropertyValue('--native-titlebar-height')).toBe(
        '0px',
      );
    }
  });

  it('首次显示前同步玻璃外观，忽略旧版本的不透明偏好', async () => {
    const { prepareStartupWindow } = await import('./nativeWindow');
    await prepareStartupWindow();
    expect(invoke.mock.calls[0]).toEqual([
      'set_titlebar_color',
      { color: { red: 28, green: 28, blue: 30 } },
    ]);
    expect(invoke.mock.calls.at(-1)?.[0]).toBe('show_main_window');
    expect(localStorage.getItem).not.toHaveBeenCalled();
    expect(document.documentElement.dataset.nativeMaterial).toBe('liquid-glass');
    expect(document.documentElement.style.getPropertyValue('--native-titlebar-height')).toBe(
      '52px',
    );
    const { useNativeWindowStore } = await import('@/stores/nativeWindowStore');
    expect(useNativeWindowStore.getState().titlebarHeight).toBe(52);
    expect(useNativeWindowStore.getState().trafficLightsRight).toBe(79);
    expect(useNativeWindowStore.getState().isMacOS).toBe(true);
  });

  it('继续尊重原生系统辅助功能返回的材质与对比度', async () => {
    invoke.mockResolvedValueOnce({
      material: 'solid',
      platform: 'macos',
      reduceMotion: true,
      highContrast: true,
    });
    const { syncNativeAppearance } = await import('./nativeWindow');
    await syncNativeAppearance({ red: 250, green: 250, blue: 248 });
    expect(document.documentElement.dataset).toMatchObject({
      nativeMaterial: 'solid',
      highContrast: 'true',
      reduceMotion: 'false',
      systemReduceMotion: 'true',
      motionPolicy: 'application',
    });
  });

  it('全屏后重新读取避让区，并在取消监听后停止同步', async () => {
    const { syncNativeAppearance, observeNativeWindowLayout } = await import('./nativeWindow');
    const { useNativeWindowStore } = await import('@/stores/nativeWindowStore');
    await syncNativeAppearance({ red: 28, green: 28, blue: 30 });
    const stop = observeNativeWindowLayout();
    invoke.mockResolvedValue({ material: 'liquid-glass', platform: 'macos', titlebarHeight: 0 });
    window.dispatchEvent(new Event('resize'));
    await vi.waitFor(() => expect(useNativeWindowStore.getState().titlebarHeight).toBe(0));
    expect(invoke.mock.calls.filter(([command]) => command === 'set_titlebar_color')).toHaveLength(
      1,
    );
    expect(invoke).toHaveBeenCalledWith('get_window_layout');
    stop();
    invoke.mockClear();
    window.dispatchEvent(new Event('resize'));
    await new Promise((resolve) => requestAnimationFrame(resolve));
    expect(invoke).not.toHaveBeenCalled();
  });

  it('连续切换外观串行执行，旧响应不覆盖当前主题', async () => {
    const { syncNativeAppearance } = await import('./nativeWindow');
    let finishOld!: (value: unknown) => void;
    invoke.mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          finishOld = resolve;
        }),
    );
    const old = syncNativeAppearance({ red: 250, green: 250, blue: 248 });
    await vi.waitFor(() => expect(invoke).toHaveBeenCalledTimes(1));
    invoke.mockResolvedValueOnce({ material: 'solid', platform: 'macos', titlebarHeight: 52 });
    const latest = syncNativeAppearance({ red: 28, green: 28, blue: 30 });
    expect(invoke).toHaveBeenCalledTimes(1);
    finishOld({ material: 'vibrancy', platform: 'macos', titlebarHeight: 28 });
    await Promise.all([old, latest]);
    expect(invoke).toHaveBeenLastCalledWith('set_titlebar_color', {
      color: { red: 28, green: 28, blue: 30 },
    });
    expect(document.documentElement.dataset.nativeMaterial).toBe('solid');
    expect(document.documentElement.style.getPropertyValue('--native-titlebar-height')).toBe(
      '52px',
    );
  });

  it('退出全屏的resize先返回32pt，原生收尾后无新resize也恢复52pt', async () => {
    const { syncNativeAppearance, observeNativeWindowLayout } = await import('./nativeWindow');
    const { useNativeWindowStore } = await import('@/stores/nativeWindowStore');
    await syncNativeAppearance({ red: 28, green: 28, blue: 30 });
    invoke.mockResolvedValue({ platform: 'macos', titlebarHeight: 32, trafficLightsRight: 79 });
    const stop = observeNativeWindowLayout();
    window.dispatchEvent(new Event('resize'));
    await vi.waitFor(() => expect(useNativeWindowStore.getState().titlebarHeight).toBe(32));
    invoke.mockResolvedValue({ platform: 'macos', titlebarHeight: 52, trafficLightsRight: 79 });
    nativeEvents.layoutChanged?.();
    await vi.waitFor(() => expect(useNativeWindowStore.getState().titlebarHeight).toBe(52));
    expect(document.documentElement.style.getPropertyValue('--native-titlebar-height')).toBe(
      '52px',
    );
    expect(invoke.mock.calls.filter(([command]) => command === 'set_titlebar_color')).toHaveLength(
      1,
    );
    stop();
    expect(nativeEvents.unlisten).toHaveBeenCalledTimes(1);
    invoke.mockClear();
    nativeEvents.layoutChanged?.();
    await new Promise((resolve) => requestAnimationFrame(resolve));
    expect(invoke).not.toHaveBeenCalled();
  });

  it('原生收尾通知早于未完成的临时几何响应时，仍会重新测量最终栏高', async () => {
    const { syncNativeAppearance, observeNativeWindowLayout } = await import('./nativeWindow');
    const { useNativeWindowStore } = await import('@/stores/nativeWindowStore');
    await syncNativeAppearance({ red: 28, green: 28, blue: 30 });
    let finishLayout!: (value: unknown) => void;
    invoke.mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          finishLayout = resolve;
        }),
    );
    const stop = observeNativeWindowLayout();
    await vi.waitFor(() => expect(invoke).toHaveBeenLastCalledWith('get_window_layout'));
    invoke.mockResolvedValue({ platform: 'macos', titlebarHeight: 52, trafficLightsRight: 79 });
    nativeEvents.layoutChanged?.();
    finishLayout({ platform: 'macos', titlebarHeight: 32, trafficLightsRight: 79 });
    await vi.waitFor(() =>
      expect(invoke.mock.calls.filter(([command]) => command === 'get_window_layout')).toHaveLength(
        2,
      ),
    );
    await vi.waitFor(() => expect(useNativeWindowStore.getState().titlebarHeight).toBe(52));
    stop();
  });

  it('恢复窗口只读取几何，卸载后的迟到测量不回写', async () => {
    const { syncNativeAppearance, observeNativeWindowLayout } = await import('./nativeWindow');
    await syncNativeAppearance({ red: 28, green: 28, blue: 30 });
    let finishLayout!: (value: unknown) => void;
    invoke.mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          finishLayout = resolve;
        }),
    );
    const stop = observeNativeWindowLayout();
    window.dispatchEvent(new Event('focus'));
    await vi.waitFor(() => expect(invoke).toHaveBeenLastCalledWith('get_window_layout'));
    stop();
    finishLayout({ platform: 'macos', titlebarHeight: 0, trafficLightsRight: 0 });
    await new Promise((resolve) => requestAnimationFrame(resolve));
    expect(document.documentElement.style.getPropertyValue('--native-titlebar-height')).toBe(
      '52px',
    );
    expect(invoke.mock.calls.filter(([command]) => command === 'set_titlebar_color')).toHaveLength(
      1,
    );
  });

  it.each([
    { platform: 'windows', titlebarHeight: 52 },
    { platform: 'macos', titlebarHeight: -12 },
    { platform: 'macos', titlebarHeight: Number.NaN },
    { platform: 'macos' },
  ])('非 macOS 或无效测量不保留旧避让区：%j', async (measurement) => {
    const { syncNativeAppearance } = await import('./nativeWindow');
    await syncNativeAppearance({ red: 28, green: 28, blue: 30 });
    invoke.mockResolvedValueOnce({ material: 'solid', ...measurement });
    await syncNativeAppearance({ red: 28, green: 28, blue: 30 });
    expect(document.documentElement.style.getPropertyValue('--native-titlebar-height')).toBe('0px');
  });
});
