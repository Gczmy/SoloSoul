import { invokeCommand as invoke } from './ipcClient';
import { withTimeout } from './withTimeout';
import { useNativeWindowStore } from '@/stores/nativeWindowStore';

interface WindowAppearance {
  material: 'solid' | 'vibrancy' | 'liquid-glass' | 'mica';
  platform: 'macos' | 'windows' | 'other';
  reduceMotion: boolean;
  highContrast: boolean;
  titlebarHeight?: number;
}

export async function syncNativeAppearance(color: { red: number; green: number; blue: number }) {
  const appearance = await withTimeout(
    invoke<WindowAppearance>('set_titlebar_color', { color }),
    1000,
  );
  if (!appearance || !['solid', 'vibrancy', 'liquid-glass', 'mica'].includes(appearance.material))
    return;
  const root = document.documentElement;
  root.dataset.nativeMaterial = appearance.material;
  root.dataset.desktopPlatform = appearance.platform;
  root.dataset.reduceMotion = String(appearance.reduceMotion);
  root.dataset.highContrast = String(appearance.highContrast);
  const height = appearance.platform === 'macos' ? appearance.titlebarHeight : 0;
  const titlebarHeight =
    typeof height === 'number' && Number.isFinite(height) ? Math.max(0, height) : 0;
  root.style.setProperty('--native-titlebar-height', `${titlebarHeight}px`);
  useNativeWindowStore.setState({ titlebarHeight });
}

/** 全屏/缩放完成布局后重新测量系统避让区，不使用固定像素或主题轮询时序。 */
export function observeNativeWindowLayout() {
  let frame = 0;
  const update = () => {
    if (document.documentElement.dataset.desktopPlatform !== 'macos') return;
    cancelAnimationFrame(frame);
    frame = requestAnimationFrame(() => {
      void refreshNativeAppearance();
    });
  };
  window.addEventListener('resize', update);
  update();
  return () => {
    cancelAnimationFrame(frame);
    window.removeEventListener('resize', update);
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
