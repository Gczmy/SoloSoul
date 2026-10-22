import { beforeEach, afterEach, describe, it, expect, vi } from 'vitest';
import { resizeObserverInstances } from '@/test/setup';
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
  it('updates moved controls without restarting size observation for position-only styles', async () => {
    const { observeTitlebarControls } = await import('./nativeTitlebarControls');
    const { element, button } = header(new DOMRect(10, 10, 32, 32));
    const stop = observeTitlebarControls(element);
    const sizes = resizeObserverInstances.at(-1)!;
    try {
      await vi.waitFor(() => expect(invoke).toHaveBeenCalled());
      const observations = sizes.observe.mock.calls.length;
      const disconnects = sizes.disconnect.mock.calls.length;
      button.getBoundingClientRect = () => new DOMRect(110, 10, 32, 32);
      element.style.setProperty('--editor-actions-right', '150px');
      await vi.waitFor(() =>
        expect(invoke.mock.lastCall?.[1].regions).toEqual([
          { x: 110, y: 10, width: 32, height: 32 },
        ]),
      );
      expect(sizes.observe).toHaveBeenCalledTimes(observations);
      expect(sizes.disconnect).toHaveBeenCalledTimes(disconnects);
      const extra = document.createElement('button');
      extra.dataset.titlebarControl = '';
      extra.getBoundingClientRect = () => new DOMRect(150, 10, 32, 32);
      element.append(extra);
      await vi.waitFor(() => expect(invoke.mock.lastCall?.[1].regions).toHaveLength(2));
      expect(sizes.observe.mock.calls.length).toBeGreaterThan(observations);
    } finally {
      stop();
    }
  });
  it('moves a nested custom mask into the same native surface and restores its inline style on removal', async () => {
    const { useNativeWindowStore } = await import('@/stores/nativeWindowStore');
    useNativeWindowStore.setState({
      windowsFrame: {
        enabled: true,
        fullWindowBackdrop: true,
        headerHeight: 52,
        dpi: 96,
        maximized: false,
        captionBounds: [754, 0, 146, 30],
      },
    });
    const { observeNativeCaptionBackdrops } = await import('./nativeTitlebarControls');
    const outer = document.createElement('div');
    outer.dataset.macosGlassBackdrop = '';
    outer.dataset.nativeWindowBackdrop = 'true';
    outer.style.outlineColor = 'rgba(31, 28, 24, 0.4)';
    outer.style.backgroundColor = 'transparent';
    outer.getBoundingClientRect = () => new DOMRect(0, 0, window.innerWidth, window.innerHeight);
    const inner = document.createElement('div');
    inner.dataset.macosGlassBackdrop = '';
    inner.style.backgroundColor = 'rgba(18, 16, 14, 0.65)';
    inner.getBoundingClientRect = outer.getBoundingClientRect;
    document.body.append(outer, inner);
    const stop = observeNativeCaptionBackdrops();
    try {
      await vi.waitFor(() =>
        expect(
          invoke.mock.lastCall?.[1].backdrop.layers.map(
            (layer: { fullWindow: boolean }) => layer.fullWindow,
          ),
        ).toEqual([true, true]),
      );
      expect(inner.style.backgroundColor).toBe('transparent');
      inner.remove();
      await vi.waitFor(() => expect(invoke.mock.lastCall?.[1].backdrop.layers).toHaveLength(1));
      expect(inner.style.backgroundColor).toBe('rgba(18, 16, 14, 0.65)');
    } finally {
      stop();
    }
  });
  it('uses one full-window surface with card cutouts and releases it before the close acknowledgement', async () => {
    const { useNativeWindowStore } = await import('@/stores/nativeWindowStore');
    useNativeWindowStore.setState({
      windowsFrame: {
        enabled: true,
        fullWindowBackdrop: true,
        headerHeight: 52,
        dpi: 96,
        maximized: false,
        captionBounds: [754, 0, 146, 30],
      },
    });
    const { observeNativeCaptionBackdrops, releaseNativeWindowBackdrop } =
      await import('./nativeTitlebarControls');
    const mask = document.createElement('div');
    mask.dataset.macosGlassBackdrop = '';
    mask.dataset.nativeWindowBackdrop = 'true';
    mask.style.outlineColor = 'rgba(31, 28, 24, 0.4)';
    mask.style.backgroundColor = 'transparent';
    mask.getBoundingClientRect = () => new DOMRect(0, 0, window.innerWidth, window.innerHeight);
    const panel = document.createElement('div');
    panel.dataset.nativeBackdropPanel = '';
    panel.style.borderTopLeftRadius = '16px';
    panel.getBoundingClientRect = () => new DOMRect(100, 90, 360, 400);
    mask.append(panel);
    document.body.append(mask);
    const stop = observeNativeCaptionBackdrops();
    await vi.waitFor(() =>
      expect(invoke.mock.lastCall?.[1].backdrop.layers).toEqual([
        {
          red: 31,
          green: 28,
          blue: 24,
          alpha: 102,
          fullWindow: true,
          exclusions: [{ x: 100, y: 90, width: 360, height: 400, radius: 16 }],
        },
      ]),
    );
    let release!: () => void;
    invoke.mockImplementationOnce(
      () =>
        new Promise<void>((resolve) => {
          release = resolve;
        }),
    );
    let closed = false;
    const closing = releaseNativeWindowBackdrop(mask).then(() => {
      closed = true;
    });
    await vi.waitFor(() => expect(invoke.mock.lastCall?.[1].backdrop.alpha).toBe(0));
    expect(closed).toBe(false);
    expect(panel.isConnected).toBe(true);
    release();
    await closing;
    expect(closed).toBe(true);
    stop();
  });
  it('starts a mask update in the mutation turn without waiting for an animation frame', async () => {
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
    const { observeNativeCaptionBackdrops } = await import('./nativeTitlebarControls');
    const raf = vi.spyOn(window, 'requestAnimationFrame');
    const stop = observeNativeCaptionBackdrops();
    try {
      const mask = document.createElement('div');
      mask.dataset.macosGlassBackdrop = '';
      mask.style.backgroundColor = 'rgba(31, 28, 24, 0.4)';
      mask.getBoundingClientRect = () => new DOMRect(0, 0, window.innerWidth, window.innerHeight);
      document.body.append(mask);
      await vi.waitFor(() => expect(invoke.mock.lastCall?.[1].backdrop.alpha).toBe(102));
      expect(raf).not.toHaveBeenCalled();
      mask.remove();
      await vi.waitFor(() => expect(invoke.mock.lastCall?.[1].backdrop.alpha).toBe(0));
      expect(raf).not.toHaveBeenCalled();
    } finally {
      stop();
      raf.mockRestore();
    }
  });
  it('composites nested masks over the caption, restores on close and leaves controls unchanged', async () => {
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
    const { observeTitlebarControls, observeNativeCaptionBackdrops } =
      await import('./nativeTitlebarControls');
    const { element } = header(new DOMRect(10, 10, 32, 32));
    const stopHeader = observeTitlebarControls(element);
    const stopMasks = observeNativeCaptionBackdrops();
    const mask = (color: string) => {
      const el = document.createElement('div');
      el.dataset.macosGlassBackdrop = '';
      el.style.backgroundColor = color;
      el.getBoundingClientRect = () => new DOMRect(0, 0, window.innerWidth, window.innerHeight);
      document.body.append(el);
      return el;
    };
    const first = mask('rgba(31, 28, 24, 0.4)');
    await vi.waitFor(() =>
      expect(invoke).toHaveBeenLastCalledWith('set_titlebar_controls', {
        regions: [{ x: 10, y: 10, width: 32, height: 32 }],
        backdrop: { red: 31, green: 28, blue: 24, alpha: 102 },
      }),
    );
    const second = mask('rgba(18, 16, 14, 0.65)');
    await vi.waitFor(() =>
      expect(invoke.mock.lastCall?.[1].backdrop).toEqual({
        red: 20,
        green: 18,
        blue: 16,
        alpha: 201,
      }),
    );
    second.remove();
    await vi.waitFor(() => expect(invoke.mock.lastCall?.[1].backdrop.alpha).toBe(102));
    first.remove();
    await vi.waitFor(() => expect(invoke.mock.lastCall?.[1].backdrop.alpha).toBe(0));
    expect(invoke.mock.lastCall?.[1].regions).toEqual([{ x: 10, y: 10, width: 32, height: 32 }]);
    stopMasks();
    stopHeader();
  });

  it('closing a mask supersedes its pending native request and hidden/partial masks cannot shade the caption', async () => {
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
    const { observeNativeCaptionBackdrops } = await import('./nativeTitlebarControls');
    const stop = observeNativeCaptionBackdrops();
    let release!: () => void;
    invoke.mockImplementationOnce(
      () =>
        new Promise<void>((resolve) => {
          release = resolve;
        }),
    );
    const mask = document.createElement('div');
    mask.dataset.macosGlassBackdrop = '';
    mask.style.backgroundColor = 'rgba(31, 28, 24, 0.4)';
    mask.getBoundingClientRect = () => new DOMRect(0, 0, window.innerWidth, window.innerHeight);
    document.body.append(mask);
    await vi.waitFor(() => expect(invoke).toHaveBeenCalledTimes(1));
    mask.remove();
    await tick();
    release();
    await vi.waitFor(() => expect(invoke.mock.lastCall?.[1].backdrop.alpha).toBe(0));
    mask.style.display = 'none';
    document.body.append(mask);
    await tick();
    expect(invoke.mock.lastCall?.[1].backdrop.alpha).toBe(0);
    mask.style.display = 'block';
    mask.getBoundingClientRect = () => new DOMRect(100, 52, 400, 400);
    await tick();
    expect(invoke.mock.lastCall?.[1].backdrop.alpha).toBe(0);
    stop();
  });
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
