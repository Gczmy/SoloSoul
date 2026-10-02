import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

// 注意：以下 mock 必须在使用 useSyncStore 之前声明（hoisted）。

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

type EventHandler = (event: { payload: unknown }) => void;
const handlers = new Map<string, EventHandler>();
const mockUnlisten = vi.fn();
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(async (name: string, handler: EventHandler) => {
    handlers.set(name, handler);
    return mockUnlisten;
  }),
}));

import { useSyncStore, __resetSyncCompletedMergeForTest } from './syncStore';
import { useAuthStore } from '@/stores/authStore';
import { useUiStore } from '@/stores/uiStore';
import { useObjectStore } from '@/stores/objectStore';
import { useTemplateStore } from '@/stores/templateStore';
import { useTrashStore } from '@/stores/trashStore';
import { useProfileStore } from '@/stores/profileStore';
import type { SyncConflictDetail, SyncConflictSummary } from '@/lib/ipc';

describe('syncStore pairing_pending detection', () => {
  beforeEach(() => {
    handlers.clear();
    mockInvoke.mockReset();
    mockUnlisten.mockClear();
    useSyncStore.setState({
      isLoading: false,
      error: null,
      lastResult: null,
      pairingPendingPeerId: null,
      pairingPendingAddr: null,
      incomingPairingRequest: null,
    });
  });

  it('detects pairing_pending error and enters A-side pairing flow', async () => {
    // 首次 sync_with_device 返回 pairing_pending；随后 loadStatus 返回状态
    mockInvoke
      .mockImplementationOnce(() => Promise.reject('__SYNC_ERR__:pairing_pending:node-B'))
      .mockResolvedValueOnce({
        isDiscovering: false,
        syncEnabled: true,
        autoSyncEnabled: false,
        localFingerprint: 'fp',
        connectedPeers: [
          {
            id: 'node-B',
            name: 'SoloSoul-ab12cd34',
            addr: '10.0.0.2:42069',
            fingerprint: 'ab12cd34',
            trusted: false,
            lastSeen: 'now',
          },
        ],
      });

    await useSyncStore.getState().syncWithDevice('10.0.0.2:42069');

    const s = useSyncStore.getState();
    expect(s.pairingPendingPeerId).toBe('node-B');
    expect(s.pairingPendingAddr).toBe('10.0.0.2:42069');
    expect(s.error).toBeNull();
    expect(s.isLoading).toBe(false);
  });

  it('parses sasCode from new pairing_pending format', async () => {
    // 新后端返回 `{peerId}:{sas}`；sas 应存入 pairingPendingSasCode 供配对卡片展示
    mockInvoke
      .mockImplementationOnce(() => Promise.reject('__SYNC_ERR__:pairing_pending:node-B:482913'))
      .mockResolvedValueOnce({
        isDiscovering: false,
        syncEnabled: true,
        autoSyncEnabled: false,
        localFingerprint: 'fp',
        connectedPeers: [
          {
            id: 'node-B',
            name: 'SoloSoul-ab12cd34',
            addr: '10.0.0.2:42069',
            fingerprint: 'ab12cd34',
            trusted: false,
            lastSeen: 'now',
          },
        ],
      });

    await useSyncStore.getState().syncWithDevice('10.0.0.2:42069');

    const s = useSyncStore.getState();
    expect(s.pairingPendingPeerId).toBe('node-B');
    expect(s.pairingPendingSasCode).toBe('482913');
    expect(s.error).toBeNull();
  });

  it('keeps pairingPendingSasCode null for legacy pairing_pending format', async () => {
    // 旧格式 `{peerId}` 无 sas 部分，sasCode 应为 null（前端回退显示指纹）
    mockInvoke
      .mockImplementationOnce(() => Promise.reject('__SYNC_ERR__:pairing_pending:node-B'))
      .mockResolvedValueOnce({
        isDiscovering: false,
        syncEnabled: true,
        autoSyncEnabled: false,
        localFingerprint: 'fp',
        connectedPeers: [],
      });

    await useSyncStore.getState().syncWithDevice('10.0.0.2:42069');

    const s = useSyncStore.getState();
    expect(s.pairingPendingPeerId).toBe('node-B');
    expect(s.pairingPendingSasCode).toBeNull();
  });

  it('keeps generic error for non-pairing failures', async () => {
    mockInvoke.mockImplementationOnce(() => Promise.reject('__SYNC_ERR__:connect_failed:timeout'));

    await useSyncStore.getState().syncWithDevice('10.0.0.99:42069');

    const s = useSyncStore.getState();
    expect(s.pairingPendingPeerId).toBeNull();
    expect(s.error).toBe('SYNC_CONNECT_FAILED');
  });

  it('clearPairingPending resets A-side flow', () => {
    useSyncStore.setState({ pairingPendingPeerId: 'node-B', pairingPendingAddr: '10.0.0.2:42069' });
    useSyncStore.getState().clearPairingPending();
    expect(useSyncStore.getState().pairingPendingPeerId).toBeNull();
    expect(useSyncStore.getState().pairingPendingAddr).toBeNull();
  });

  it('initPairingRequestListener sets incomingPairingRequest on event', async () => {
    const unlisten = await useSyncStore.getState().initPairingRequestListener();
    const handler = handlers.get('sync-pairing-request');
    expect(handler).toBeDefined();

    handler!({
      payload: {
        nodeId: 'node-A',
        fingerprint: 'aabbccdd11223344',
        addr: '10.0.0.1:42069',
        deviceName: 'SoloSoul-aabbccdd',
        sasCode: '730154',
      },
    });

    const req = useSyncStore.getState().incomingPairingRequest;
    expect(req).not.toBeNull();
    expect(req!.id).toBe('node-A');
    expect(req!.name).toBe('SoloSoul-aabbccdd');
    expect(req!.fingerprint).toBe('aabbccdd11223344');
    expect(req!.sasCode).toBe('730154');
    expect(req!.trusted).toBe(false);

    useSyncStore.getState().clearIncomingPairingRequest();
    expect(useSyncStore.getState().incomingPairingRequest).toBeNull();
    expect(unlisten).toBe(mockUnlisten);
  });

  it('updates sasCode on duplicate nodeId event (new handshake)', async () => {
    const unlisten = await useSyncStore.getState().initPairingRequestListener();
    const handler = handlers.get('sync-pairing-request');
    expect(handler).toBeDefined();

    // 首次事件（握手 H1）
    handler!({
      payload: {
        nodeId: 'node-A',
        fingerprint: 'aa',
        addr: '10.0.0.1:1',
        deviceName: 'n',
        sasCode: '111111',
      },
    });
    // 同一 peer 重连（握手 H2，新验证码）：不重建卡片，仅更新 sasCode
    handler!({
      payload: {
        nodeId: 'node-A',
        fingerprint: 'aa',
        addr: '10.0.0.1:1',
        deviceName: 'n',
        sasCode: '222222',
      },
    });

    const req = useSyncStore.getState().incomingPairingRequest;
    expect(req).not.toBeNull();
    expect(req!.sasCode).toBe('222222');
    expect(req!.id).toBe('node-A');

    useSyncStore.getState().clearIncomingPairingRequest();
    expect(unlisten).toBe(mockUnlisten);
  });
});

