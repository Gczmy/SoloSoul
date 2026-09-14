import { createStore } from 'zustand/vanilla';
import { ST_MACOS_WINDOW_OPAQUE } from '@/lib/constants';

function readOpaquePreference(): boolean {
  try {
    return localStorage.getItem(ST_MACOS_WINDOW_OPAQUE) === 'true';
  } catch {
    return false;
  }
}

/** 设备兼容选项独立于账户主题；首屏同步读取，vanilla store 避免启动层提前加载 React。 */
export const windowAppearanceStore = createStore<{
  opaque: boolean;
  isSaving: boolean;
  setOpaque: (opaque: boolean) => void;
}>((set) => ({
  opaque: readOpaquePreference(),
  isSaving: false,
  setOpaque: (opaque) => {
    // 持久化失败时保持原选项，由调用方提示错误。
    localStorage.setItem(ST_MACOS_WINDOW_OPAQUE, String(opaque));
    set({ opaque });
  },
}));
