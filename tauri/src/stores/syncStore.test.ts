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

import { useSyncStore } from './syncStore';

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
      .mockImplementationOnce(() =>
        Promise.reject('__SYNC_ERR__:pairing_pending:node-B'),
      )
      .mockResolvedValueOnce({
        isDiscovering: false,
        syncEnabled: true,
        autoSyncEnabled: false,
        localFingerprint: 'fp',
        connectedPeers: [
          { id: 'node-B', name: 'SoloSoul-ab12cd34', addr: '10.0.0.2:42069', fingerprint: 'ab12cd34', trusted: false, lastSeen: 'now' },
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
      .mockImplementationOnce(() =>
        Promise.reject('__SYNC_ERR__:pairing_pending:node-B:482913'),
      )
      .mockResolvedValueOnce({
        isDiscovering: false,
        syncEnabled: true,
        autoSyncEnabled: false,
        localFingerprint: 'fp',
        connectedPeers: [
          { id: 'node-B', name: 'SoloSoul-ab12cd34', addr: '10.0.0.2:42069', fingerprint: 'ab12cd34', trusted: false, lastSeen: 'now' },
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
      .mockImplementationOnce(() =>
        Promise.reject('__SYNC_ERR__:pairing_pending:node-B'),
      )
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

    handler!({ payload: { nodeId: 'node-A', fingerprint: 'aabbccdd11223344', addr: '10.0.0.1:42069', deviceName: 'SoloSoul-aabbccdd', sasCode: '730154' } });

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
    handler!({ payload: { nodeId: 'node-A', fingerprint: 'aa', addr: '10.0.0.1:1', deviceName: 'n', sasCode: '111111' } });
    // 同一 peer 重连（握手 H2，新验证码）：不重建卡片，仅更新 sasCode
    handler!({ payload: { nodeId: 'node-A', fingerprint: 'aa', addr: '10.0.0.1:1', deviceName: 'n', sasCode: '222222' } });

    const req = useSyncStore.getState().incomingPairingRequest;
    expect(req).not.toBeNull();
    expect(req!.sasCode).toBe('222222');
    expect(req!.id).toBe('node-A');

    useSyncStore.getState().clearIncomingPairingRequest();
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
