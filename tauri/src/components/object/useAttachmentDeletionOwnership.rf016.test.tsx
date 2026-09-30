import { act, cleanup, renderHook, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { setRequestSession } from '@/lib/sessionRequests';
import { useUiStore } from '@/stores/uiStore';
import type { AttachmentItem } from '@/lib/attachmentUtils';
import { useAttachmentViewer } from './useAttachmentViewer';

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@/lib/ipcClient', () => ({ invokeCommand: mocks.invoke }));
vi.mock('@/hooks/useDragToAttach', () => ({
  useDragToAttach: () => ({ ref: { current: null }, dragState: {} }),
}));

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((ok, fail) => {
    resolve = ok;
    reject = fail;
  });
  return { promise, resolve, reject };
}
function itemFor(objectId: string): AttachmentItem {
  return {
    id: `attachment-${objectId}`,
    objectId,
    fileName: `${objectId}.txt`,
    mimeType: 'text/plain',
    sizeBytes: 5,
    createdAt: '2026-09-30',
  };
}
function clearToasts() {
  for (const toast of useUiStore.getState().toasts) useUiStore.getState().dismissToast(toast.id);
}

beforeEach(() => {
  mocks.invoke.mockReset();
  setRequestSession('account-a');
  clearToasts();
});
afterEach(() => {
  cleanup();
  setRequestSession(null);
  clearToasts();
});

type Mode = 'single' | 'batch';
type Outcome = 'success' | 'pending' | 'failure';
function settle(operation: ReturnType<typeof deferred<void>>, outcome: Outcome) {
  if (outcome === 'success') operation.resolve(undefined);
  else
    operation.reject(
      outcome === 'pending' ? 'attachment_cleanup_pending' : new Error('database failed'),
    );
}

async function fixture(mode: Mode) {
  const deletion = deferred<void>();
  const onCountChange = vi.fn();
  let refreshing = false;
  const activeRefresh = deferred<AttachmentItem[]>();
  const deletedRefresh = deferred<AttachmentItem[]>();
  let holdRefresh = false;
  mocks.invoke.mockImplementation(
    (command: string, args?: { objectId?: string; showDeleted?: boolean }) => {
      if (command === 'attachment_delete' || command === 'attachment_batch_delete') {
        refreshing = true;
        return deletion.promise;
      }
      if (command === 'attachment_list') {
        if (refreshing && holdRefresh && args?.objectId === 'object-a')
          return args.showDeleted ? deletedRefresh.promise : activeRefresh.promise;
        return Promise.resolve(args?.showDeleted ? [] : [itemFor(args?.objectId ?? '')]);
      }
      return Promise.resolve(undefined);
    },
  );
  const hook = renderHook(
    ({ id }) => useAttachmentViewer({ objectId: id, onClose: vi.fn(), onCountChange }),
    { initialProps: { id: 'object-a' } },
  );
  await waitFor(() => expect(hook.result.current.items).toHaveLength(1));
  if (mode === 'batch')
    act(() => hook.result.current.toggleSelect('object-a::attachment-object-a'));
  function start() {
    let promise!: Promise<void>;
    act(() => {
      promise =
        mode === 'single'
          ? hook.result.current.handlePermanentDelete(itemFor('object-a'))
          : hook.result.current.handleBatchPermanentDelete();
    });
    return promise;
  }
  return {
    ...hook,
    deletion,
    onCountChange,
    start,
    activeRefresh,
    deletedRefresh,
    holdRefresh: () => {
      holdRefresh = true;
    },
  };
}

