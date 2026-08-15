import { describe, it, expect, vi, beforeEach } from 'vitest';

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
import { useUiStore } from '@/stores/uiStore';

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
    expect(s.error).toContain('__SYNC_ERR__:connect_failed');
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

    // 等待 loadStatus 重读完成并设置错误提示
    await vi.waitFor(() => {
      expect(useSyncStore.getState().error).toBe('__SYNC_ERR__:nsd_failed');
    });
    expect(mockInvoke).toHaveBeenCalledWith('sync_get_status');

    const s = useSyncStore.getState();
    expect(s.isLoading).toBe(false);
    // 重读后端状态后，开关 UI 纠正为禁用，消除状态漂移
    expect(s.syncEnabled).toBe(false);
    expect(unlisten).toBe(mockUnlisten);
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
    expect(s.error).toContain('__SYNC_ERR__:connect_failed');
    // 失败不写 lastResult（不触发「同步完成」toast），但写入失败历史条目
    expect(s.lastResult).toBeNull();
    const entry = s.recentResults[0];
    expect(entry).toBeDefined();
    expect(entry.failed).toBe(true);
    expect(entry.errorSummary).toContain('connect_failed');
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