describe('syncStore uiPrefsSync toggle', () => {
  beforeEach(() => {
    handlers.clear();
    mockInvoke.mockReset();
    mockUnlisten.mockClear();
    useSyncStore.setState({ uiPrefsSyncEnabled: true, isLoading: false, error: null });
  });

  it('loadUiPrefsSync reads backend toggle', async () => {
    mockInvoke.mockResolvedValueOnce(false);
    await useSyncStore.getState().loadUiPrefsSync();
    expect(mockInvoke).toHaveBeenCalledWith('sync_get_ui_prefs_sync');
    expect(useSyncStore.getState().uiPrefsSyncEnabled).toBe(false);
  });

  it('setUiPrefsSyncEnabled persists toggle and updates state', async () => {
    mockInvoke.mockResolvedValueOnce(false);
    await useSyncStore.getState().setUiPrefsSyncEnabled(false);
    expect(mockInvoke).toHaveBeenCalledWith('sync_set_ui_prefs_sync', { enabled: false });
    expect(useSyncStore.getState().uiPrefsSyncEnabled).toBe(false);
    expect(useSyncStore.getState().isLoading).toBe(false);
  });

  it('关闭成功后忽略开关前开始的迟到外观偏好状态读取', async () => {
    let finishRead!: (enabled: boolean) => void;
    mockInvoke.mockImplementation((command: string) => {
      if (command === 'sync_get_ui_prefs_sync') {
        return new Promise<boolean>((resolve) => {
          finishRead = resolve;
        });
      }
      if (command === 'sync_set_ui_prefs_sync') return Promise.resolve(false);
      throw new Error(`Unexpected command: ${command}`);
    });

    const oldRead = useSyncStore.getState().loadUiPrefsSync();
    await useSyncStore.getState().setUiPrefsSyncEnabled(false);
    finishRead(true);
    await oldRead;

    expect(useSyncStore.getState().uiPrefsSyncEnabled).toBe(false);
    expect(useSyncStore.getState().error).toBeNull();
  });

  it('关闭处理中开始的迟到外观偏好状态读取也不能覆盖成功结果', async () => {
    let finishRead!: (enabled: boolean) => void;
    let finishToggle!: (enabled: boolean) => void;
    mockInvoke.mockImplementation((command: string) => {
      if (command === 'sync_get_ui_prefs_sync') {
        return new Promise<boolean>((resolve) => {
          finishRead = resolve;
        });
      }
      if (command === 'sync_set_ui_prefs_sync') {
        return new Promise<boolean>((resolve) => {
          finishToggle = resolve;
        });
      }
      throw new Error(`Unexpected command: ${command}`);
    });

    const toggle = useSyncStore.getState().setUiPrefsSyncEnabled(false);
    const oldRead = useSyncStore.getState().loadUiPrefsSync();
    finishToggle(false);
    await toggle;
    finishRead(true);
    await oldRead;

    expect(useSyncStore.getState().uiPrefsSyncEnabled).toBe(false);
    expect(useSyncStore.getState().error).toBeNull();
  });
});

describe('syncStore auto-sync toggle state', () => {
  beforeEach(() => {
    mockInvoke.mockReset();
    useSyncStore.setState({ autoSyncEnabled: false, isLoading: false, error: null });
  });

  it('启用成功后忽略开关前开始的迟到状态读取', async () => {
    let finishRead!: (enabled: boolean) => void;
    mockInvoke.mockImplementation((command: string) => {
      if (command === 'sync_get_auto_status') {
        return new Promise<boolean>((resolve) => {
          finishRead = resolve;
        });
      }
      if (command === 'sync_set_auto_enabled') return Promise.resolve(true);
      throw new Error(`Unexpected command: ${command}`);
    });

    const oldRead = useSyncStore.getState().loadAutoSyncStatus();
    await useSyncStore.getState().setAutoSyncEnabled(true);
    finishRead(false);
    await oldRead;

    expect(useSyncStore.getState().autoSyncEnabled).toBe(true);
    expect(useSyncStore.getState().error).toBeNull();
  });

  it('启用处理中开始的迟到状态读取也不能覆盖成功结果', async () => {
    let finishRead!: (enabled: boolean) => void;
    let finishToggle!: (enabled: boolean) => void;
    mockInvoke.mockImplementation((command: string) => {
      if (command === 'sync_get_auto_status') {
        return new Promise<boolean>((resolve) => {
          finishRead = resolve;
        });
      }
      if (command === 'sync_set_auto_enabled') {
        return new Promise<boolean>((resolve) => {
          finishToggle = resolve;
        });
      }
      throw new Error(`Unexpected command: ${command}`);
    });

    const toggle = useSyncStore.getState().setAutoSyncEnabled(true);
    const oldRead = useSyncStore.getState().loadAutoSyncStatus();
    finishToggle(true);
    await toggle;
    finishRead(false);
    await oldRead;

    expect(useSyncStore.getState().autoSyncEnabled).toBe(true);
    expect(useSyncStore.getState().error).toBeNull();
  });
});

