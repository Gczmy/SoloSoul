import { invokeCommand as invoke } from './ipcClient';
import { withTimeout } from './withTimeout';
import { useNativeWindowStore, type WindowsFrameLayout } from '@/stores/nativeWindowStore';
import { listen } from '@tauri-apps/api/event';
import { trackAsyncListener } from './asyncListener';
import { observeNativeCaptionBackdrops } from './nativeTitlebarControls';
import { getPlatform } from './platform';
import { applyNativeMotionPreference, applyMotionPlatform } from './motionPolicy';

interface WindowLayout {
  platform: 'macos' | 'windows' | 'other';
  titlebarHeight?: number;
  trafficLightsRight?: number;
  windowsFrame?: WindowsFrameLayout;
}

interface WindowAppearance extends WindowLayout {
  material: 'solid' | 'vibrancy' | 'liquid-glass' | 'mica';
  reduceMotion: boolean;
  highContrast: boolean;
}

let appearanceRevision = 0;
let layoutRevision = 0;
let appearanceQueue: Promise<void> = Promise.resolve();
let resolvedColor: { red: number; green: number; blue: number } | undefined;

function validWindowsFrame(frame: WindowsFrameLayout | undefined) {
  if (
    !frame ||
    frame.enabled !== true ||
    !Number.isFinite(frame.dpi) ||
    frame.dpi <= 0 ||
    !Number.isFinite(frame.headerHeight) ||
    frame.headerHeight < 0 ||
    frame.headerHeight > 52 ||
    !Array.isArray(frame.captionBounds) ||
    frame.captionBounds.length !== 4 ||
    !frame.captionBounds.every(Number.isFinite) ||
    frame.captionBounds[2] < 0 ||
    frame.captionBounds[3] < 0 ||
    (frame.headerHeight > 0 && frame.captionBounds[2] === 0)
  )
    return null;
  return frame;
}

function applyWindowLayout(appearance: WindowLayout, appearanceOnly = false) {
  const root = document.documentElement;
  root.dataset.desktopPlatform = appearance.platform;
  // 外观响应不含窗口按钮几何，不能在切换主题时清除已安装窗框。
  const windowsFrame =
    appearance.platform === 'windows'
      ? appearanceOnly
        ? useNativeWindowStore.getState().windowsFrame
        : validWindowsFrame(appearance.windowsFrame)
      : null;
  const height =
    appearance.platform === 'macos' ? appearance.titlebarHeight : (windowsFrame?.headerHeight ?? 0);
  const titlebarHeight =
    typeof height === 'number' && Number.isFinite(height) ? Math.max(0, height) : 0;
  root.style.setProperty('--native-titlebar-height', `${titlebarHeight}px`);
  root.dataset.windowsFrame = String(Boolean(windowsFrame));
  const captionInset =
    windowsFrame && windowsFrame.headerHeight > 0
      ? Math.max(0, Math.min(window.innerWidth, window.innerWidth - windowsFrame.captionBounds[0]))
      : 0;
  root.style.setProperty('--native-caption-inset', `${captionInset}px`);
  const right = appearance.platform === 'macos' ? appearance.trafficLightsRight : 0;
  const trafficLightsRight =
    typeof right === 'number' && Number.isFinite(right) ? Math.max(0, right) : 0;
  useNativeWindowStore.setState({
    isMacOS: appearance.platform === 'macos',
    isWindows: appearance.platform === 'windows',
    titlebarHeight,
    trafficLightsRight,
    windowsFrame,
  });
}

/** 外观请求串行执行；连续切换时跳过尚未执行的旧主题，迟到响应不回写布局。 */
export function syncNativeAppearance(color: { red: number; green: number; blue: number }) {
  resolvedColor = { ...color };
  return enqueueNativeAppearance(color);
}

function enqueueNativeAppearance(color: { red: number; green: number; blue: number }) {
  const revision = ++appearanceRevision;
  const geometryRevision = ++layoutRevision;
  const next = appearanceQueue
    .catch(() => {})
    .then(async () => {
      if (revision !== appearanceRevision) return;
      const appearance = await withTimeout(
        invoke<WindowAppearance>('set_titlebar_color', { color }),
        1000,
      );
      if (
        revision !== appearanceRevision ||
        !appearance ||
        !['solid', 'vibrancy', 'liquid-glass', 'mica'].includes(appearance.material)
      )
        return;
      const root = document.documentElement;
      root.dataset.nativeMaterial = appearance.material;
      applyNativeMotionPreference(appearance.platform, appearance.reduceMotion);
      root.dataset.highContrast = String(appearance.highContrast);
      if (geometryRevision === layoutRevision) applyWindowLayout(appearance, true);
    });
  appearanceQueue = next;
  return next;
}

/** 恢复/全屏/缩放只读几何；不重设外观、背景或材质视图。 */
export function observeNativeWindowLayout() {
  const stopBackdrops = observeNativeCaptionBackdrops();
  let frame = 0;
  let stopped = false;
  let running = false;
  let dirty = false;
  const measure = async () => {
    running = true;
    dirty = false;
    await appearanceQueue.catch(() => {});
    if (stopped) return;
    const revision = ++layoutRevision;
    try {
      const layout = await withTimeout(invoke<WindowLayout>('get_window_layout'), 1000);
      if (!stopped && revision === layoutRevision && layout) applyWindowLayout(layout);
    } catch {
      // 测量失败保留最后一次有效几何，避免恢复窗口时回落到默认尺寸。
    } finally {
      running = false;
      if (!stopped && dirty) update();
    }
  };
  const update = () => {
    if (
      stopped ||
      !['macos', 'windows'].includes(document.documentElement.dataset.desktopPlatform ?? '')
    )
      return;
    if (running) {
      dirty = true;
      return;
    }
    cancelAnimationFrame(frame);
    frame = requestAnimationFrame(() => void measure());
  };
  window.addEventListener('resize', update);
  window.addEventListener('focus', update);
  // AppKit 的工具栏恢复可能晚于 resize；原生收尾后再次只读最终几何。
  const unlisten = trackAsyncListener(listen('native-window-layout-changed', update));
  update();
  return () => {
    stopped = true;
    cancelAnimationFrame(frame);
    window.removeEventListener('resize', update);
    window.removeEventListener('focus', update);
    unlisten();
    stopBackdrops();
  };
}

