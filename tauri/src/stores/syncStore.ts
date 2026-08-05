import { create } from 'zustand';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import i18next from '@/lib/i18n';
import type {
  SyncResult,
  SyncConflictSummary,
  SyncConflictDetail,
  SyncConflictStrategy,
} from '@/lib/ipc';
import { useUiStore } from '@/stores/uiStore';
import { logger } from '@/lib/logger';

export interface SyncPeer {
  id: string;
  name: string;
  addr: string;
  fingerprint: string;
  trusted: boolean;
  lastSeen: string;
  /** 最近一次同步/在线的原始 unix 秒时间戳（精确展示用）。 */
  lastSeenTs?: number | null;
  /** 最近一次信任该设备的时间（unix 秒）。从未信任/已撤销时为 null。 */
  trustedAt?: number | null;
  /** 客户端类型：macos / windows / linux / android / ios / unknown。 */
  clientType?: string;
  /** 6 位 SAS 配对验证码（仅配对中临时携带，两侧展示同一数字供目视比对）。 */
  sasCode?: string;
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
  /** 本地监听地址（host:port，未启用时为空串）。 */
  listenAddr: string;
  conflicts: SyncConflictSummary[];
  selectedConflict: SyncConflictDetail | null;
  /** 是否有未查看的冲突通知（由 sync-conflicts-updated 事件触发）。 */
  hasUnreadConflicts: boolean;
  /** 配对中（A 侧发起方）：等待对端确认配对的 peer。 */
  pairingPendingPeerId: string | null;
  /** 配对中（A 侧发起方）：发起同步时使用的地址，用于确认信任后自动重试。 */
  pairingPendingAddr: string | null;
  /** 配对中（A 侧发起方）：本次握手派生的 6 位 SAS 验证码（配对卡片展示）。 */
  pairingPendingSasCode: string | null;
  /** 入站配对请求（B 侧响应方）：AppShell 全局监听后弹出确认对话框。 */
  incomingPairingRequest: SyncPeer | null;

  loadStatus: () => Promise<void>;
  loadListenAddr: () => Promise<void>;
  enable: (enabled: boolean) => Promise<void>;
  discoverDevices: (timeoutMs?: number) => Promise<void>;
  syncWithDevice: (deviceId: string) => Promise<void>;
  /** 标记 peer 信任状态。fingerprint（可选）：配对确认时绑定握手认证指纹（P001/P103）。 */
  trustPeer: (peerNodeId: string, trusted: boolean, fingerprint?: string) => Promise<void>;
  forgetPeer: (peerNodeId: string) => Promise<void>;
  loadAutoSyncStatus: () => Promise<void>;
  setAutoSyncEnabled: (enabled: boolean) => Promise<void>;
  triggerForegroundSync: () => Promise<void>;
  loadConflicts: () => Promise<void>;
  loadConflictDetail: (conflictId: string) => Promise<void>;
  resolveConflict: (conflictId: string, strategy: SyncConflictStrategy) => Promise<void>;
  markConflictsRead: () => void;
  initConflictListener: () => Promise<UnlistenFn>;
  initNsdFailedListener: () => Promise<UnlistenFn>;
  /** 监听入站配对请求事件（sync-pairing-request），B 用户任意页面都能收到。 */
  initPairingRequestListener: () => Promise<UnlistenFn>;
  /** 监听入站同步完成事件（sync-completed），B 侧任意页面收到完成提醒与具体条数。 */
  initSyncCompletedListener: () => Promise<UnlistenFn>;
  /** 清除 A 侧配对中状态（取消等待 / 配对完成）。 */
  clearPairingPending: () => void;
  /** 清除 B 侧入站配对请求。 */
  clearIncomingPairingRequest: () => void;
}