describe('syncStore initSyncCompletedListener', () => {
  beforeEach(() => {
    handlers.clear();
    mockInvoke.mockReset();
    mockUnlisten.mockClear();
    // 清空跨窗口合并缓存，模拟新会话窗口（避免上个用例的 node-A 条目污染本用例）
    __resetSyncCompletedMergeForTest();
    useSyncStore.setState({ lastResult: null, recentResults: [] });
  });

  it('records inbound result with counts and shows global toast on sync-completed', async () => {
    // loadStatus（刷新对端）+ loadConflicts（conflicts>0 时刷新冲突）
    mockInvoke
      .mockResolvedValueOnce({
        isDiscovering: false,
        syncEnabled: true,
        autoSyncEnabled: false,
        localFingerprint: '',
        connectedPeers: [],
      })
      .mockResolvedValueOnce([]);
    const toastSpy = vi.spyOn(useUiStore.getState(), 'showToast').mockImplementation(() => {});

    const unlisten = await useSyncStore.getState().initSyncCompletedListener();
    const handler = handlers.get('sync-completed');
    expect(handler).toBeDefined();

    handler!({
      payload: {
        peerNodeId: 'node-A',
        examined: 12,
        applied: 10,
        skipped: 2,
        conflicts: 1,
        outboundRecords: 5,
      },
    });

    const s = useSyncStore.getState();
    // 入站结果写入 lastResult（结果行展示具体条数），inbound 标记避免同步页通用 toast 双弹
    expect(s.lastResult).not.toBeNull();
    expect(s.lastResult!.examined).toBe(12);
    expect(s.lastResult!.applied).toBe(10);
    expect(s.lastResult!.skipped).toBe(2);
    expect(s.lastResult!.conflictCount).toBe(1);
    // B：发回对端条数随结果携带（结果行/历史面板展示完整交换量）
    expect(s.lastResult!.outboundRecords).toBe(5);
    expect(s.lastResult!.inbound).toBe(true);
    // 全局 toast（B 侧不在同步页也能收到）。测试环境 locale 可能未加载
    // sync_completed_inbound 键（t() 返回 key 串），只断言 toast 已触发。
    expect(toastSpy).toHaveBeenCalledTimes(1);
    const arg = toastSpy.mock.calls[0][0] as { type: string };
    expect(arg.type).toBe('success');

    // 冲突刷新路径被触发
    expect(mockInvoke).toHaveBeenCalledWith('sync_list_conflicts');

    toastSpy.mockRestore();
    expect(unlisten).toBe(mockUnlisten);
  });

  it('merges duplicate sync-completed events from same peer within window (C)', async () => {
    // 一次「立即同步」被多个自动同步源叠加触发 → 同一 peer 短窗口内多个事件：
    // 只弹一次 toast、只写一条历史，计数累加（完整交换量）。
    mockInvoke.mockResolvedValue({});
    const toastSpy = vi.spyOn(useUiStore.getState(), 'showToast').mockImplementation(() => {});

    const unlisten = await useSyncStore.getState().initSyncCompletedListener();
    const handler = handlers.get('sync-completed');
    expect(handler).toBeDefined();

    // 事件 1：入站 12 条 + 发回 5 条
    handler!({
      payload: {
        peerNodeId: 'node-A',
        examined: 12,
        applied: 10,
        skipped: 2,
        conflicts: 1,
        outboundRecords: 5,
      },
    });
    // 事件 2：同 peer 窗口内（合并，不重复 toast/历史）——入站 0 条 + 发回 8 条
    handler!({
      payload: {
        peerNodeId: 'node-A',
        examined: 0,
        applied: 0,
        skipped: 0,
        conflicts: 0,
        outboundRecords: 8,
      },
    });

    const s = useSyncStore.getState();
    // 只弹一次 toast
    expect(toastSpy).toHaveBeenCalledTimes(1);
    // 只写一条历史
    expect(s.recentResults.length).toBe(1);
    // 计数累加：入站 12 + 发回 13
    expect(s.lastResult!.examined).toBe(12);
    expect(s.lastResult!.outboundRecords).toBe(13);
    expect(s.lastResult!.conflictCount).toBe(1);

    toastSpy.mockRestore();
    expect(unlisten).toBe(mockUnlisten);
  });

  it('合并事件只有新增写入时才刷新账户数据 Store', async () => {
    const originalLoadObjects = useObjectStore.getState().loadObjects;
    const originalLoadTemplates = useTemplateStore.getState().loadTemplates;
    const originalLoadItems = useTrashStore.getState().loadItems;
    const originalLoadProfile = useProfileStore.getState().loadProfile;
    const loadObjects = vi.fn().mockResolvedValue(undefined);
    const loadTemplates = vi.fn().mockResolvedValue(undefined);
    const loadItems = vi.fn().mockResolvedValue(undefined);
    const loadProfile = vi.fn().mockResolvedValue(undefined);
    const toastSpy = vi.spyOn(useUiStore.getState(), 'showToast').mockImplementation(() => {});
    useObjectStore.setState({ loadObjects });
    useTemplateStore.setState({ loadTemplates });
    useTrashStore.setState({ loadItems });
    useProfileStore.setState({ loadProfile });
    useAuthStore.setState({
      isAuthenticated: true,
      currentAccount: { id: 'account-a', name: 'A' },
    });
    mockInvoke.mockResolvedValue({
      isDiscovering: false,
      syncEnabled: true,
      autoSyncEnabled: false,
      localFingerprint: '',
      connectedPeers: [],
    });

    try {
      await useSyncStore.getState().initSyncCompletedListener();
      const handler = handlers.get('sync-completed')!;
      handler({
        payload: {
          peerNodeId: 'node-A',
          examined: 2,
          applied: 2,
          skipped: 0,
          conflicts: 0,
          outboundRecords: 0,
        },
      });
      handler({
        payload: {
          peerNodeId: 'node-A',
          examined: 0,
          applied: 0,
          skipped: 0,
          conflicts: 0,
          outboundRecords: 1,
        },
      });

      expect(loadObjects).toHaveBeenCalledTimes(1);
      expect(loadObjects).toHaveBeenCalledWith('account-a', undefined);
      expect(loadTemplates).toHaveBeenCalledTimes(1);
      expect(loadItems).toHaveBeenCalledTimes(1);
      expect(loadItems).toHaveBeenCalledWith('account-a');
      expect(loadProfile).toHaveBeenCalledTimes(1);
      expect(loadProfile).toHaveBeenCalledWith('account-a');

      handler({
        payload: {
          peerNodeId: 'node-A',
          examined: 1,
          applied: 1,
          skipped: 0,
          conflicts: 0,
          outboundRecords: 0,
        },
      });
      expect(loadObjects).toHaveBeenCalledTimes(2);
      expect(loadTemplates).toHaveBeenCalledTimes(2);
      expect(loadItems).toHaveBeenCalledTimes(2);
      expect(loadProfile).toHaveBeenCalledTimes(2);
    } finally {
      useAuthStore.setState({ isAuthenticated: false, currentAccount: null });
      useObjectStore.setState({ loadObjects: originalLoadObjects });
      useTemplateStore.setState({ loadTemplates: originalLoadTemplates });
      useTrashStore.setState({ loadItems: originalLoadItems });
      useProfileStore.setState({ loadProfile: originalLoadProfile });
      toastSpy.mockRestore();
    }
  });

  it('skips toast and history for all-zero exchange (C)', async () => {
    // 无实际数据交换的会话（检查/应用/跳过/发回全 0）不弹 toast、不写历史
    const toastSpy = vi.spyOn(useUiStore.getState(), 'showToast').mockImplementation(() => {});

    const unlisten = await useSyncStore.getState().initSyncCompletedListener();
    const handler = handlers.get('sync-completed');
    expect(handler).toBeDefined();

    handler!({
      payload: {
        peerNodeId: 'node-Zero',
        examined: 0,
        applied: 0,
        skipped: 0,
        conflicts: 0,
        outboundRecords: 0,
      },
    });

    const s = useSyncStore.getState();
    expect(toastSpy).not.toHaveBeenCalled();
    expect(s.lastResult).toBeNull();
    expect(s.recentResults.length).toBe(0);

    toastSpy.mockRestore();
    expect(unlisten).toBe(mockUnlisten);
  });

  it('records a nonzero completion after an all-zero event from the same peer', async () => {
    mockInvoke.mockResolvedValue({});
    const toastSpy = vi.spyOn(useUiStore.getState(), 'showToast').mockImplementation(() => {});
    await useSyncStore.getState().initSyncCompletedListener();
    const handler = handlers.get('sync-completed')!;

    handler({
      payload: {
        peerNodeId: 'node-A',
        examined: 0,
        applied: 0,
        skipped: 0,
        conflicts: 0,
        outboundRecords: 0,
      },
    });
    handler({
      payload: {
        peerNodeId: 'node-A',
        examined: 2,
        applied: 1,
        skipped: 1,
        conflicts: 0,
        outboundRecords: 0,
      },
    });

    expect(useSyncStore.getState().lastResult?.applied).toBe(1);
    expect(useSyncStore.getState().recentResults).toHaveLength(1);
    expect(toastSpy).toHaveBeenCalledTimes(1);
    toastSpy.mockRestore();
  });

  it('does not suppress a completion that only reports conflicts', async () => {
    mockInvoke.mockResolvedValue({});
    const toastSpy = vi.spyOn(useUiStore.getState(), 'showToast').mockImplementation(() => {});
    await useSyncStore.getState().initSyncCompletedListener();
    const handler = handlers.get('sync-completed')!;

    handler({
      payload: {
        peerNodeId: 'node-conflict',
        examined: 0,
        applied: 0,
        skipped: 0,
        conflicts: 1,
        outboundRecords: 0,
      },
    });

    expect(useSyncStore.getState().lastResult?.conflictCount).toBe(1);
    expect(useSyncStore.getState().recentResults).toHaveLength(1);
    expect(toastSpy).toHaveBeenCalledTimes(1);
    expect(mockInvoke).toHaveBeenCalledWith('sync_list_conflicts');
    toastSpy.mockRestore();
  });
});

