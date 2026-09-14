import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ST_MACOS_WINDOW_OPAQUE } from './constants';

const invoke = vi.hoisted(() => vi.fn());
vi.mock('./ipcClient', () => ({ invokeCommand: invoke }));

beforeEach(() => {
  vi.resetModules();
  vi.restoreAllMocks();
  invoke.mockReset();
  const storage = new Map<string, string>();
  vi.stubGlobal('localStorage', {
    getItem: (key: string) => storage.get(key) ?? null,
    setItem: (key: string, value: string) => storage.set(key, value),
  });
  delete document.documentElement.dataset.nativeMaterial;
  invoke.mockImplementation(async (command: string, args?: { opaque?: boolean }) => {
    if (command === 'set_titlebar_color') {
      return {
        material: args?.opaque ? 'solid' : 'liquid-glass',
        platform: 'macos',
        reduceMotion: false,
        highContrast: false,
      };
    }
  });
});

describe('macOS 窗口透明兼容模式', () => {
  it('首次显示窗口前读取本机选择，不等待账户偏好加载', async () => {
    localStorage.setItem(ST_MACOS_WINDOW_OPAQUE, 'true');
    const { prepareStartupWindow } = await import('./nativeWindow');
    await prepareStartupWindow();
    expect(invoke.mock.calls[0]).toEqual([
      'set_titlebar_color',
      expect.objectContaining({ opaque: true }),
    ]);
    expect(invoke.mock.calls.at(-1)?.[0]).toBe('show_main_window');
    expect(document.documentElement.dataset.nativeMaterial).toBe('solid');
  });

  it('切换后主题同步沿用兼容模式，重新加载时仍保留选择', async () => {
    const { setOpaqueWindow, syncNativeAppearance } = await import('./nativeWindow');
    await setOpaqueWindow(true);
    await syncNativeAppearance({ red: 250, green: 250, blue: 248 });
    expect(invoke).toHaveBeenLastCalledWith('set_titlebar_color', {
      color: { red: 250, green: 250, blue: 248 },
      opaque: true,
    });
    expect(localStorage.getItem(ST_MACOS_WINDOW_OPAQUE)).toBe('true');
    vi.resetModules();
    const { windowAppearanceStore } = await import('@/stores/windowAppearanceStore');
    expect(windowAppearanceStore.getState().opaque).toBe(true);
  });

  it('允许恢复玻璃效果，并保存关闭兼容模式的选择', async () => {
    localStorage.setItem(ST_MACOS_WINDOW_OPAQUE, 'true');
    const { setOpaqueWindow } = await import('./nativeWindow');
    await setOpaqueWindow(false);
    expect(invoke).toHaveBeenCalledWith(
      'set_titlebar_color',
      expect.objectContaining({ opaque: false }),
    );
    expect(localStorage.getItem(ST_MACOS_WINDOW_OPAQUE)).toBe('false');
    expect(document.documentElement.dataset.nativeMaterial).toBe('liquid-glass');
  });

  it('原生应用失败时恢复选择、持久化值和原生材质', async () => {
    invoke.mockRejectedValueOnce(new Error('Native apply failed'));
    const { setOpaqueWindow } = await import('./nativeWindow');
    const { windowAppearanceStore } = await import('@/stores/windowAppearanceStore');
    await expect(setOpaqueWindow(true)).rejects.toThrow('Native apply failed');
    expect(windowAppearanceStore.getState()).toMatchObject({ opaque: false, isSaving: false });
    expect(localStorage.getItem(ST_MACOS_WINDOW_OPAQUE)).toBe('false');
    expect(invoke).toHaveBeenLastCalledWith(
      'set_titlebar_color',
      expect.objectContaining({ opaque: false }),
    );
    expect(document.documentElement.dataset.nativeMaterial).toBe('liquid-glass');
  });

  it('存储不可写时不更改原生窗口或显示成功状态', async () => {
    const { setOpaqueWindow } = await import('./nativeWindow');
    const { windowAppearanceStore } = await import('@/stores/windowAppearanceStore');
    vi.spyOn(localStorage, 'setItem').mockImplementation(() => {
      throw new Error('Storage unavailable');
    });
    await expect(setOpaqueWindow(true)).rejects.toThrow('Storage unavailable');
    expect(invoke).not.toHaveBeenCalled();
    expect(windowAppearanceStore.getState()).toMatchObject({ opaque: false, isSaving: false });
  });

  it.each(['false', 'invalid', '1'])('缓存值 %s 不会意外禁用玻璃效果', async (value) => {
    localStorage.setItem(ST_MACOS_WINDOW_OPAQUE, value);
    const { syncNativeAppearance } = await import('./nativeWindow');
    await syncNativeAppearance({ red: 28, green: 28, blue: 30 });
    expect(invoke).toHaveBeenLastCalledWith(
      'set_titlebar_color',
      expect.objectContaining({ opaque: false }),
    );
  });
});
