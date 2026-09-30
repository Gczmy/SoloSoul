import { act, cleanup, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import i18n from '@/lib/i18n';
import { setRequestSession } from '@/lib/sessionRequests';
import { useAttachmentManagerBatchOps } from '@/hooks/useAttachmentManagerBatchOps';
import type { AttachmentTreePage } from '@/components/attachment/attachmentManagerTypes';

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }));

// 只替换传输边界：hook、useBatchSelect、错误识别和翻译均为真实实现。
vi.mock('@/lib/ipcClient', () => ({ invokeCommand: mocks.invoke }));

beforeEach(async () => {
  mocks.invoke.mockReset();
  setRequestSession('account-a');
  await i18n.changeLanguage('en-US');
});
afterEach(() => {
  cleanup();
  setRequestSession(null);
});

function mountBatch(keys: string[]) {
  const loadData = vi.fn().mockResolvedValue(undefined);
  const showToast = vi.fn();
  const objectIds = [...new Set(keys.map((key) => key.split('::')[0]))];
  const displayPages: AttachmentTreePage[] = [
    {
      pageId: 'synthetic-page',
      pageName: 'Synthetic page',
      objects: objectIds.map((objectId) => ({
        objectId,
        objectName: objectId,
        attachments: keys
          .filter((key) => key.startsWith(`${objectId}::`))
          .map((key) => ({
            id: key.split('::')[1],
            objectId,
            fileName: 'synthetic.txt',
            mimeType: 'text/plain',
            sizeBytes: 1,
            createdAt: '2026-09-30T00:00:00Z',
          })),
      })),
    },
  ];
  const view = renderHook(() =>
    useAttachmentManagerBatchOps({
      allVisibleKeys: keys,
      displayPages,
      loadData,
      t: i18n.getFixedT('en-US'),
      showToast,
    }),
  );
  act(() => {
    for (const key of keys) view.result.current.toggleSelect(key);
    view.result.current.setBatchPermanentDeleteConfirm(true);
  });
  return { ...view, loadData, showToast };
}

async function permanentlyDelete(view: ReturnType<typeof mountBatch>) {
  await act(async () => {
    await view.result.current.handleBatchPermanentDelete();
  });
}

describe('RF-016 attachment manager batch deletion outcome', () => {
  it.each([
    ['string', 'attachment_cleanup_pending'],
    ['Error', new Error('attachment_cleanup_pending')],
  ] as const)('accepts a pending %s result and refreshes with a warning', async (_, error) => {
    const view = mountBatch(['owner-a::attachment-1', 'owner-a::attachment-2']);
    mocks.invoke.mockRejectedValueOnce(error);

    await permanentlyDelete(view);

    expect(mocks.invoke).toHaveBeenCalledTimes(1);
    expect(mocks.invoke).toHaveBeenCalledWith(
      'attachment_batch_delete',
      {
        objectId: 'owner-a',
        attachmentIds: ['attachment-1', 'attachment-2'],
      },
      { requestIsCurrent: expect.any(Function) },
    );
    expect(view.loadData).toHaveBeenCalledTimes(1);
    expect(view.result.current.batchPermanentDeleteConfirm).toBe(false);
    expect(view.result.current.selectedIds.size).toBe(0);
    expect(view.showToast).toHaveBeenCalledTimes(1);
    const toast = view.showToast.mock.calls[0][0];
    expect(toast.type).toBe('warning');
    expect(toast.message).toContain('2/2');
    expect(toast.message).toContain(i18n.t('common:attachment_cleanup_pending'));
    expect(toast.message).not.toContain('failed');
  });

  it('counts a database failure as failed rather than accepted or pending', async () => {
    const view = mountBatch(['owner-a::attachment-1', 'owner-a::attachment-2']);
    mocks.invoke.mockRejectedValueOnce(new Error('sqlite_commit_failed'));

    await permanentlyDelete(view);

    expect(mocks.invoke).toHaveBeenCalledTimes(1);
    expect(view.loadData).toHaveBeenCalledTimes(1);
    const toast = view.showToast.mock.calls[0][0];
    expect(toast.type).toBe('warning');
    expect(toast.message).toContain('0/2');
    expect(toast.message).toContain('2 failed');
    expect(toast.message).not.toContain(i18n.t('common:attachment_cleanup_pending'));
  });

  it('keeps pending and completed groups accepted while counting database failures separately', async () => {
    const view = mountBatch([
      'owner-pending::pending-1',
      'owner-pending::pending-2',
      'owner-completed::complete-1',
      'owner-failed::failed-1',
      'owner-failed::failed-2',
    ]);
    mocks.invoke.mockImplementation((command: string, args?: Record<string, unknown>) => {
      if (command !== 'attachment_batch_delete') throw new Error('unexpected test IPC');
      if (args?.objectId === 'owner-pending') return Promise.reject('attachment_cleanup_pending');
      if (args?.objectId === 'owner-completed') return Promise.resolve(undefined);
      if (args?.objectId === 'owner-failed')
        return Promise.reject(new Error('sqlite_commit_failed'));
      throw new Error('unexpected test object');
    });

    await permanentlyDelete(view);

    expect(mocks.invoke).toHaveBeenCalledTimes(3);
    for (const [objectId, attachmentIds] of [
      ['owner-pending', ['pending-1', 'pending-2']],
      ['owner-completed', ['complete-1']],
      ['owner-failed', ['failed-1', 'failed-2']],
    ] as const) {
      expect(mocks.invoke).toHaveBeenCalledWith(
        'attachment_batch_delete',
        {
          objectId,
          attachmentIds,
        },
        { requestIsCurrent: expect.any(Function) },
      );
    }
    expect(view.loadData).toHaveBeenCalledTimes(1);
    expect(view.showToast).toHaveBeenCalledTimes(1);
    const toast = view.showToast.mock.calls[0][0];
    expect(toast.type).toBe('warning');
    expect(toast.message).toContain('3/5');
    expect(toast.message).toContain('2 failed');
    expect(toast.message).toContain(i18n.t('common:attachment_cleanup_pending'));
  });

  it('uses the completed result without a pending warning when every group finishes', async () => {
    const view = mountBatch([
      'owner-a::attachment-1',
      'owner-a::attachment-2',
      'owner-b::attachment-3',
    ]);
    mocks.invoke.mockResolvedValue(undefined);

    await permanentlyDelete(view);

    expect(mocks.invoke).toHaveBeenCalledTimes(2);
    expect(view.loadData).toHaveBeenCalledTimes(1);
    expect(view.result.current.selectedIds.size).toBe(0);
    const toast = view.showToast.mock.calls[0][0];
    expect(toast.type).toBe('info');
    expect(toast.message).toContain('3/3');
    expect(toast.message).not.toContain(i18n.t('common:attachment_cleanup_pending'));
    expect(toast.message).not.toContain('failed');
  });
});

