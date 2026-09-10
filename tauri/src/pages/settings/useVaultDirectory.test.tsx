import { StrictMode } from 'react';
import { act, cleanup, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { listen, type Event, type EventCallback } from '@tauri-apps/api/event';
import { useVaultDirectory, type SyncProgress } from './useVaultDirectory';

const mocks = vi.hoisted(() => ({
  t: (key: string) => key,
  onError: vi.fn(),
  onSuccess: vi.fn(),
}));
vi.mock('react-i18next', () => ({ useTranslation: () => ({ t: mocks.t }) }));
vi.mock('@/hooks/useToastError', () => ({ useToastError: () => mocks }));
vi.mock('@/lib/platform', () => ({ getPlatform: vi.fn(async () => 'android') }));
vi.mock('@tauri-apps/plugin-process', () => ({ relaunch: vi.fn() }));
vi.mock('@/stores/uiStore', () => ({ useUiStore: { getState: vi.fn() } }));
vi.mock('@/lib/vaultDirectory', () => ({
  getVaultDirectory: vi.fn(async () => ({ directoryType: 'local', safTreeUri: null, valid: true })),
  setVaultDirectory: vi.fn(),
  pickVaultDirectory: vi.fn(),
  syncVaultToRemote: vi.fn(),
  syncVaultFromRemote: vi.fn(),
}));

let handlers: EventCallback<SyncProgress>[];
function progress(payload: SyncProgress) {
  act(() => handlers.at(-1)!({ payload } as Event<SyncProgress>));
}

describe('useVaultDirectory 事件生命周期', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.clearAllMocks();
    handlers = [];
    vi.mocked(listen).mockImplementation(async (_name, handler) => {
      handlers.push(handler as EventCallback<SyncProgress>);
      return vi.fn<() => void>();
    });
  });
  afterEach(() => {
    cleanup();
    vi.useRealTimers();
  });

  it('卸载早于订阅完成时，仍释放迟到的监听器', async () => {
    let resolve!: (fn: () => void) => void;
    vi.mocked(listen).mockReturnValueOnce(
      new Promise((done) => {
        resolve = done;
      }),
    );
    const unlisten = vi.fn();
    const { unmount } = renderHook(() => useVaultDirectory());
    unmount();
    await act(async () => {
      resolve(unlisten);
    });
    expect(unlisten).toHaveBeenCalledOnce();
  });

  it('StrictMode 重建 Effect 后，完成进度仍在两秒后消失', async () => {
    const { result } = renderHook(() => useVaultDirectory(), { wrapper: StrictMode });
    await act(async () => {});
    progress({ phase: 'sync_to_remote', current: 1, total: 1 });
    expect(result.current.syncProgress?.current).toBe(1);
    act(() => vi.advanceTimersByTime(2000));
    expect(result.current.syncProgress).toBeNull();
  });

  it('切换语言不会取消已完成进度的清除计时', async () => {
    const { result, rerender } = renderHook(() => useVaultDirectory());
    await act(async () => {});
    progress({ phase: 'sync_to_remote', current: 1, total: 1 });
    act(() => vi.advanceTimersByTime(1000));
    mocks.t = (key: string) => `translated:${key}`;
    rerender();
    await act(async () => {});
    act(() => vi.advanceTimersByTime(1000));
    expect(result.current.syncProgress).toBeNull();
  });

  it('上一轮的完成计时器不会隐藏新操作，并在卸载时释放', async () => {
    const { result, unmount } = renderHook(() => useVaultDirectory());
    await act(async () => {});
    progress({ phase: 'sync_to_remote', current: 1, total: 1 });
    act(() => vi.advanceTimersByTime(1000));
    progress({ phase: 'migrate', current: 1, total: 3 });
    act(() => vi.advanceTimersByTime(1000));
    expect(result.current.syncProgress).toEqual({ phase: 'migrate', current: 1, total: 3 });
    progress({ phase: 'migrate', current: 3, total: 3 });
    unmount();
    expect(vi.getTimerCount()).toBe(0);
  });
});
