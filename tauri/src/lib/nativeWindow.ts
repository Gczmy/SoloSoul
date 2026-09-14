import { invokeCommand as invoke } from './ipcClient';
import { withTimeout } from './withTimeout';
import { windowAppearanceStore } from '@/stores/windowAppearanceStore';

interface WindowAppearance {
  material: 'solid' | 'vibrancy' | 'liquid-glass' | 'mica';
  platform: 'macos' | 'windows' | 'other';
  reduceMotion: boolean;
  highContrast: boolean;
}

export async function syncNativeAppearance(color: { red: number; green: number; blue: number }) {
  const appearance = await withTimeout(
    invoke<WindowAppearance>('set_titlebar_color', {
      color,
      opaque: windowAppearanceStore.getState().opaque,
    }),
    1000,
  );
  if (!appearance || !['solid', 'vibrancy', 'liquid-glass', 'mica'].includes(appearance.material))
    return;
  const root = document.documentElement;
  root.dataset.nativeMaterial = appearance.material;
  root.dataset.desktopPlatform = appearance.platform;
  root.dataset.reduceMotion = String(appearance.reduceMotion);
  root.dataset.highContrast = String(appearance.highContrast);
  return appearance;
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
  await syncNativeAppearance(currentBackgroundColor(startup)).catch(() => {});
}

function currentBackgroundColor(startup = false) {
  const root = document.documentElement;
  const hex = getComputedStyle(root)
    .getPropertyValue(startup ? '--startup-background' : '--bg-base')
    .trim();
  const value = /^#[\da-f]{6}$/i.test(hex) ? Number.parseInt(hex.slice(1), 16) : 0x1c1c1e;
  return {
    red: value >> 16,
    green: (value >> 8) & 255,
    blue: value & 255,
  };
}

/** 先保存当前选择，让并发的主题同步也使用同一材质；应用失败时恢复原选项。 */
export async function setOpaqueWindow(opaque: boolean): Promise<void> {
  const previous = windowAppearanceStore.getState();
  if (previous.isSaving || previous.opaque === opaque) return;
  windowAppearanceStore.setState({ isSaving: true });
  try {
    previous.setOpaque(opaque);
    const appearance = await syncNativeAppearance(currentBackgroundColor());
    if (
      !appearance ||
      appearance.platform !== 'macos' ||
      (opaque && appearance.material !== 'solid')
    ) {
      throw new Error('Native window appearance was not applied');
    }
  } catch (error) {
    previous.setOpaque(previous.opaque);
    await refreshNativeAppearance();
    throw error;
  } finally {
    windowAppearanceStore.setState({ isSaving: false });
  }
}
