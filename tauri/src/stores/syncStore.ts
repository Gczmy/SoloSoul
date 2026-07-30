import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type {
  SyncResult,
  SyncConflictSummary,
  SyncConflictDetail,
  SyncConflictStrategy,
} from '@/lib/ipc';
import { logger } from '@/lib/logger';

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
  conflicts: SyncConflictSummary[];
  selectedConflict: SyncConflictDetail | null;
  /** 是否有未查看的冲突通知（由 sync-conflicts-updated 事件触发）。 */
  hasUnreadConflicts: boolean;

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
  loadConflicts: () => Promise<void>;
  loadConflictDetail: (conflictId: string) => Promise<void>;
  resolveConflict: (conflictId: string, strategy: SyncConflictStrategy) => Promise<void>;
  markConflictsRead: () => void;
  initConflictListener: () => Promise<UnlistenFn>;
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
  conflicts: [],
  selectedConflict: null,
  hasUnreadConflicts: false,

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
      const devices = await invoke<DiscoveredDevice[]>('mdns_discover', { timeoutMs: timeoutMs });
      set({ discoveredDevices: devices, isDiscoveringDevices: false, error: null });
    } catch (err) {
      set({ isDiscoveringDevices: false, error: String(err) });
    }
  },

  syncWithDevice: async (deviceId) => {
    set({ isLoading: true, error: null, lastResult: null });
    try {
      const result = await invoke<SyncResult>('sync_with_device', { deviceId: deviceId });
      await get().loadStatus();
      await get().loadConflicts();
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
      await invoke<void>('sync_trust_peer', { peerNodeId: peerNodeId, trusted });
      await get().loadStatus();
      set({ isLoading: false });
    } catch (err) {
      set({ isLoading: false, error: String(err) });
    }
  },

  forgetPeer: async (peerNodeId) => {
    set({ isLoading: true, error: null });
    try {
      await invoke<void>('sync_forget_peer', { peerNodeId: peerNodeId });
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

  loadConflicts: async () => {
    try {
      const conflicts = await invoke<SyncConflictSummary[]>('sync_list_conflicts');
      set({ conflicts, error: null });
    } catch (err) {
      set({ error: String(err) });
    }
  },

  /** 标记冲突通知为已读（用户打开冲突对话框时调用）。 */
  markConflictsRead: () => {
    set({ hasUnreadConflicts: false });
  },

  /** 初始化 sync-conflicts-updated 事件监听器。
   *  返回 unlisten 函数，调用方应在组件卸载时调用以清理。 */
  initConflictListener: (): Promise<UnlistenFn> => {
    return listen<{ count: number }>('sync-conflicts-updated', (event) => {
      const count = event.payload?.count ?? 0;
      if (count > 0) {
        set({ hasUnreadConflicts: true });
        // 自动刷新冲突列表，确保 UI 数据是最新的
        get().loadConflicts().catch((err) =>
          logger.warn('[syncStore] Failed to auto-reload conflicts after event:', err),
        );
      }
    });
  },

  loadConflictDetail: async (conflictId) => {
    try {
      const detail = await invoke<SyncConflictDetail>('sync_get_conflict_detail', {
        conflictId: conflictId,
      });
      set({ selectedConflict: detail, error: null });
    } catch (err) {
      set({ error: String(err) });
    }
  },

  resolveConflict: async (conflictId, strategy) => {
    set({ isLoading: true, error: null });
    try {
      await invoke<boolean>('sync_resolve_conflict', { conflictId: conflictId, strategy });
      await get().loadConflicts();
      set((state) => ({
        selectedConflict:
          state.selectedConflict?.id === conflictId ? null : state.selectedConflict,
        isLoading: false,
      }));
    } catch (err) {
      set({ isLoading: false, error: String(err) });
    }
  },
}));
