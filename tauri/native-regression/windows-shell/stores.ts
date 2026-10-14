import { create } from 'zustand';
export const syncPlaintextPref = async () => {};
export const useSettingsStore = create(() => ({
  settings: { sidebarPosition: 'left', reduceMotion: false, theme: 'dark' },
}));
export const useAuthStore = create(() => ({
  isAuthenticated: true,
  isUnlocked: true,
  user: null,
  lock: () => {},
}));
export const useSyncStore = create(() => ({
  incomingPairingRequest: null,
  initPairingRequestListener: async () => () => {},
  initSyncCompletedListener: async () => () => {},
  clearIncomingPairingRequest: () => {},
}));
