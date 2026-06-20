import { create } from 'zustand';

export type QuickPanelTab = 'all' | 'installed' | 'running';

interface PluginQuickState {
  isOpen: boolean;
  activeTab: QuickPanelTab;
  setOpen: (open: boolean) => void;
  toggleOpen: () => void;
  setActiveTab: (tab: QuickPanelTab) => void;
}

export const usePluginQuickStore = create<PluginQuickState>()((set) => ({
  isOpen: false,
  activeTab: 'all',
  setOpen: (open) => set({ isOpen: open }),
  toggleOpen: () => set((s) => ({ isOpen: !s.isOpen })),
  setActiveTab: (tab) => set({ activeTab: tab }),
}));
