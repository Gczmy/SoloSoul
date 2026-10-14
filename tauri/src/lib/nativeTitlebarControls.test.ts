import { beforeEach, afterEach, describe, it, expect, vi } from 'vitest';
const invoke = vi.hoisted(() => vi.fn());
vi.mock('./ipcClient', () => ({ invokeCommand: invoke }));
beforeEach(() => {
  vi.resetModules();
  invoke.mockReset();
  invoke.mockResolvedValue(undefined);
});
afterEach(() => {
  document.body.innerHTML = '';
});

function header(rect: DOMRect) {
  const element = document.createElement('header');
  const button = document.createElement('button');
  button.dataset.titlebarControl = '';
  button.getBoundingClientRect = () => rect;
  element.append(button);
  document.body.append(element);
  return { element, button };
}
const tick = () => new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));

describe('native titlebar control ownership', () => {
  it('clips Windows controls to the native band and excludes controls below it', async () => {
    const { useNativeWindowStore } = await import('@/stores/nativeWindowStore');
    useNativeWindowStore.setState({
      windowsFrame: {
        enabled: true,
        headerHeight: 52,
        dpi: 96,
        maximized: false,
        captionBounds: [754, 0, 146, 30],
      },
    });
    const { observeTitlebarControls } = await import('./nativeTitlebarControls');
    const { element } = header(new DOMRect(-10, 45, 20, 40));
    const below = document.createElement('button');
    below.dataset.titlebarControl = '';
    below.getBoundingClientRect = () => new DOMRect(10, 80, 32, 32);
    element.append(below);
    const stop = observeTitlebarControls(element);
    await vi.waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('set_titlebar_controls', {
        regions: [{ x: 0, y: 45, width: 10, height: 7 }],
      }),
    );
    stop();
  });
  it('returning to the last successful geometry still supersedes an in-flight update', async () => {
    const { observeTitlebarControls } = await import('./nativeTitlebarControls');
    const { element, button } = header(new DOMRect(10, 10, 32, 32));
    const stop = observeTitlebarControls(element);
    await vi.waitFor(() => expect(invoke).toHaveBeenCalledTimes(1));
    let release!: () => void;
    invoke.mockImplementationOnce(
      () =>
        new Promise<void>((resolve) => {
          release = resolve;
        }),
    );
    button.getBoundingClientRect = () => new DOMRect(50, 10, 32, 32);
    window.dispatchEvent(new Event('resize'));
    await tick();
    await vi.waitFor(() => expect(invoke).toHaveBeenCalledTimes(2));
    button.getBoundingClientRect = () => new DOMRect(10, 10, 32, 32);
    window.dispatchEvent(new Event('resize'));
    await tick();
    release();
    await vi.waitFor(() =>
      expect(invoke).toHaveBeenLastCalledWith('set_titlebar_controls', {
        regions: [{ x: 10, y: 10, width: 32, height: 32 }],
      }),
    );
    expect(invoke).toHaveBeenCalledTimes(3);
    stop();
  });
  it('a failed registration is retried and closing a preview restores the mounted toolbar', async () => {
    const { observeTitlebarControls } = await import('./nativeTitlebarControls');
    const app = header(new DOMRect(10, 10, 32, 32));
    invoke.mockRejectedValueOnce(new Error('UI thread unavailable'));
    const stopApp = observeTitlebarControls(app.element);
    await vi.waitFor(() => expect(invoke).toHaveBeenCalledTimes(1));
    window.dispatchEvent(new Event('resize'));
    await tick();
    await vi.waitFor(() => expect(invoke).toHaveBeenCalledTimes(2));
    const preview = header(new DOMRect(100, 10, 32, 32));
    const stopPreview = observeTitlebarControls(preview.element, { priority: 1 });
    await vi.waitFor(() => expect(invoke.mock.lastCall?.[1].regions[0].x).toBe(100));
    stopPreview();
    await vi.waitFor(() => expect(invoke.mock.lastCall?.[1].regions[0].x).toBe(10));
    stopApp();
  });
});