describe('syncStore initNsdFailedListener', () => {
  beforeEach(() => {
    handlers.clear();
    mockInvoke.mockReset();
    mockUnlisten.mockClear();
    // 模拟 NSD 注册失败前的漂移状态：开关显示已启用且仍在加载
    useSyncStore.setState({ isLoading: true, error: null, syncEnabled: true });
  });

  it('resets loading, sets localized error code and reloads backend status on sync-nsd-failed', async () => {
    // 后端已回滚为禁用，loadStatus 应读到禁用状态
    mockInvoke.mockResolvedValue({
      isDiscovering: false,
      syncEnabled: false,
      autoSyncEnabled: false,
      localFingerprint: '',
      connectedPeers: [],
    });

    const unlisten = await useSyncStore.getState().initNsdFailedListener();
    const handler = handlers.get('sync-nsd-failed');
    expect(handler).toBeDefined();

    handler!({ payload: { error: 'register failed' } });

    // 错误立即可见；等待 loadStatus 重读后端状态完成
    await vi.waitFor(() => {
      expect(useSyncStore.getState().error).toBe('SYNC_DISCOVERY_FAILED');
      expect(useSyncStore.getState().syncEnabled).toBe(false);
    });
    expect(mockInvoke).toHaveBeenCalledWith('sync_get_status');

    const s = useSyncStore.getState();
    expect(s.isLoading).toBe(false);
    // 重读后端状态后，开关 UI 纠正为禁用，消除状态漂移
    expect(s.syncEnabled).toBe(false);
    expect(unlisten).toBe(mockUnlisten);
  });

  it('does not restore an obsolete NSD error after a newer status refresh', async () => {
    let resolveOldStatus!: (status: {
      isDiscovering: boolean;
      syncEnabled: boolean;
      autoSyncEnabled: boolean;
      localFingerprint: string;
      connectedPeers: never[];
    }) => void;
    const oldStatus = new Promise<Parameters<typeof resolveOldStatus>[0]>((resolve) => {
      resolveOldStatus = resolve;
    });
    mockInvoke.mockReturnValueOnce(oldStatus).mockResolvedValueOnce({
      isDiscovering: false,
      syncEnabled: true,
      autoSyncEnabled: false,
      localFingerprint: 'current',
      connectedPeers: [],
    });

    await useSyncStore.getState().initNsdFailedListener();
    handlers.get('sync-nsd-failed')!({ payload: { error: 'old registration failure' } });
    await useSyncStore.getState().loadStatus();
    expect(useSyncStore.getState().error).toBeNull();

    resolveOldStatus({
      isDiscovering: false,
      syncEnabled: false,
      autoSyncEnabled: false,
      localFingerprint: '',
      connectedPeers: [],
    });
    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(useSyncStore.getState().syncEnabled).toBe(true);
    expect(useSyncStore.getState().error).toBeNull();
  });

  it('keeps the NSD failure reason when the status refresh also fails', async () => {
    mockInvoke.mockRejectedValue(new Error('status unavailable'));

    await useSyncStore.getState().initNsdFailedListener();
    handlers.get('sync-nsd-failed')!({ payload: { error: 'register failed' } });
    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(mockInvoke).toHaveBeenCalledWith('sync_get_status');
    expect(useSyncStore.getState().error).toBe('SYNC_DISCOVERY_FAILED');
  });
});

