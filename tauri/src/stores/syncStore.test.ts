import { describe, it, expect, vi, beforeEach } from 'vitest';

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
