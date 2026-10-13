import { beforeEach, afterEach, describe, it, expect, vi } from 'vitest';

const ipc = vi.hoisted(() => ({ invoke: vi.fn(), enabled: false }));
vi.mock('./ipcClient', () => ({ invokeCommand: ipc.invoke }));
vi.mock('@tauri-apps/api/event', () => ({ listen: async () => () => {} }));

beforeEach(() => {
  vi.resetModules();
  ipc.enabled = false;
  ipc.invoke.mockReset();
  vi.stubGlobal('innerWidth', 900);
  document.documentElement.style.setProperty('--startup-background', '#1c1c1e');
  delete document.documentElement.dataset.windowsFrame;
  ipc.invoke.mockImplementation(async (command, args) => {
    if (command === 'set_titlebar_controls' && args.windowsFrame !== undefined)
      ipc.enabled = args.windowsFrame;
    if (command === 'set_titlebar_color')
      return { platform: 'windows', material: 'mica', reduceMotion: false, highContrast: false };
    if (command === 'get_window_layout')
      return {
        platform: 'windows',
        titlebarHeight: ipc.enabled ? 52 : 0,
        windowsFrame: ipc.enabled
          ? {
              enabled: true,
              headerHeight: 52,
              dpi: 96,
              maximized: false,
              captionBounds: [754, 0, 146, 30],
            }
          : undefined,
      };
  });
});

afterEach(() => {
  vi.unstubAllGlobals();
  document.documentElement.style.removeProperty('--startup-background');
});

describe('Windows native frame frontend lifecycle', () => {
  it('startup explicitly enables the frame and reserves the actual native buttons', async () => {
    const { prepareStartupWindow } = await import('./nativeWindow');
    await prepareStartupWindow();
    expect(ipc.invoke).toHaveBeenCalledWith('set_titlebar_controls', {
      regions: [],
      windowsFrame: true,
    });
    expect(document.documentElement.dataset.windowsFrame).toBe('true');
    expect(document.documentElement.style.getPropertyValue('--native-caption-inset')).toBe('146px');
    expect(document.documentElement.style.getPropertyValue('--native-titlebar-height')).toBe(
      '52px',
    );
    const calls = ipc.invoke.mock.calls.map(([command]) => command);
    expect(calls.lastIndexOf('set_titlebar_color')).toBeGreaterThan(
      calls.indexOf('set_titlebar_controls'),
    );
    expect(calls.indexOf('show_main_window')).toBeGreaterThan(calls.indexOf('get_window_layout'));
  });

  it('a theme response without frame geometry preserves the installed frame', async () => {
    const { prepareStartupWindow, syncNativeAppearance } = await import('./nativeWindow');
    await prepareStartupWindow();
    await syncNativeAppearance({ red: 250, green: 250, blue: 250 });
    expect(document.documentElement.dataset.windowsFrame).toBe('true');
    expect(document.documentElement.style.getPropertyValue('--native-caption-inset')).toBe('146px');
    expect(document.documentElement.style.getPropertyValue('--native-titlebar-height')).toBe(
      '52px',
    );
  });

  it('resize reads updated DPI/fullscreen geometry without reapplying material', async () => {
    const { prepareStartupWindow, observeNativeWindowLayout } = await import('./nativeWindow');
    await prepareStartupWindow();
    const stop = observeNativeWindowLayout();
    await vi.waitFor(() => expect(ipc.invoke).toHaveBeenLastCalledWith('get_window_layout'));
    ipc.invoke.mockClear();
    ipc.invoke.mockResolvedValue({
      platform: 'windows',
      windowsFrame: {
        enabled: true,
        headerHeight: 0,
        dpi: 144,
        maximized: false,
        captionBounds: [0, 0, 0, 0],
      },
    });
    window.dispatchEvent(new Event('resize'));
    await vi.waitFor(() =>
      expect(document.documentElement.style.getPropertyValue('--native-caption-inset')).toBe('0px'),
    );
    expect(document.documentElement.dataset.windowsFrame).toBe('true');
    expect(ipc.invoke.mock.calls.every(([command]) => command === 'get_window_layout')).toBe(true);
    stop();
  });

  it('an unsupported native frame restores the legacy caption and still shows the window', async () => {
    const original = ipc.invoke.getMockImplementation()!;
    ipc.invoke.mockImplementation(async (command, args) => {
      if (command === 'set_titlebar_controls' && args.windowsFrame === true)
        throw new Error('Unsupported Wry host');
      return original(command, args);
    });
    const { prepareStartupWindow } = await import('./nativeWindow');
    await prepareStartupWindow();
    expect(ipc.invoke).toHaveBeenCalledWith('set_titlebar_controls', {
      regions: [],
      windowsFrame: false,
    });
    expect(document.documentElement.dataset.windowsFrame).toBe('false');
    expect(ipc.invoke).toHaveBeenCalledWith('show_main_window');
  });
});