describe('syncStore activity stamping (timestamp + peer info + failure record)', () => {
  beforeEach(() => {
    handlers.clear();
    mockInvoke.mockReset();
    mockUnlisten.mockClear();
    useSyncStore.setState({
      isLoading: false,
      error: null,
      lastResult: null,
      recentResults: [],
      connectedPeers: [],
    });
  });

  it('stamps local timestamp and resolved peer info on manual sync success', async () => {
    const before = Date.now();
    mockInvoke
      .mockResolvedValueOnce({
        summary: 'examined=3, applied=2, skipped=1, conflicts=0',
        examined: 3,
        applied: 2,
        skipped: 1,
        conflicts: [],
        per_table: [{ table: 'object', examined: 3, applied: 2, skipped: 1 }],
      })
      .mockResolvedValueOnce({
        isDiscovering: false,
        syncEnabled: true,
        autoSyncEnabled: false,
        localFingerprint: 'fp',
        connectedPeers: [
          {
            id: 'node-B',
            name: 'SoloSoul-ab12cd34',
            addr: '10.0.0.2:42069',
            fingerprint: 'ab12cd34',
            trusted: true,
            lastSeen: 'now',
            clientType: 'windows',
          },
        ],
      })
      .mockResolvedValueOnce([]);

    await useSyncStore.getState().syncWithDevice('10.0.0.2:42069');

    const entry = useSyncStore.getState().recentResults[0];
    expect(entry).toBeDefined();
    // 本地时间戳盖章（记录时刻）
    expect(entry.at).toBeGreaterThanOrEqual(before);
    expect(entry.at).toBeLessThanOrEqual(Date.now());
    // 对端信息从 loadStatus 刷新后的 connectedPeers 解析并固化
    expect(entry.peerName).toBe('SoloSoul-ab12cd34');
    expect(entry.peerClientType).toBe('windows');
    expect(entry.peerNodeId).toBe('10.0.0.2:42069');
    expect(entry.failed).toBeFalsy();
    expect(useSyncStore.getState().lastResult).not.toBeNull();
  });

  it('records a failed history entry on generic sync error (timestamp + peer info)', async () => {
    // 失败路径不走 loadStatus，设备信息从当前 connectedPeers 解析
    useSyncStore.setState({
      connectedPeers: [
        {
          id: 'node-B',
          name: 'SoloSoul-ab12cd34',
          addr: '10.0.0.2:42069',
          fingerprint: 'ab12cd34',
          trusted: true,
          lastSeen: 'now',
          clientType: 'android',
        },
      ],
    });
    mockInvoke.mockImplementationOnce(() => Promise.reject('__SYNC_ERR__:connect_failed:timeout'));

    await useSyncStore.getState().syncWithDevice('10.0.0.2:42069');

    const s = useSyncStore.getState();
    expect(s.error).toBe('SYNC_CONNECT_FAILED');
    // 失败不写 lastResult（不触发「同步完成」toast），但写入失败历史条目
    expect(s.lastResult).toBeNull();
    const entry = s.recentResults[0];
    expect(entry).toBeDefined();
    expect(entry.failed).toBe(true);
    expect(entry.errorSummary).toBe('SYNC_CONNECT_FAILED');
    expect(entry.at).toBeGreaterThan(0);
    expect(entry.peerName).toBe('SoloSoul-ab12cd34');
    expect(entry.peerClientType).toBe('android');
  });

  it('stamps timestamp + peer info on inbound sync-completed event', async () => {
    // 事件收到前 connectedPeers 已含对端（本端曾与对端同步/loadStatus 加载过）
    useSyncStore.setState({
      connectedPeers: [
        {
          id: 'node-A',
          name: 'SoloSoul-11223344',
          addr: '10.0.0.1:42069',
          fingerprint: '11223344',
          trusted: true,
          lastSeen: 'now',
          clientType: 'macos',
        },
      ],
    });
    mockInvoke.mockResolvedValue([]);

    const unlisten = await useSyncStore.getState().initSyncCompletedListener();
    const handler = handlers.get('sync-completed');
    expect(handler).toBeDefined();

    const before = Date.now();
    handler!({
      payload: {
        peerNodeId: 'node-A',
        examined: 5,
        applied: 4,
        skipped: 1,
        conflicts: 0,
        outboundRecords: 2,
      },
    });

    const entry = useSyncStore.getState().recentResults[0];
    expect(entry).toBeDefined();
    expect(entry.inbound).toBe(true);
    expect(entry.at).toBeGreaterThanOrEqual(before);
    expect(entry.peerName).toBe('SoloSoul-11223344');
    expect(entry.peerClientType).toBe('macos');
    expect(entry.peerNodeId).toBe('node-A');
    expect(unlisten).toBe(mockUnlisten);
  });
});

describe('syncStore history self-healing truncation (P028)', () => {
  const accountHistoryKey = 'solosoul.syncHistory.v2.account-a';

  beforeEach(() => {
    localStorage.clear();
    mockInvoke.mockReset();
    mockUnlisten.mockClear();
  });

  it('truncates over-limit persisted history on store load and writes back', async () => {
    // 模拟旧版本（当时无清理逻辑）写入的 15 条超限历史。历史数组头部为最新
    // （pushSyncHistory 前插），故 15 条按 examined 14→0 递减排列。新版本在
    // store 创建时 loadSyncHistory 读取即按 SYNC_HISTORY_MAX(10) 截断并写回，
    // 避免 localStorage 残留永久垃圾（仅 slice 不写回会在每次重启后重复加载
    // 同样的超限旧数据）。
    const oversized = Array.from({ length: 15 }, (_, i) => ({
      summary: `examined=${14 - i}`,
      examined: 14 - i,
      applied: 0,
      skipped: 0,
      conflicts: [],
      per_table: [],
      at: 1_700_000_000_000 + (14 - i),
    }));
    localStorage.setItem(accountHistoryKey, JSON.stringify(oversized));

    // 重新加载模块，并在已解锁账户下触发 store 创建。
    vi.resetModules();
    const { useAuthStore: reloadedAuthStore } = await import('./authStore');
    reloadedAuthStore.setState({
      isAuthenticated: true,
      currentAccount: { id: 'account-a', name: 'A' },
    });
    const { useSyncStore: reloadedStore } = await import('./syncStore');

    // 内存态截断为上限
    expect(reloadedStore.getState().recentResults).toHaveLength(10);
    // 写回后 localStorage 同步截断（不再残留超限数据）
    const persisted: unknown = JSON.parse(localStorage.getItem(accountHistoryKey) ?? 'null');
    expect(Array.isArray(persisted)).toBe(true);
    expect((persisted as unknown[]).length).toBe(10);
    // 保留最新前 10 条（数组头部为最新）
    expect((persisted as Array<{ examined: number }>)[0].examined).toBe(14);
    expect((persisted as Array<{ examined: number }>)[9].examined).toBe(5);
  });

  it('keeps within-limit persisted history untouched', async () => {
    const within = Array.from({ length: 3 }, (_, i) => ({
      summary: `examined=${i}`,
      examined: i,
      applied: 0,
      skipped: 0,
      conflicts: [],
      per_table: [],
      at: 1_700_000_000_000 + i,
    }));
    localStorage.setItem(accountHistoryKey, JSON.stringify(within));

    vi.resetModules();
    const { useAuthStore: reloadedAuthStore } = await import('./authStore');
    reloadedAuthStore.setState({
      isAuthenticated: true,
      currentAccount: { id: 'account-a', name: 'A' },
    });
    const { useSyncStore: reloadedStore } = await import('./syncStore');

    expect(reloadedStore.getState().recentResults).toHaveLength(3);
    // 未超限不触发写回（值与原写入一致，仅需无超限残留）
    const persisted: unknown = JSON.parse(localStorage.getItem(accountHistoryKey) ?? 'null');
    expect((persisted as unknown[]).length).toBe(3);
  });
});

describe('syncStore loadConflicts 异常数据归一化（防整页白屏）', () => {
  beforeEach(() => {
    handlers.clear();
    mockInvoke.mockReset();
    mockUnlisten.mockClear();
    useSyncStore.setState({ conflicts: [], error: null });
  });

  it.each([
    ['undefined', undefined],
    ['null', null],
    ['非数组对象', {}],
  ])('后端返回 %s 时 conflicts 归一化为空数组', async (_label, bad) => {
    mockInvoke.mockResolvedValueOnce(bad);
    await useSyncStore.getState().loadConflicts();
    expect(mockInvoke).toHaveBeenCalledWith('sync_list_conflicts');
    // 关键不变量：conflicts 恒为数组——GlobalSyncIndicator 等消费点
    // 的 `.length` 不会因异常数据抛 TypeError 整页白屏
    expect(useSyncStore.getState().conflicts).toEqual([]);
    expect(useSyncStore.getState().error).toBeNull();
  });

  it('后端返回正常数组时原样保留', async () => {
    const list = [
      {
        id: 'c1',
        table: 'objects',
        record_id: 'obj-1',
        local_hlc: { wall_time_ms: 1, counter: 1, node_id: 'a' },
        remote_hlc: { wall_time_ms: 2, counter: 1, node_id: 'b' },
        winner: 'remote',
        created_at: '2026-01-01T00:00:00Z',
      },
    ];
    mockInvoke.mockResolvedValueOnce(list);
    await useSyncStore.getState().loadConflicts();
    expect(useSyncStore.getState().conflicts).toEqual(list);
    expect(useSyncStore.getState().error).toBeNull();
  });
});