function ownershipDeferred() {
  let resolve!: () => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<void>((ok, fail) => {
    resolve = ok;
    reject = fail;
  });
  return { promise, resolve, reject };
}

describe('RF016 manager completion ownership', () => {
  it.each(['success', 'pending', 'failure'] as const)(
    'unmounted %s completion cannot refresh or show a toast',
    async (outcome) => {
      const pending = ownershipDeferred();
      const view = mountBatch(['owner-a::attachment-1']);
      mocks.invoke.mockReturnValue(pending.promise);
      let operation!: Promise<void>;
      act(() => {
        operation = view.result.current.handleBatchPermanentDelete();
      });
      view.unmount();
      await act(async () => {
        if (outcome === 'success') pending.resolve();
        else
          pending.reject(
            outcome === 'pending' ? 'attachment_cleanup_pending' : new Error('database failed'),
          );
        await operation;
      });
      expect(view.loadData).not.toHaveBeenCalled();
      expect(view.showToast).not.toHaveBeenCalled();
    },
  );

  it.each(['lock', 'account-switch', 'same-account-reunlock'] as const)(
    '%s rejects an old pending result',
    async (change) => {
      const pending = ownershipDeferred();
      const view = mountBatch(['owner-a::attachment-1']);
      mocks.invoke.mockReturnValue(pending.promise);
      let operation!: Promise<void>;
      act(() => {
        operation = view.result.current.handleBatchPermanentDelete();
      });
      if (change === 'account-switch') setRequestSession('account-b');
      else {
        setRequestSession(null);
        if (change === 'same-account-reunlock') setRequestSession('account-a');
      }
      await act(async () => {
        pending.reject('attachment_cleanup_pending');
        await operation;
      });
      expect(view.loadData).not.toHaveBeenCalled();
      expect(view.showToast).not.toHaveBeenCalled();
    },
  );

  it('does not report the prior result after the refresh loses its session', async () => {
    const refreshing = ownershipDeferred();
    const view = mountBatch(['owner-a::attachment-1']);
    view.loadData.mockImplementation(() => refreshing.promise);
    mocks.invoke.mockRejectedValue('attachment_cleanup_pending');
    let operation!: Promise<void>;
    act(() => {
      operation = view.result.current.handleBatchPermanentDelete();
    });
    await act(async () => {
      await Promise.resolve();
      await Promise.resolve();
    });
    expect(view.loadData).toHaveBeenCalledOnce();
    setRequestSession(null);
    await act(async () => {
      refreshing.resolve();
      await operation;
    });
    expect(view.showToast).not.toHaveBeenCalled();
  });

  it('rejects later group dispatch when the first dispatch invalidates the original session', async () => {
    const pending = ownershipDeferred();
    const view = mountBatch(['owner-a::attachment-1', 'owner-b::attachment-2']);
    // 两组保持并发；只要求派发时已失效的组不得进入传输层，不撤回有效的在途组。
    mocks.invoke.mockImplementation(() => {
      setRequestSession(null);
      return pending.promise;
    });
    let operation!: Promise<void>;
    act(() => {
      operation = view.result.current.handleBatchPermanentDelete();
    });
    expect(mocks.invoke).toHaveBeenCalledOnce();
    await act(async () => {
      pending.reject('attachment_cleanup_pending');
      await operation;
    });
    expect(mocks.invoke).toHaveBeenCalledOnce();
    expect(view.loadData).not.toHaveBeenCalled();
    expect(view.showToast).not.toHaveBeenCalled();
  });
});