describe.each(['single', 'batch'] as const)('RF016 %s deletion ownership', (mode) => {
  it.each(['success', 'pending', 'failure'] as const)(
    'unmounted %s completion has no toast, reload or parent callback',
    async (outcome) => {
      const f = await fixture(mode);
      const operation = f.start();
      f.unmount();
      const calls = mocks.invoke.mock.calls.length;
      await act(async () => {
        settle(f.deletion, outcome);
        await operation;
      });
      expect(mocks.invoke).toHaveBeenCalledTimes(calls);
      expect(f.onCountChange).not.toHaveBeenCalled();
      expect(useUiStore.getState().toasts).toEqual([]);
    },
  );

  it.each(['lock', 'account-switch', 'same-account-reunlock'] as const)(
    '%s rejects pending completion in a still mounted hook',
    async (change) => {
      const f = await fixture(mode);
      const operation = f.start();
      if (change === 'account-switch') setRequestSession('account-b');
      else {
        setRequestSession(null);
        if (change === 'same-account-reunlock') setRequestSession('account-a');
      }
      const calls = mocks.invoke.mock.calls.length;
      await act(async () => {
        f.deletion.reject('attachment_cleanup_pending');
        await operation;
      });
      expect(mocks.invoke).toHaveBeenCalledTimes(calls);
      expect(f.onCountChange).not.toHaveBeenCalled();
      expect(useUiStore.getState().toasts).toEqual([]);
    },
  );

  it('changed object rejects the former successful completion', async () => {
    const f = await fixture(mode);
    const operation = f.start();
    f.rerender({ id: 'object-b' });
    await waitFor(() => expect(f.result.current.items[0]?.objectId).toBe('object-b'));
    const calls = mocks.invoke.mock.calls.length;
    await act(async () => {
      f.deletion.resolve(undefined);
      await operation;
    });
    expect(mocks.invoke).toHaveBeenCalledTimes(calls);
    expect(f.result.current.items[0]?.objectId).toBe('object-b');
    expect(f.onCountChange).not.toHaveBeenCalled();
    expect(useUiStore.getState().toasts).toEqual([]);
  });

  it('A to B to A in the same hook keeps the original object operation expired', async () => {
    const f = await fixture(mode);
    const operation = f.start();
    f.rerender({ id: 'object-b' });
    await waitFor(() => expect(f.result.current.items[0]?.objectId).toBe('object-b'));
    f.rerender({ id: 'object-a' });
    await waitFor(() => expect(f.result.current.items[0]?.objectId).toBe('object-a'));
    const calls = mocks.invoke.mock.calls.length;
    await act(async () => {
      f.deletion.reject('attachment_cleanup_pending');
      await operation;
    });
    expect(mocks.invoke).toHaveBeenCalledTimes(calls);
    expect(f.onCountChange).not.toHaveBeenCalled();
    expect(useUiStore.getState().toasts).toEqual([]);
  });

  it('session expiry during the metadata reload rejects list values and the later parent callback', async () => {
    const f = await fixture(mode);
    f.holdRefresh();
    const operation = f.start();
    await act(async () => {
      f.deletion.reject('attachment_cleanup_pending');
      await Promise.resolve();
    });
    await waitFor(() =>
      expect(
        mocks.invoke.mock.calls.filter(([command]) => command === 'attachment_list'),
      ).toHaveLength(4),
    );
    clearToasts();
    setRequestSession(null);
    await act(async () => {
      f.activeRefresh.resolve([]);
      f.deletedRefresh.resolve([]);
      await operation;
    });
    expect(f.result.current.items[0]?.objectId).toBe('object-a');
    expect(f.onCountChange).not.toHaveBeenCalled();
    expect(useUiStore.getState().toasts).toEqual([]);
  });

  it('current pending deletion reloads accepted metadata and reports cleanup pending', async () => {
    const f = await fixture(mode);
    const operation = f.start();
    await act(async () => {
      f.deletion.reject('attachment_cleanup_pending');
      await operation;
    });
    expect(
      mocks.invoke.mock.calls.filter(([command]) => command === 'attachment_list'),
    ).toHaveLength(4);
    expect(f.onCountChange).toHaveBeenCalledOnce();
    expect(useUiStore.getState().toasts).toEqual([expect.objectContaining({ type: 'warning' })]);
    if (mode === 'batch') expect(f.result.current.selectedIds.size).toBe(0);
  });
});