describe('syncStore conflict detail and resolution lifecycle', () => {
  const summary: SyncConflictSummary = {
    id: 'conflict-1',
    table: 'objects',
    record_id: 'obj-1',
    local_hlc: { wall_time_ms: 1, counter: 0, node_id: 'local' },
    remote_hlc: { wall_time_ms: 2, counter: 0, node_id: 'remote' },
    winner: 'remote',
    created_at: '2026-01-01T00:00:00Z',
  };
  const detail: SyncConflictDetail = {
    ...summary,
    local_data: { name: 'local' },
    remote_data: { name: 'remote' },
    remote_deleted: false,
  };

  beforeEach(() => {
    mockInvoke.mockReset();
    useSyncStore.setState({
      conflicts: [summary],
      selectedConflict: detail,
      isLoading: false,
      error: null,
    });
  });

  it('loads the selected detail using its conflict ID', async () => {
    useSyncStore.setState({ selectedConflict: null });
    mockInvoke.mockResolvedValueOnce(detail);

    await useSyncStore.getState().loadConflictDetail('conflict-1');

    expect(mockInvoke).toHaveBeenCalledWith('sync_get_conflict_detail', {
      conflictId: 'conflict-1',
    });
    expect(useSyncStore.getState().selectedConflict).toEqual(detail);
  });

  it('treats keep_local returning false as a successful resolution and reloads conflicts', async () => {
    // Host 的 false 表示没有应用远端值，仍是成功的 keep_local 结果。
    mockInvoke.mockResolvedValueOnce(false).mockResolvedValueOnce([]);

    await useSyncStore.getState().resolveConflict('conflict-1', 'keep_local');

    expect(mockInvoke).toHaveBeenNthCalledWith(1, 'sync_resolve_conflict', {
      conflictId: 'conflict-1',
      strategy: 'keep_local',
    });
    expect(mockInvoke).toHaveBeenNthCalledWith(2, 'sync_list_conflicts');
    expect(useSyncStore.getState()).toMatchObject({
      conflicts: [],
      selectedConflict: null,
      isLoading: false,
      error: null,
    });
  });

  it('keeps the detail available when the Host rejects resolution', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('permission denied'));

    await useSyncStore.getState().resolveConflict('conflict-1', 'keep_remote');

    expect(mockInvoke).toHaveBeenCalledTimes(1);
    expect(useSyncStore.getState()).toMatchObject({
      conflicts: [summary],
      selectedConflict: detail,
      isLoading: false,
      error: 'SYNC_CONFLICT_FAILED',
    });
  });

  it('does not reload conflicts or restore a late result after vault lock', async () => {
    let finish!: (appliedRemote: boolean) => void;
    mockInvoke.mockImplementationOnce(
      () =>
        new Promise<boolean>((resolve) => {
          finish = resolve;
        }),
    );
    const resolving = useSyncStore.getState().resolveConflict('conflict-1', 'keep_local');

    useSyncStore.getState().clearOnVaultLock();
    finish(false);
    await resolving;

    expect(mockInvoke).toHaveBeenCalledTimes(1);
    expect(useSyncStore.getState()).toMatchObject({
      conflicts: [],
      selectedConflict: null,
      isLoading: false,
      error: null,
    });
  });
});

describe('syncStore encrypted device names', () => {
  const peer = {
    id: 'peer',
    name: 'original',
    addr: '',
    fingerprint: '12345678',
    trusted: true,
    lastSeen: '',
  };
  beforeEach(() => {
    mockInvoke.mockReset();
    useSyncStore.setState({ connectedPeers: [peer] });
  });
  it('updates the visible alias only after the vault confirms saving', async () => {
    mockInvoke.mockResolvedValueOnce('Work Mac');
    await useSyncStore.getState().renamePeer('peer', 'Work Mac');
    expect(mockInvoke).toHaveBeenCalledWith('sync_rename_peer', {
      peerNodeId: 'peer',
      name: 'Work Mac',
    });
    expect(useSyncStore.getState().connectedPeers[0]).toEqual({ ...peer, customName: 'Work Mac' });
  });
  it('ignores a stale status response that started before the rename', async () => {
    let finish!: (status: unknown) => void;
    mockInvoke.mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          finish = resolve;
        }),
    );
    const loading = useSyncStore.getState().loadStatus();
    mockInvoke.mockResolvedValueOnce('Work Mac');
    await useSyncStore.getState().renamePeer('peer', 'Work Mac');
    finish({ connectedPeers: [peer] });
    await loading;
    expect(useSyncStore.getState().connectedPeers[0].customName).toBe('Work Mac');
  });

  it('propagates save failures and keeps the existing name', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('disk full'));
    await expect(useSyncStore.getState().renamePeer('peer', 'Work Mac')).rejects.toThrow(
      'SYNC_WRITE_FAILED',
    );
    expect(useSyncStore.getState().connectedPeers[0]).toEqual(peer);
  });
  it('rejects a late save after locking without repopulating device data', async () => {
    let finish!: (name: string) => void;
    mockInvoke.mockImplementationOnce(
      () =>
        new Promise<string>((resolve) => {
          finish = resolve;
        }),
    );
    const saving = useSyncStore.getState().renamePeer('peer', 'Work Mac');
    const result = expect(saving).rejects.toThrow('expired session');
    useSyncStore.getState().clearOnVaultLock();
    finish('Work Mac');
    await result;
    expect(useSyncStore.getState().connectedPeers).toEqual([]);
  });
});

