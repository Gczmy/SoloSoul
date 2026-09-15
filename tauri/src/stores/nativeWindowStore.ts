import { create } from 'zustand';

/** 原生标题栏几何与平台状态；浏览器、Windows 和移动端的几何均为 0，不持久化。 */
export const useNativeWindowStore = create<{
  isMacOS: boolean;
  isWindows: boolean;
  titlebarHeight: number;
  trafficLightsRight: number;
}>(() => ({
  isMacOS: false,
  isWindows: false,
  titlebarHeight: 0,
  trafficLightsRight: 0,
}));
