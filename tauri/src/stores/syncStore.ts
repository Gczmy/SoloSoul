import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import type { SyncResult } from '@/lib/ipc';

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
  lastResult: SyncResult | null;
  recentResults: SyncResult[];

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
  recentResults: [],

  loadStatus: async () => {
    try {
      const status = await invoke<SyncStatus>('sync_get_status');
      set({ ...status, error: null });
    } catch (err) {
      set({ error: String(err) });
    }
  },

  enable: async (enabled) => {
    set({ isLoading: true, error: null, lastResult: null });
    try {
      await invoke<void>('sync_enable', { enable: enabled });
      const status = await invoke<SyncStatus>('sync_get_status');
      set({ ...status, isLoading: false, error: null });
    } catch (err) {
      set({ isLoading: false, error: String(err) });
    }
  },

  syncWithDevice: async (deviceId) => {
    set({ isLoading: true, error: null, lastResult: null });
    try {
      const result = await invoke<SyncResult>('sync_with_device', { deviceId });
      await get().loadStatus();
      set((state) => ({
        isLoading: false,
        lastResult: result,
        recentResults: [result, ...state.recentResults].slice(0, 10),
      }));
    } catch (err) {
      set({ isLoading: false, error: String(err) });
    }
  },

  trustPeer: async (peerNodeId, trusted) => {
    set({ isLoading: true, error: null });
    try {
      await invoke<void>('sync_trust_peer', { peerNodeId, trusted });
      await get().loadStatus();
      set({ isLoading: false });
    } catch (err) {
      set({ isLoading: false, error: String(err) });
    }
  },

  forgetPeer: async (peerNodeId) => {
    set({ isLoading: true, error: null });
    try {
      await invoke<void>('sync_forget_peer', { peerNodeId });
      await get().loadStatus();
      set({ isLoading: false });
    } catch (err) {
      set({ isLoading: false, error: String(err) });
    }
  },
}));