describe('syncStore peer mutation feedback', () => {
  beforeEach(() => {
    mockInvoke.mockReset();
    useSyncStore.setState({
      isLoading: false,
      error: null,
      connectedPeers: [
        {
          id: 'peer-1',
          name: 'Old Mac',
          addr: '192.0.2.1:42069',
          fingerprint: 'aabbccdd',
          trusted: false,
          lastSeen: '',
        },
      ],
    });
  });

  it('信任失败向调用方传播错误并保留设备', async () => {
    const failure = new Error('trust denied');
    mockInvoke.mockRejectedValueOnce(failure);

    await expect(
      useSyncStore.getState().trustPeer('peer-1', true, 'aabbccdd'),
    ).rejects.toMatchObject({ backend: { code: 'SYNC_WRITE_FAILED' } });
    expect(mockInvoke).toHaveBeenCalledWith('sync_trust_peer', {
      peerNodeId: 'peer-1',
      trusted: true,
      fingerprint: 'aabbccdd',
    });
    expect(useSyncStore.getState().connectedPeers[0].trusted).toBe(false);
    expect(useSyncStore.getState().error).toBe('SYNC_WRITE_FAILED');
    expect(useSyncStore.getState().isLoading).toBe(false);
  });

  it('信任成功后按后端状态刷新受信任设备', async () => {
    const trustedPeer = { ...useSyncStore.getState().connectedPeers[0], trusted: true };
    mockInvoke.mockResolvedValueOnce(undefined).mockResolvedValueOnce({
      isDiscovering: false,
      syncEnabled: true,
      autoSyncEnabled: false,
      localFingerprint: '',
      connectedPeers: [trustedPeer],
    });

    await useSyncStore.getState().trustPeer('peer-1', true, 'aabbccdd');
    expect(mockInvoke).toHaveBeenNthCalledWith(1, 'sync_trust_peer', {
      peerNodeId: 'peer-1',
      trusted: true,
      fingerprint: 'aabbccdd',
    });
    expect(mockInvoke).toHaveBeenNthCalledWith(2, 'sync_get_status');
    expect(useSyncStore.getState().connectedPeers[0].trusted).toBe(true);
    expect(useSyncStore.getState().isLoading).toBe(false);
  });

  it('忘记失败向调用方传播错误并保留设备', async () => {
    const failure = new Error('forget denied');
    mockInvoke.mockRejectedValueOnce(failure);

    await expect(useSyncStore.getState().forgetPeer('peer-1')).rejects.toMatchObject({
      backend: { code: 'SYNC_WRITE_FAILED' },
    });
    expect(mockInvoke).toHaveBeenCalledWith('sync_forget_peer', { peerNodeId: 'peer-1' });
    expect(useSyncStore.getState().connectedPeers[0].id).toBe('peer-1');
    expect(useSyncStore.getState().error).toBe('SYNC_WRITE_FAILED');
    expect(useSyncStore.getState().isLoading).toBe(false);
  });

  it('忘记成功后按后端状态移除设备', async () => {
    mockInvoke.mockResolvedValueOnce(undefined).mockResolvedValueOnce({
      isDiscovering: false,
      syncEnabled: true,
      autoSyncEnabled: false,
      localFingerprint: '',
      connectedPeers: [],
    });

    await useSyncStore.getState().forgetPeer('peer-1');
    expect(mockInvoke).toHaveBeenNthCalledWith(1, 'sync_forget_peer', {
      peerNodeId: 'peer-1',
    });
    expect(mockInvoke).toHaveBeenNthCalledWith(2, 'sync_get_status');
    expect(useSyncStore.getState().connectedPeers).toEqual([]);
    expect(useSyncStore.getState().isLoading).toBe(false);
  });

  it('锁定后迟到的信任结果不能被当作成功', async () => {
    let finishTrust!: () => void;
    mockInvoke.mockImplementationOnce(
      () =>
        new Promise<void>((resolve) => {
          finishTrust = resolve;
        }),
    );
    const pending = useSyncStore.getState().trustPeer('peer-1', true, 'aabbccdd');
    const rejection = expect(pending).rejects.toThrow('expired session');
    useSyncStore.getState().clearOnVaultLock();
    finishTrust();
    await rejection;
    expect(useSyncStore.getState().connectedPeers).toEqual([]);
    expect(mockInvoke).toHaveBeenCalledTimes(1);
  });
});

describe('syncStore listen address after disabling sync', () => {
  beforeEach(() => {
    mockInvoke.mockReset();
    useSyncStore.setState({
      syncEnabled: true,
      listenAddr: '192.0.2.1:42069',
      isLoading: false,
      error: null,
    });
  });

  it('does not restore a late address read after sync has been disabled', async () => {
    let resolveAddress!: (value: string) => void;
    mockInvoke.mockImplementation((command: string) => {
      if (command === 'sync_listen_addr') {
        return new Promise<string>((resolve) => {
          resolveAddress = resolve;
        });
      }
      if (command === 'sync_enable') return Promise.resolve();
      if (command === 'sync_get_status') {
        return Promise.resolve({
          isDiscovering: false,
          syncEnabled: false,
          autoSyncEnabled: false,
          localFingerprint: '',
          connectedPeers: [],
        });
      }
      throw new Error(`Unexpected command: ${command}`);
    });

    const loadingAddress = useSyncStore.getState().loadListenAddr();
    await useSyncStore.getState().enable(false);
    expect(useSyncStore.getState()).toMatchObject({ syncEnabled: false, listenAddr: '' });

    resolveAddress('192.0.2.1:42069');
    await loadingAddress;

    expect(useSyncStore.getState()).toMatchObject({ syncEnabled: false, listenAddr: '' });
  });
});

describe('syncStore backend-disabled status', () => {
  beforeEach(() => {
    mockInvoke.mockReset();
    useSyncStore.setState({
      syncEnabled: true,
      listenAddr: '192.0.2.1:42069',
      discoveredDevices: [{ name: 'Nearby Mac', host: 'mac.local', port: 42069, addresses: [] }],
      isDiscoveringDevices: true,
      error: null,
    });
  });

  it('removes stale discovery data when a status refresh reports sync disabled', async () => {
    mockInvoke.mockResolvedValueOnce({
      isDiscovering: false,
      syncEnabled: false,
      autoSyncEnabled: false,
      localFingerprint: '',
      connectedPeers: [],
    });

    await useSyncStore.getState().loadStatus();

    expect(mockInvoke).toHaveBeenCalledWith('sync_get_status');
    expect(useSyncStore.getState()).toMatchObject({
      syncEnabled: false,
      listenAddr: '',
      discoveredDevices: [],
      isDiscoveringDevices: false,
    });
  });
});

describe('syncStore enable response follows backend status', () => {
  const nearbyDevice = { name: 'Nearby Mac', host: 'mac.local', port: 42069, addresses: [] };

  beforeEach(() => {
    mockInvoke.mockReset();
    useSyncStore.setState({
      syncEnabled: true,
      discoveredDevices: [nearbyDevice],
      listenAddr: '192.0.2.1:42069',
      isDiscoveringDevices: false,
      isLoading: false,
      error: null,
    });
  });

  it('does not discover devices when an enable request leaves the backend disabled', async () => {
    mockInvoke.mockImplementation((command: string) => {
      if (command === 'sync_enable') return Promise.resolve();
      if (command === 'sync_get_status') {
        return Promise.resolve({
          isDiscovering: false,
          syncEnabled: false,
          autoSyncEnabled: false,
          localFingerprint: '',
          connectedPeers: [],
        });
      }
      if (command === 'mdns_discover') return Promise.resolve([]);
      if (command === 'sync_listen_addr') return Promise.resolve('192.0.2.1:42069');
      throw new Error(`Unexpected command: ${command}`);
    });

    await useSyncStore.getState().enable(true);

    expect(useSyncStore.getState()).toMatchObject({
      syncEnabled: false,
      discoveredDevices: [],
      listenAddr: '',
      isDiscoveringDevices: false,
    });
    expect(mockInvoke).not.toHaveBeenCalledWith('mdns_discover', expect.anything());
    expect(mockInvoke).not.toHaveBeenCalledWith('sync_listen_addr');
  });

  it('retains discovery data when a disable request leaves the backend enabled', async () => {
    mockInvoke.mockImplementation((command: string) => {
      if (command === 'sync_enable') return Promise.resolve();
      if (command === 'sync_get_status') {
        return Promise.resolve({
          isDiscovering: false,
          syncEnabled: true,
          autoSyncEnabled: false,
          localFingerprint: '',
          connectedPeers: [],
        });
      }
      if (command === 'mdns_discover') return Promise.resolve([nearbyDevice]);
      if (command === 'sync_listen_addr') return Promise.resolve('192.0.2.1:42069');
      throw new Error(`Unexpected command: ${command}`);
    });

    await useSyncStore.getState().enable(false);

    expect(useSyncStore.getState().syncEnabled).toBe(true);
    expect(useSyncStore.getState().discoveredDevices).toEqual([nearbyDevice]);
    expect(useSyncStore.getState().listenAddr).toBe('192.0.2.1:42069');
  });
});

