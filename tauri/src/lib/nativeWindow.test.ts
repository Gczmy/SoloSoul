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
  invoke.mockImplementation(async (command: string) => {
    if (command === 'set_titlebar_color') {
      return {
        material: 'liquid-glass',
        platform: 'macos',
        reduceMotion: false,
        highContrast: false,
      };
    }
  });
});

afterEach(() => {
  vi.unstubAllGlobals();
  document.documentElement.style.removeProperty('--startup-background');
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
});
