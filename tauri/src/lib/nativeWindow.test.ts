import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const invoke = vi.hoisted(() => vi.fn());
vi.mock('./ipcClient', () => ({ invokeCommand: invoke }));

beforeEach(() => {
  vi.resetModules();
  invoke.mockReset();
  // 模拟旧版本留下的兼容模式选择，验证启动已不再读取它。
  vi.stubGlobal('localStorage', { getItem: vi.fn(() => 'true') });
  document.documentElement.style.setProperty('--startup-background', '#1c1c1e');
  delete document.documentElement.dataset.nativeMaterial;
  delete document.documentElement.dataset.desktopPlatform;
  invoke.mockImplementation(async (command: string) => {
    if (command === 'set_titlebar_color') {
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
      reduceMotion: 'true',
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
    stop();
    invoke.mockClear();
    window.dispatchEvent(new Event('resize'));
    await new Promise((resolve) => requestAnimationFrame(resolve));
    expect(invoke).not.toHaveBeenCalled();
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