describe('syncStore status refresh during enable', () => {
  beforeEach(() => {
    mockInvoke.mockReset();
    useSyncStore.setState({ syncEnabled: true, isLoading: false, error: null });
  });

  it('does not let an older status read undo a successful disable', async () => {
    const enabledStatus = {
      isDiscovering: false,
      syncEnabled: true,
      autoSyncEnabled: false,
      localFingerprint: '',
      connectedPeers: [],
    };
    let resolveOldStatus!: (status: typeof enabledStatus) => void;
    const oldStatus = new Promise<typeof enabledStatus>((resolve) => {
      resolveOldStatus = resolve;
    });
    let statusReads = 0;
    mockInvoke.mockImplementation((command: string) => {
      if (command === 'sync_enable') return Promise.resolve();
      if (command === 'sync_get_status') {
        statusReads += 1;
        return statusReads === 1
          ? oldStatus
          : Promise.resolve({ ...enabledStatus, syncEnabled: false });
      }
      throw new Error(`Unexpected command: ${command}`);
    });

    const firstRefresh = useSyncStore.getState().loadStatus();
    await useSyncStore.getState().enable(false);
    expect(useSyncStore.getState().syncEnabled).toBe(false);

    resolveOldStatus(enabledStatus);
    await firstRefresh;

    expect(useSyncStore.getState().syncEnabled).toBe(false);
  });
});

describe('syncStore enable failure feedback', () => {
  beforeEach(() => {
    mockInvoke.mockReset();
    useSyncStore.setState({ syncEnabled: true, isLoading: false, error: null });
  });

  it('keeps the enable error visible after refreshing the actual backend status', async () => {
    mockInvoke.mockImplementation((command: string) => {
      if (command === 'sync_enable') return Promise.reject(new Error('NSD permission denied'));
      if (command === 'sync_get_status') {
        return Promise.resolve({
          isDiscovering: false,
          syncEnabled: false,
          autoSyncEnabled: false,
          localFingerprint: '',
          connectedPeers: [],
        });
      }
      throw new Error(`Unexpected command: ${command}`);
    });

    await useSyncStore.getState().enable(true);
    await vi.waitFor(() => expect(useSyncStore.getState().syncEnabled).toBe(false));

    expect(useSyncStore.getState().isLoading).toBe(false);
    expect(useSyncStore.getState().error).toBe('SYNC_ENABLE_FAILED');
  });

  it('clears an older error during an ordinary successful status refresh', async () => {
    useSyncStore.setState({ error: 'previous failure' });
    mockInvoke.mockResolvedValueOnce({
      isDiscovering: false,
      syncEnabled: true,
      autoSyncEnabled: false,
      localFingerprint: '',
      connectedPeers: [],
    });

    await useSyncStore.getState().loadStatus();

    expect(useSyncStore.getState().error).toBeNull();
  });
});

describe('RF-1028 sync history account isolation', () => {
  const entry = (summary: string) => ({
    summary,
    examined: 1,
    applied: 1,
    skipped: 0,
    conflicts: [],
    per_table: [],
    at: 1_700_000_000_000,
  });

  beforeEach(() => {
    useAuthStore.setState({ isAuthenticated: false, currentAccount: null });
    useSyncStore.getState().clearOnVaultLock();
    localStorage.clear();
    mockInvoke.mockReset();
  });

  afterEach(() => {
    useAuthStore.setState({ isAuthenticated: false, currentAccount: null });
    localStorage.clear();
  });

  it('restores only the unlocked account history and never displays the ambiguous legacy history', () => {
    localStorage.setItem('solosoul.syncHistory.v1', JSON.stringify([entry('legacy account')]));
    localStorage.setItem('solosoul.syncHistory.v2.account-a', JSON.stringify([entry('Alice')]));
    localStorage.setItem('solosoul.syncHistory.v2.account-b', JSON.stringify([entry('Bob')]));

    useAuthStore.setState({
      isAuthenticated: true,
      currentAccount: { id: 'account-a', name: 'A' },
    });
    expect(useSyncStore.getState().recentResults.map((result) => result.summary)).toEqual([
      'Alice',
    ]);

    useAuthStore.setState({ currentAccount: { id: 'account-b', name: 'B' } });
    expect(useSyncStore.getState().recentResults.map((result) => result.summary)).toEqual(['Bob']);

    useAuthStore.setState({ currentAccount: { id: 'account-c', name: 'C' } });
    expect(useSyncStore.getState().recentResults).toEqual([]);
    expect(localStorage.getItem('solosoul.syncHistory.v1')).not.toBeNull();
  });

  it('persists a completed sync under its account and restores it after locking', async () => {
    useAuthStore.setState({
      isAuthenticated: true,
      currentAccount: { id: 'account-a', name: 'A' },
    });
    mockInvoke.mockImplementation((command: string) => {
      if (command === 'sync_with_device') return Promise.resolve(entry('Synced with B'));
      if (command === 'sync_get_status') {
        return Promise.resolve({
          isDiscovering: false,
          syncEnabled: true,
          autoSyncEnabled: false,
          localFingerprint: 'fingerprint-a',
          connectedPeers: [],
        });
      }
      if (command === 'sync_list_conflicts') return Promise.resolve([]);
      throw new Error(`Unexpected command: ${command}`);
    });

    await useSyncStore.getState().syncWithDevice('node-b');
    const persisted = JSON.parse(
      localStorage.getItem('solosoul.syncHistory.v2.account-a') ?? 'null',
    ) as Array<{ summary: string }> | null;
    expect(persisted?.[0].summary).toBe('Synced with B');
    expect(localStorage.getItem('solosoul.syncHistory.v1')).toBeNull();

    useAuthStore.setState({ isAuthenticated: false, currentAccount: null });
    expect(useSyncStore.getState().recentResults).toEqual([]);
    useAuthStore.setState({
      isAuthenticated: true,
      currentAccount: { id: 'account-a', name: 'A' },
    });
    expect(useSyncStore.getState().recentResults[0].summary).toBe('Synced with B');
  });
});
