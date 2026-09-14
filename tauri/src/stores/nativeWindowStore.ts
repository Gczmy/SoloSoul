import { create } from 'zustand';

/** 原生标题栏的避让高度；浏览器、Windows 和移动端均为 0，不持久化。 */
export const useNativeWindowStore = create<{ titlebarHeight: number }>(() => ({
  titlebarHeight: 0,
}));
