import { create } from 'zustand';

export interface WindowsFrameLayout {
  enabled: boolean;
  headerHeight: number;
  dpi: number;
  maximized: boolean;
  fullWindowBackdrop?: boolean;
  captionBounds: [number, number, number, number];
}

/** 原生标题栏几何与平台状态；只保存当前窗口测量，不持久化。 */
export const useNativeWindowStore = create<{
  isMacOS: boolean;
  isWindows: boolean;
  titlebarHeight: number;
  trafficLightsRight: number;
  windowsFrame: WindowsFrameLayout | null;
}>(() => ({
  isMacOS: false,
  isWindows: false,
  titlebarHeight: 0,
  trafficLightsRight: 0,
  windowsFrame: null,
}));
