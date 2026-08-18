import { create } from 'zustand';
import type { ReactNode } from 'react';

/**
 * 页面壳配置（B1 壳常驻）：
 * 每个页面通过 <PageShell> 把 title/actions/onBack 注册到这里，
 * 常驻的 <ShellLayout> 读取本 store 渲染 <AppShell>（侧边栏/顶栏/底部导航）。
 * 壳在路由 Suspense 之外，切页不再整体卸载，消除「整窗空白」。
 */
export interface ShellConfig {
  title: string;
  actions?: ReactNode;
  onBack?: () => void;
}

interface ShellConfigState extends ShellConfig {
  /** 页面注册配置；内容不变时跳过更新，避免页面每次重渲染都触发壳重渲染。 */
  setConfig: (config: ShellConfig) => void;
}

export const useShellConfigStore = create<ShellConfigState>((set) => ({
  title: '',
  actions: undefined,
  onBack: undefined,
  setConfig: (config) =>
    set((prev) => {
      if (
        prev.title === config.title &&
        prev.actions === config.actions &&
        prev.onBack === config.onBack
      ) {
        return prev;
      }
      return config;
    }),
}));