export const useSyncStore = create<SyncStoreState>((set, get) => {
  /** 串行化 enable 切换的队列尾（闭包内私有，不进 state，避免每次入队触发 re-render）。 */
  let enableChain: Promise<void> = Promise.resolve();

  /** 追加一个需要串行的 enable 任务，返回该任务完成后的 promise。 */
  const enqueueEnable = (task: () => Promise<void>): Promise<void> => {
    const next = enableChain.then(task).catch((err) => {
      // 上游任务失败不应阻断后续任务；错误已在各自任务内处理
      logger.warn('[syncStore] enable chain task failed:', err);
    });
    enableChain = next;
    return next;
  };

  return {
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
  listenAddr: '',
  conflicts: [],
  selectedConflict: null,
  hasUnreadConflicts: false,
  pairingPendingPeerId: null,
  pairingPendingAddr: null,
  pairingPendingSasCode: null,
  incomingPairingRequest: null,

  loadStatus: async () => {
    try {
      const status = await invoke<SyncStatus>('sync_get_status');
      set({ ...status, error: null });
    } catch (err) {
      set({ error: String(err) });
    }
  },

  loadListenAddr: async () => {
    try {
      const addr = await invoke<string>('sync_listen_addr');
      set({ listenAddr: addr });
    } catch (err) {
      set({ error: String(err) });
    }
  },

  enable: async (enabled) => {
    // 串行化：若已有 enable 流程在途，排队等待其完成后再执行本次切换，
    // 确保快速连点“启用→禁用”时最终状态与最后一次点击一致。
    await enqueueEnable(async () => {
      set({ isLoading: true, error: null, lastResult: null });
      let timeoutHandle: ReturnType<typeof setTimeout> | undefined;
      try {
        // 超时保护：sync_enable + sync_get_status 总耗时超过 15 秒时自动重置 isLoading
        const result = await Promise.race([
          (async () => {
            await invoke<void>('sync_enable', { enable: enabled });
            const status = await invoke<SyncStatus>('sync_get_status');
            return { status };
          })(),
          new Promise<never>((_, reject) => {
            timeoutHandle = setTimeout(
              () => reject(new Error('__SYNC_ERR__:enable_timeout')),
              15_000,
            );
          }),
        ]);
        clearTimeout(timeoutHandle);
        set({ ...result.status, isLoading: false, error: null });
        // 启用后自动发现附近设备，同时刷新监听地址用于手动 fallback
        if (enabled) {
          void get().discoverDevices(5000);
          void get().loadListenAddr();
        } else {
          set({ discoveredDevices: [], listenAddr: '', isDiscoveringDevices: false });
        }
      } catch (err) {
        clearTimeout(timeoutHandle);
        set({ isLoading: false, error: String(err) });
        // 超时/失败后主动重读后端状态：Android 上 sync_enable 可能因 NSD 权限弹窗
        // 或命令排队导致超时，但服务端最终可能已切换成功。重读可让开关 UI 与
        // 实际状态保持一致，避免“禁用失败但实际已禁用”的假卡死。
        void get()
          .loadStatus()
          .catch((e2) => logger.warn('[syncStore] status resync after enable error:', e2));
      }
    });
  },

  discoverDevices: async (timeoutMs = 5000) => {
    // 已在发现中则忽略，避免 enable 成功回调与页面 useEffect 重复触发叠加
    if (get().isDiscoveringDevices) {
      return;
    }
    set({ isDiscoveringDevices: true, error: null });
    let timeoutHandle: ReturnType<typeof setTimeout> | undefined;
    try {
      const devices = await Promise.race([
        invoke<DiscoveredDevice[]>('mdns_discover', { timeoutMs: timeoutMs }),
        new Promise<never>((_, reject) => {
          // 兜底超时：移动端 request_permissions 弹窗未响应时 mdns_discover 可能挂起
          timeoutHandle = setTimeout(
            () => reject(new Error('__SYNC_ERR__:discovery_timeout')),
            timeoutMs + 10_000,
          );
        }),
      ]);
      clearTimeout(timeoutHandle);
      // 禁用同步后，在途发现结果不再回写，避免设备列表复活
      if (!get().syncEnabled) {
        set({ isDiscoveringDevices: false });
        return;
      }
      set({ discoveredDevices: devices, isDiscoveringDevices: false, error: null });
    } catch (err) {
      clearTimeout(timeoutHandle);
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
      const raw = String(err);
      // 对端尚未信任本设备 → 进入双侧确认配对流程（A 侧发起方）。
      // B 已被 record_peer 持久化（含指纹），重读状态后弹配对卡片。
      // 新后端返回 `__SYNC_ERR__:pairing_pending:{peerId}:{sas}`（sas = 6 位验证码），
      // 旧格式 `{peerId}` 亦兼容（无 sasCode）。
      const pairingMatch = raw.match(/^__SYNC_ERR__:pairing_pending:([^:]+)(?::(\d{6}))?$/);
      if (pairingMatch) {
        const peerId = pairingMatch[1];
        const sasCode = pairingMatch[2] ?? null;
        await get().loadStatus();
        set({
          isLoading: false,
          error: null,
          pairingPendingPeerId: peerId,
          pairingPendingAddr: deviceId,
          pairingPendingSasCode: sasCode,
        });
        return;
      }
      set({ isLoading: false, error: raw });
    }
  },

  trustPeer: async (peerNodeId, trusted, fingerprint) => {
    set({ isLoading: true, error: null });
    try {
      await invoke<void>('sync_trust_peer', {
        peerNodeId: peerNodeId,
        trusted,
        fingerprint: fingerprint ?? null,
      });
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

  /** 初始化 sync-pairing-request 事件监听器（入站配对请求）。
   *  后端在响应方落库一条新的未信任 peer 记录时触发；
   *  AppShell 全局挂载后，B 用户不在同步页也能收到配对确认对话框。
   *  返回 unlisten 函数，调用方应在组件卸载时调用以清理。 */
  initPairingRequestListener: (): Promise<UnlistenFn> => {
    return listen<{
      nodeId: string;
      fingerprint: string;
      addr: string;
      deviceName: string;
      sasCode?: string;
    }>('sync-pairing-request', (event) => {
      const p = event.payload;
      // P103: 未信任 peer 不再落库，同一 peer 重连会重复触发本事件。
      // 若已展示同一 peer 的配对请求：仅更新本次会话的 SAS 验证码（A 侧
      // 忽略后重新发起会开启新握手、新验证码），不重建整个卡片，避免
      // A 侧自动重试（多次连接）导致 B 侧对话框反复闪烁。
      // 用户确认或忽略清除后，新请求可重新弹出。
      const existing = get().incomingPairingRequest;
      if (existing?.id === p.nodeId) {
        if (p.sasCode && existing.sasCode !== p.sasCode) {
          set({ incomingPairingRequest: { ...existing, sasCode: p.sasCode } });
        }
        return;
      }
      const deviceName =
        p.deviceName ||
        (p.fingerprint ? `SoloSoul-${p.fingerprint.slice(0, 8)}` : p.nodeId);
      set({
        incomingPairingRequest: {
          id: p.nodeId,
          name: deviceName,
          addr: p.addr || '',
          fingerprint: p.fingerprint || '',
          trusted: false,
          lastSeen: '',
          lastSeenTs: null,
          trustedAt: null,
          clientType: 'unknown',
          sasCode: p.sasCode || '',
        },
      });
    });
  },

  /** 清除 A 侧配对中状态（取消等待 / 配对完成 / 忽略）。 */
  clearPairingPending: () => {
    set({ pairingPendingPeerId: null, pairingPendingAddr: null, pairingPendingSasCode: null });
  },

  /** 清除 B 侧入站配对请求（确认或忽略后）。 */
  clearIncomingPairingRequest: () => {
    set({ incomingPairingRequest: null });
  },

  /** 初始化 sync-completed 事件监听器（响应方入站同步完成通知）。
   *  后端在响应方成功完成一次入站会话时 emit（两侧同时提醒，与发起方 toast 对称）；
   *  这里写入 lastResult/recentResults（结果行展示具体条数）+ 全局 toast + 刷新状态/冲突。
   *  返回 unlisten 函数，调用方应在组件卸载时调用以清理。 */
  initSyncCompletedListener: (): Promise<UnlistenFn> => {
    return listen<{
      peerNodeId: string;
      examined: number;
      applied: number;
      skipped: number;
      conflicts: number;
    }>('sync-completed', (event) => {
      const p = event.payload;
      // 构造与本地同步同形的结果（inbound 标记让同步页通用 toast 跳过，避免双弹）
      const result: SyncResult = {
        summary: `examined=${p.examined}, applied=${p.applied}, skipped=${p.skipped}, conflicts=${p.conflicts}`,
        examined: p.examined,
        applied: p.applied,
        skipped: p.skipped,
        conflicts: [],
        per_table: [],
        inbound: true,
      };
      set((state) => ({
        lastResult: result,
        recentResults: [result, ...state.recentResults].slice(0, 10),
      }));
      // 全局完成提醒（B 侧用户不在同步页也能看到）
      useUiStore.getState().showToast({
        type: 'success',
        message:
          i18next.t('settings:sync_completed_inbound', {
            examined: p.examined,
            applied: p.applied,
            skipped: p.skipped,
          }) ?? 'Inbound sync completed',
      });
      // 刷新对端列表与冲突（响应方可能因此产生新冲突）
      get()
        .loadStatus()
        .catch((err) => logger.warn('[syncStore] status refresh after inbound sync:', err));
      if (p.conflicts > 0) {
        get()
          .loadConflicts()
          .catch((err) => logger.warn('[syncStore] conflicts refresh after inbound sync:', err));
      }
    });
  },

  /** 初始化 sync-nsd-failed 事件监听器（移动端 NSD 注册失败）。
   *  后端失败时已回滚为禁用状态，这里重读后端状态并提示错误，
   *  避免开关 UI 仍显示「已启用」与实际状态漂移。
   *  返回 unlisten 函数，调用方应在组件卸载时调用以清理。 */
  initNsdFailedListener: (): Promise<UnlistenFn> => {
    return listen<{ error?: string }>('sync-nsd-failed', (event) => {
      logger.warn('[syncStore] NSD registration failed:', event.payload?.error);
      set({ isLoading: false });
      // 先重读后端状态（后端已回滚为禁用），完成后再设置错误提示，
      // 避免 loadStatus 成功路径的 error: null 把提示清掉。
      get()
        .loadStatus()
        .catch((err) => logger.warn('[syncStore] status resync after nsd failure:', err))
        .finally(() => set({ error: '__SYNC_ERR__:nsd_failed' }));
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
  };
});
