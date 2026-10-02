import { beforeEach, afterEach, it, expect, vi } from 'vitest';
import { render, act, cleanup } from '@testing-library/react';
import { listen } from '@tauri-apps/api/event';
import { GlobalSyncIndicator } from './GlobalSyncIndicator';
import { useUiStore } from '@/stores/uiStore';
import { useSyncStore } from '@/stores/syncStore';
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn() }));
const callbacks = new Map<string, (event: { payload: unknown }) => void>();
const old = useSyncStore.getState().initConflictListener;
beforeEach(() => {
  callbacks.clear();
  vi.useFakeTimers();
  useSyncStore.setState({
    initConflictListener: async () => () => {},
    hasUnreadConflicts: false,
    conflicts: [],
  });
  useUiStore.setState({ safSyncState: 'idle', safSyncError: null, safAuthRevoked: false });
  vi.mocked(listen).mockImplementation(async (name, fn) => {
    callbacks.set(name, fn as never);
    return () => {};
  });
});
afterEach(() => {
  cleanup();
  vi.useRealTimers();
  useSyncStore.setState({ initConflictListener: old });
});
it.each(['__SYNC_ERR__:connect_failed:RF319_PRIVATE', 'SYNC_CONNECT_REFUSED', 'RF319_PRIVATE'])(
  'RF319 actual indicator localizes old/new events without raw body: %s',
  (message) => {
    const view = render(<GlobalSyncIndicator />);
    act(() => {
      callbacks.get('sync-progress')!({ payload: { phase: 'error', message } });
    });
    expect(useUiStore.getState().safSyncState).toBe('error');
    expect(useUiStore.getState().safSyncError).not.toMatch(/RF319_PRIVATE|SYNC_CONNECT/);
    expect(view.container.textContent).not.toMatch(/RF319_PRIVATE|SYNC_CONNECT/);
  },
);
