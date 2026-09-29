import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { listen } from '@tauri-apps/api/event';
import { useSafSyncStore, type SyncProgressPayload } from './safSyncStore';

vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn() }));

const callbacks: Array<(payload: SyncProgressPayload) => void> = [];
const unlisten = vi.fn();

async function startListening() {
  useSafSyncStore.getState().startListening();
  await Promise.resolve();
}

beforeEach(() => {
  vi.useFakeTimers();
  useSafSyncStore.getState().stopListening();
  useSafSyncStore.getState().reset();
  callbacks.length = 0;
  vi.clearAllMocks();
  vi.mocked(listen).mockImplementation((_event, callback) => {
    callbacks.push((payload) => callback({ payload } as never));
    return Promise.resolve(unlisten);
  });
});

afterEach(() => {
  useSafSyncStore.getState().stopListening();
  vi.useRealTimers();
});

describe('safSyncStore progress listener lifecycle', () => {
  it('后一次完成事件保持完整的三秒可见时间', async () => {
    await startListening();
    callbacks[0]({ phase: 'sync_complete' });
    vi.advanceTimersByTime(1000);
    callbacks[0]({ phase: 'sync_complete' });

    vi.advanceTimersByTime(2000);
    expect(useSafSyncStore.getState().status).toBe('completed');
    vi.advanceTimersByTime(1000);
    expect(useSafSyncStore.getState().status).toBe('idle');
  });

  it('后一次错误事件保持完整的五秒可见时间', async () => {
    await startListening();
    callbacks[0]({ phase: 'error', message: 'first error' });
    vi.advanceTimersByTime(1000);
    callbacks[0]({ phase: 'error', message: 'second error' });

    vi.advanceTimersByTime(4000);
    expect(useSafSyncStore.getState().status).toBe('error');
    expect(useSafSyncStore.getState().error).toBe('second error');
    vi.advanceTimersByTime(1000);
    expect(useSafSyncStore.getState().status).toBe('idle');
    expect(useSafSyncStore.getState().error).toBeNull();
  });

  it('停止监听后旧回调不能改写新监听会话状态', async () => {
    await startListening();
    const oldCallback = callbacks[0];
    useSafSyncStore.getState().stopListening();
    await startListening();
    expect(callbacks).toHaveLength(2);

    oldCallback({ phase: 'error', message: 'old error' });
    expect(useSafSyncStore.getState().status).toBe('idle');
    expect(useSafSyncStore.getState().error).toBeNull();
    callbacks[1]({ phase: 'sync_start' });
    expect(useSafSyncStore.getState().status).toBe('syncing');
  });
});
