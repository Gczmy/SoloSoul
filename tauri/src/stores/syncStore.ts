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

export interface DiscoveredDevice {
  name: string;
  host: string;
  port: number;
  addresses: string[];
}

interface SyncStatus {
  isDiscovering: boolean;
  syncEnabled: boolean;
  autoSyncEnabled: boolean;
  localFingerprint: string;
  connectedPeers: SyncPeer[];
}

interface SyncStoreState extends SyncStatus {
  isLoading: boolean;
  error: string | null;
  lastResult: SyncResult | null;
  recentResults: SyncResult[];
  discoveredDevices: DiscoveredDevice[];
  isDiscoveringDevices: boolean;
  listenPort: number;

  loadStatus: () => Promise<void>;
  loadListenPort: () => Promise<void>;
  enable: (enabled: boolean) => Promise<void>;
  discoverDevices: (timeoutMs?: number) => Promise<void>;
  syncWithDevice: (deviceId: string) => Promise<void>;
  trustPeer: (peerNodeId: string, trusted: boolean) => Promise<void>;
  forgetPeer: (peerNodeId: string) => Promise<void>;
  loadAutoSyncStatus: () => Promise<void>;
  setAutoSyncEnabled: (enabled: boolean) => Promise<void>;
  triggerForegroundSync: () => Promise<void>;
}

export const useSyncStore = create<SyncStoreState>((set, get) => ({
  isDiscovering: false,
  syncEnabled: false,
  autoSyncEnabled: false,
  localFingerprint: '',
  connectedPeers: [],
  isLoading: false,
  error: null,
  lastResult: null,
  recentResults: [],
  discoveredDevices: [],
  isDiscoveringDevices: false,
  listenPort: 0,

  loadStatus: async () => {
    try {
      const status = await invoke<SyncStatus>('sync_get_status');
      set({ ...status, error: null });
    } catch (err) {
      set({ error: String(err) });
    }
  },

  loadListenPort: async () => {
    try {
      const port = await invoke<number>('sync_listen_port');
      set({ listenPort: port });
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
      // 启用后自动发现附近设备，同时刷新监听端口用于手动 fallback
      if (enabled) {
        void get().discoverDevices(5000);
        void get().loadListenPort();
      } else {
        set({ discoveredDevices: [], listenPort: 0 });
      }
    } catch (err) {
      set({ isLoading: false, error: String(err) });
    }
  },

  discoverDevices: async (timeoutMs = 5000) => {
    set({ isDiscoveringDevices: true, error: null });
    try {
      const devices = await invoke<DiscoveredDevice[]>('mdns_discover', { timeoutMs });
      set({ discoveredDevices: devices, isDiscoveringDevices: false, error: null });
    } catch (err) {
      set({ isDiscoveringDevices: false, error: String(err) });
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

  loadAutoSyncStatus: async () => {
    try {
      const enabled = await invoke<boolean>('sync_get_auto_status');
      set({ autoSyncEnabled: enabled });
    } catch (err) {
      set({ error: String(err) });
    }
  },

  setAutoSyncEnabled: async (enabled) => {
    set({ isLoading: true, error: null });
    try {
      const result = await invoke<boolean>('sync_set_auto_enabled', { enabled });
      set({ autoSyncEnabled: result, isLoading: false });
    } catch (err) {
      set({ isLoading: false, error: String(err) });
    }
  },

  triggerForegroundSync: async () => {
    try {
      await invoke<void>('sync_trigger_foreground');
    } catch (err) {
      set({ error: String(err) });
    }
  },
}));
