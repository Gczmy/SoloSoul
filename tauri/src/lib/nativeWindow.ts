import { invokeCommand as invoke } from './ipcClient';
import { withTimeout } from './withTimeout';
import { useNativeWindowStore } from '@/stores/nativeWindowStore';

interface WindowLayout {
  platform: 'macos' | 'windows' | 'other';
  titlebarHeight?: number;
  trafficLightsRight?: number;
}

interface WindowAppearance extends WindowLayout {
  material: 'solid' | 'vibrancy' | 'liquid-glass' | 'mica';
  reduceMotion: boolean;
  highContrast: boolean;
}

let appearanceRevision = 0;
let layoutRevision = 0;
let appearanceQueue: Promise<void> = Promise.resolve();

function applyWindowLayout(appearance: WindowLayout) {
  const root = document.documentElement;
  root.dataset.desktopPlatform = appearance.platform;
  const height = appearance.platform === 'macos' ? appearance.titlebarHeight : 0;
  const titlebarHeight =
    typeof height === 'number' && Number.isFinite(height) ? Math.max(0, height) : 0;
  root.style.setProperty('--native-titlebar-height', `${titlebarHeight}px`);
  const right = appearance.platform === 'macos' ? appearance.trafficLightsRight : 0;
  const trafficLightsRight =
    typeof right === 'number' && Number.isFinite(right) ? Math.max(0, right) : 0;
  useNativeWindowStore.setState({
    isMacOS: appearance.platform === 'macos',
    isWindows: appearance.platform === 'windows',
    titlebarHeight,
    trafficLightsRight,
  });
}

/** 外观请求串行执行；连续切换时跳过尚未执行的旧主题，迟到响应不回写布局。 */
export function syncNativeAppearance(color: { red: number; green: number; blue: number }) {
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
      root.dataset.reduceMotion = String(appearance.reduceMotion);
      root.dataset.highContrast = String(appearance.highContrast);
      if (geometryRevision === layoutRevision) applyWindowLayout(appearance);
    });
  appearanceQueue = next;
  return next;
}

/** 恢复/全屏/缩放只读几何；不重设外观、背景或材质视图。 */
export function observeNativeWindowLayout() {
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
    if (document.documentElement.dataset.desktopPlatform !== 'macos') return;
    if (running) {
      dirty = true;
      return;
    }
    cancelAnimationFrame(frame);
    frame = requestAnimationFrame(() => void measure());
  };
  window.addEventListener('resize', update);
  window.addEventListener('focus', update);
  update();
  return () => {
    stopped = true;
    cancelAnimationFrame(frame);
    window.removeEventListener('resize', update);
    window.removeEventListener('focus', update);
  };
}

/** 图标解码后显示原生窗口；隐藏 WebView 不保证调度 RAF，因此设置有界绘制等待。 */
export async function prepareStartupWindow(): Promise<void> {
  await refreshNativeAppearance(true).catch(() => {});
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
}

export async function refreshNativeAppearance(startup = false): Promise<void> {
  const root = document.documentElement;
  const hex = getComputedStyle(root)
    .getPropertyValue(startup ? '--startup-background' : '--bg-base')
    .trim();
  const value = /^#[\da-f]{6}$/i.test(hex) ? Number.parseInt(hex.slice(1), 16) : 0x1c1c1e;
  await syncNativeAppearance({
    red: value >> 16,
    green: (value >> 8) & 255,
    blue: value & 255,
  }).catch(() => {});
}
