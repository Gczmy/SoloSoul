import { create } from 'zustand';
import { commands } from '@/lib/ipc';

export interface SyncPeer {
  id: string;
  name: string;
  addr: string;
  fingerprint: string;
  trusted: boolean;
  lastSeen: string;
}

interface SyncStatus {
  isDiscovering: boolean;
  syncEnabled: boolean;
  localFingerprint: string;
  connectedPeers: SyncPeer[];
}

interface SyncStoreState extends SyncStatus {
  isLoading: boolean;
  error: string | null;
  lastResult: string | null;

  loadStatus: () => Promise<void>;
  enable: (enabled: boolean) => Promise<void>;
  syncWithDevice: (deviceId: string) => Promise<void>;
  trustPeer: (peerNodeId: string, trusted: boolean) => Promise<void>;
  forgetPeer: (peerNodeId: string) => Promise<void>;
}

export const useSyncStore = create<SyncStoreState>((set, get) => ({
  isDiscovering: false,
  syncEnabled: false,
  localFingerprint: '',
  connectedPeers: [],
  isLoading: false,
  error: null,
  lastResult: null,

  loadStatus: async () => {
    try {
      const status = await commands.syncGetStatus();
      set({ ...status, error: null });
    } catch (err) {
      set({ error: String(err) });
    }
  },

  enable: async (enabled) => {
    set({ isLoading: true, error: null, lastResult: null });
    try {
      await commands.syncEnable(enabled);
      const status = await commands.syncGetStatus();
      set({ ...status, isLoading: false, error: null });
    } catch (err) {
      set({ isLoading: false, error: String(err) });
    }
  },

  syncWithDevice: async (deviceId) => {
    set({ isLoading: true, error: null, lastResult: null });
    try {
      const result = await commands.syncWithDevice(deviceId);
      await get().loadStatus();
      set({ isLoading: false, lastResult: result });
    } catch (err) {
      set({ isLoading: false, error: String(err) });
    }
  },

  trustPeer: async (peerNodeId, trusted) => {
    set({ isLoading: true, error: null });
    try {
      await commands.syncTrustPeer(peerNodeId, trusted);
      await get().loadStatus();
      set({ isLoading: false });
    } catch (err) {
      set({ isLoading: false, error: String(err) });
    }
  },

  forgetPeer: async (peerNodeId) => {
    set({ isLoading: true, error: null });
    try {
      await commands.syncForgetPeer(peerNodeId);
      await get().loadStatus();
      set({ isLoading: false });
    } catch (err) {
      set({ isLoading: false, error: String(err) });
    }
  },
}));
