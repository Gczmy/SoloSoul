import { act, cleanup, renderHook, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useAuthStore } from '@/stores/authStore';
import { useAttachmentManager } from './useAttachmentManager';
import type { AttachmentListAllResult } from '@/components/attachment/attachmentManagerTypes';

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@/lib/ipcClient', () => ({ invokeCommand: mocks.invoke }));
function deferred() {
  let resolve!: (value: AttachmentListAllResult) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<AttachmentListAllResult>((done, fail) => {
    resolve = done;
    reject = fail;
  });
  return { promise, resolve, reject };
}
function data(marker: string): AttachmentListAllResult {
  return { pages: [{ pageName: marker, objects: [] }], trashPages: [] };
}
const accountA = { id: 'account-a', name: 'Synthetic A' };
const accountB = { id: 'account-b', name: 'Synthetic B' };
beforeEach(() => {
  mocks.invoke.mockReset();
  useAuthStore.setState({ currentAccount: null, isAuthenticated: false });
});
afterEach(() => {
  cleanup();
  useAuthStore.setState({ currentAccount: null, isAuthenticated: false });
});

describe('RF016 attachment manager reload ownership', () => {
  it.each(['success', 'failure'] as const)(
    'account A late %s cannot replace or clear loaded account B',
    async (outcome) => {
      const a = deferred();
      const b = deferred();
      mocks.invoke.mockImplementation((command: string, args?: { accountId?: string }) => {
        if (command === 'attachment_list_all')
          return args?.accountId === accountA.id ? a.promise : b.promise;
        return Promise.resolve(undefined);
      });
      useAuthStore.setState({ currentAccount: accountA, isAuthenticated: true });
      const f = renderHook(useAttachmentManager);
      await waitFor(() =>
        expect(
          mocks.invoke.mock.calls.filter(([cmd]) => cmd === 'attachment_list_all'),
        ).toHaveLength(1),
      );
      act(() => useAuthStore.setState({ currentAccount: accountB, isAuthenticated: true }));
      await waitFor(() =>
        expect(
          mocks.invoke.mock.calls.filter(([cmd]) => cmd === 'attachment_list_all'),
        ).toHaveLength(2),
      );
      await act(async () => {
        b.resolve(data('B'));
        await b.promise;
      });
      expect(f.result.current.data?.pages[0].pageName).toBe('B');
      await act(async () => {
        if (outcome === 'success') a.resolve(data('A'));
        else a.reject(new Error('old read failed'));
        await a.promise.catch(() => undefined);
      });
      expect(f.result.current.data?.pages[0].pageName).toBe('B');
      expect(f.result.current.loading).toBe(false);
    },
  );

  it('same account re-unlock rejects the prior session list', async () => {
    const oldRead = deferred();
    const freshRead = deferred();
    let calls = 0;
    mocks.invoke.mockImplementation((command: string) =>
      command === 'attachment_list_all'
        ? ++calls === 1
          ? oldRead.promise
          : freshRead.promise
        : Promise.resolve(undefined),
    );
    useAuthStore.setState({ currentAccount: accountA, isAuthenticated: true });
    const f = renderHook(useAttachmentManager);
    await waitFor(() => expect(calls).toBe(1));
    act(() => useAuthStore.setState({ isAuthenticated: false }));
    act(() => useAuthStore.setState({ isAuthenticated: true }));
    let reload!: Promise<void>;
    act(() => {
      reload = f.result.current.loadData();
    });
    await act(async () => {
      freshRead.resolve(data('fresh A'));
      await reload;
    });
    await act(async () => {
      oldRead.resolve(data('old A'));
      await oldRead.promise;
    });
    expect(f.result.current.data?.pages[0].pageName).toBe('fresh A');
  });

  it('a newer metadata refresh wins over an older read in the same session', async () => {
    const oldRead = deferred();
    const freshRead = deferred();
    let calls = 0;
    mocks.invoke.mockImplementation((command: string) =>
      command === 'attachment_list_all'
        ? ++calls === 1
          ? oldRead.promise
          : freshRead.promise
        : Promise.resolve(undefined),
    );
    useAuthStore.setState({ currentAccount: accountA, isAuthenticated: true });
    const f = renderHook(useAttachmentManager);
    await waitFor(() => expect(calls).toBe(1));
    let reload!: Promise<void>;
    act(() => {
      reload = f.result.current.loadData();
    });
    await act(async () => {
      freshRead.resolve(data('fresh'));
      await reload;
    });
    await act(async () => {
      oldRead.resolve(data('old'));
      await oldRead.promise;
    });
    expect(f.result.current.data?.pages[0].pageName).toBe('fresh');
  });
});
