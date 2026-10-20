import { create } from 'zustand';
export const syncPlaintextPref = async () => {};
// Public in-memory data only; the native host exposes no Vault/business commands.
const customPages = [
  {
    id: 'public-long-page',
    name: '用于验收长名称悬停滚动的公开示例页面 — Desktop layout acceptance',
    iconId: 'folder',
    createdAt: '2026-10-20T00:00:00Z',
  },
];
export const publicObjects = Array.from({ length: 48 }, (_, index) => ({
  id: `public-object-${index}`,
  name:
    index === 0
      ? '用于验收长名称悬停滚动的公开示例对象 — A long navigation label'
      : `公开示例对象 ${String(index + 1).padStart(2, '0')}`,
  sectionId: index < 36 ? 'identity' : index < 42 ? 'document' : 'public-long-page',
  iconName: 'file-text',
}));
export const useDesktopPageIndexStore = create(() => ({
  objects: publicObjects,
  loading: false,
  failed: false,
  refresh: async () => {},
}));
export const observeDesktopPageIndex = () => () => {};
export const useSettingsStore = create((set) => ({
  settings: {
    sidebarPosition: 'left',
    reduceMotion: false,
    theme: 'dark',
    customPages,
    sidebarButtonModes: {},
  },
  addCustomPage: async (_accountId: string, name: string, iconId: string, description?: string) => {
    const page = {
      id: crypto.randomUUID(),
      name,
      iconId,
      description,
      createdAt: new Date().toISOString(),
    };
    set((state: { settings: { customPages: typeof customPages } }) => ({
      settings: { ...state.settings, customPages: [...state.settings.customPages, page] },
    }));
    return page;
  },
}));
export const useAuthStore = create(() => ({
  isAuthenticated: true,
  isUnlocked: true,
  user: null,
  currentAccount: { id: 'public-layout-fixture' },
  lock: () => {},
}));
export const useSyncStore = create(() => ({
  incomingPairingRequest: null,
  initPairingRequestListener: async () => () => {},
  initSyncCompletedListener: async () => () => {},
  clearIncomingPairingRequest: () => {},
}));