/** 图标解码后显示原生窗口；隐藏 WebView 不保证调度 RAF，因此设置有界绘制等待。 */
export async function prepareStartupWindow(): Promise<void> {
  const desktop = await getPlatform().then(
    (platform) => {
      applyMotionPlatform(platform);
      return platform === 'windows' || platform === 'macos';
    },
    () => false,
  );
  if (desktop && window.__SOLOSOUL_STARTUP__?.systemTheme) {
    let retired = false;
    let changes = 0;
    const update = () => {
      if (!retired) {
        changes += 1;
        void refreshNativeAppearance(true);
      }
    };
    const subscription = listen<string>('system-theme-changed', ({ payload }) => {
      if (!retired && (payload === 'light' || payload === 'dark')) {
        changes += 1;
        window.__SOLOSOUL_STARTUP__?.systemTheme?.(payload);
      }
    });
    const stop = trackAsyncListener(subscription);
    const release = () => {
      retired = true;
      stop();
      window.removeEventListener('solosoul:startup-theme-changed', update);
      window.removeEventListener('solosoul:theme-applied', release);
      window.removeEventListener('pagehide', release);
    };
    window.addEventListener('solosoul:startup-theme-changed', update);
    window.addEventListener('solosoul:theme-applied', release);
    window.addEventListener('pagehide', release);
    // 订阅后再读一次，补齐创建 WebView 到注册监听之间的系统变化。
    const initialTheme = subscription.then(
      async () => {
        if (retired) return;
        const revision = changes;
        const mode = await withTimeout(invoke<string>('get_system_theme'), 600).catch(() => null);
        if (!retired && revision === changes && (mode === 'light' || mode === 'dark'))
          window.__SOLOSOUL_STARTUP__?.systemTheme?.(mode);
      },
      () => {},
    );
    // 显示窗口前先完成有界的系统读取，避免旧缓存色先露出再切换。
    await withTimeout(initialTheme, 650).catch(() => {});
  }
  await refreshNativeAppearance(true).catch(() => {});
  if (useNativeWindowStore.getState().isWindows) await initializeWindowsFrame();
  const logo = document.querySelector<HTMLImageElement>('.startup-logo');
  if (logo) await withTimeout(logo.decode(), 400).catch(() => {});
  await new Promise<void>((resolve) => {
    const timeout = setTimeout(resolve, 100);
    requestAnimationFrame(() =>
      requestAnimationFrame(() => {
        clearTimeout(timeout);
        resolve();
      }),
    );
  });
  await invoke('show_main_window').catch(() => {});
  // show_main_window会恢复最大化/全屏状态，显示后再次读取实际按钮保留区。
  if (useNativeWindowStore.getState().windowsFrame) {
    const revision = ++layoutRevision;
    try {
      const layout = await withTimeout(invoke<WindowLayout>('get_window_layout'), 1000);
      if (revision === layoutRevision && layout) applyWindowLayout(layout);
    } catch {
      /* 保留最后一次有效几何。 */
    }
  }
}

async function initializeWindowsFrame() {
  let completed = false;
  const enable = invoke('set_titlebar_controls', { regions: [], windowsFrame: true }).then(() => {
    completed = true;
  });
  const rollback = async () => {
    await invoke('set_titlebar_controls', { regions: [], windowsFrame: false });
    await refreshNativeAppearance(true);
  };
  try {
    await withTimeout(enable, 1000);
    // 原生窗框安装后刷新背景刷，避免浅色RGB盖住DWM按钮。
    await refreshNativeAppearance(true);
    const revision = ++layoutRevision;
    const layout = await withTimeout(invoke<WindowLayout>('get_window_layout'), 1000);
    if (layout?.platform !== 'windows' || !validWindowsFrame(layout.windowsFrame))
      throw new Error('Invalid Windows frame layout');
    if (revision === layoutRevision) applyWindowLayout(layout);
  } catch {
    ++layoutRevision;
    applyWindowLayout({ platform: 'windows' });
    // 超时不取消原生请求，迟到安装完成后仍须撤除，不能留下网页旧布局与新窗框。
    if (!completed) void enable.then(rollback, () => {}).catch(() => {});
    await withTimeout(rollback(), 1000).catch(() => {});
  }
}

export async function refreshNativeAppearance(startup = false): Promise<void> {
  // 辅助功能或迟到启动刷新只重用最近的主题结果，不能重新从正文/旧启动缓存选色。
  if (resolvedColor) {
    await enqueueNativeAppearance(resolvedColor).catch(() => {});
    return;
  }
  const root = document.documentElement;
  const hex = getComputedStyle(root)
    .getPropertyValue(startup ? '--startup-background' : '--bg-base')
    .trim();
  const value = /^#[\da-f]{6}$/i.test(hex) ? Number.parseInt(hex.slice(1), 16) : 0x1c1c1e;
  await enqueueNativeAppearance({
    red: value >> 16,
    green: (value >> 8) & 255,
    blue: value & 255,
  }).catch(() => {});
}
