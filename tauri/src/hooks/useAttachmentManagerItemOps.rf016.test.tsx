import { act, cleanup, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import i18n from '@/lib/i18n';
import { setRequestSession } from '@/lib/sessionRequests';
import { useAttachmentManagerItemOps } from '@/hooks/useAttachmentManagerItemOps';
import type { AttachmentMeta } from '@/components/attachment/attachmentManagerTypes';

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }));

// 不替换被测 hook、附件工具、错误识别或翻译；只控制 IPC 的结果。
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

const attachment: AttachmentMeta = {
  id: 'synthetic-attachment',
  // 恢复对象可保留旧存储对象 ID；IPC 必须使用显式的元数据 owner ID。
  objectId: 'storage-object',
  fileName: 'synthetic.txt',
  mimeType: 'text/plain',
  sizeBytes: 1,
  createdAt: '2026-09-30T00:00:00Z',
};

function mountItem(select = true) {
  const steps: string[] = [];
  const loadData = vi.fn(async () => {
    steps.push('refresh');
  });
  const requestConfirm = vi.fn();
  const showToast = vi.fn((toast: { type: string; message: string }) => {
    steps.push(toast.type);
  });
  const view = renderHook(() =>
    useAttachmentManagerItemOps({
      loadData,
      requestConfirm,
      t: i18n.getFixedT('en-US'),
      showToast,
    }),
  );
  if (select) {
    act(() => view.result.current.handlePermanentDelete(attachment, 'owner-object'));
  }
  return { ...view, steps, loadData, requestConfirm, showToast };
}

async function confirmPermanentDelete(view: ReturnType<typeof mountItem>) {
  await act(async () => {
    await view.result.current.doPermanentDelete();
  });
}

describe('RF-016 attachment manager single deletion outcome', () => {
  it.each([
    ['string', 'attachment_cleanup_pending'],
    ['Error', new Error('attachment_cleanup_pending')],
  ] as const)(
    'refreshes accepted pending %s deletion and warns after refresh',
    async (_, error) => {
      const view = mountItem();
      mocks.invoke.mockRejectedValueOnce(error);

      await confirmPermanentDelete(view);

      expect(mocks.invoke).toHaveBeenCalledTimes(1);
      expect(mocks.invoke).toHaveBeenCalledWith(
        'attachment_delete',
        {
          objectId: 'owner-object',
          attachmentId: 'synthetic-attachment',
        },
        { requestIsCurrent: expect.any(Function) },
      );
      expect(view.loadData).toHaveBeenCalledTimes(1);
      expect(view.showToast).toHaveBeenCalledTimes(1);
      expect(view.showToast).toHaveBeenCalledWith({
        type: 'warning',
        message: i18n.t('common:attachment_cleanup_pending'),
      });
      expect(view.steps).toEqual(['refresh', 'warning']);
      expect(view.result.current.permDeleteItem).toBeNull();
    },
  );

  it('shows a real database error without refreshing or treating it as accepted', async () => {
    const view = mountItem();
    mocks.invoke.mockRejectedValueOnce(new Error('sqlite_commit_failed'));

    await confirmPermanentDelete(view);

    expect(mocks.invoke).toHaveBeenCalledWith(
      'attachment_delete',
      {
        objectId: 'owner-object',
        attachmentId: 'synthetic-attachment',
      },
      { requestIsCurrent: expect.any(Function) },
    );
    expect(view.loadData).not.toHaveBeenCalled();
    expect(view.showToast).toHaveBeenCalledTimes(1);
    const toast = view.showToast.mock.calls[0][0];
    expect(toast.type).toBe('error');
    expect(toast.message).toContain('sqlite_commit_failed');
    expect(toast.message).not.toContain(i18n.t('common:attachment_cleanup_pending'));
    expect(view.steps).toEqual(['error']);
    expect(view.result.current.permDeleteItem).toBeNull();
  });

  it('refreshes completed deletion without a pending or failure toast', async () => {
    const view = mountItem();
    mocks.invoke.mockResolvedValueOnce(undefined);

    await confirmPermanentDelete(view);

    expect(mocks.invoke).toHaveBeenCalledTimes(1);
    expect(view.loadData).toHaveBeenCalledTimes(1);
    expect(view.showToast).not.toHaveBeenCalled();
    expect(view.steps).toEqual(['refresh']);
    expect(view.result.current.permDeleteItem).toBeNull();
  });

  it('does not invoke or refresh when no permanent deletion target is selected', async () => {
    const view = mountItem(false);

    await confirmPermanentDelete(view);

    expect(mocks.invoke).not.toHaveBeenCalled();
    expect(view.loadData).not.toHaveBeenCalled();
    expect(view.showToast).not.toHaveBeenCalled();
    expect(view.result.current.permDeleteItem).toBeNull();
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
      const view = mountItem();
      mocks.invoke.mockReturnValue(pending.promise);
      let operation!: Promise<void>;
      act(() => {
        operation = view.result.current.doPermanentDelete();
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
      const view = mountItem();
      mocks.invoke.mockReturnValue(pending.promise);
      let operation!: Promise<void>;
      act(() => {
        operation = view.result.current.doPermanentDelete();
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
    const view = mountItem();
    view.loadData.mockImplementation(() => refreshing.promise);
    mocks.invoke.mockRejectedValue('attachment_cleanup_pending');
    let operation!: Promise<void>;
    act(() => {
      operation = view.result.current.doPermanentDelete();
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

  it('does not close a new permanent-deletion target when the old operation completes', async () => {
    const pending = ownershipDeferred();
    const view = mountItem();
    mocks.invoke.mockReturnValue(pending.promise);
    let operation!: Promise<void>;
    act(() => {
      operation = view.result.current.doPermanentDelete();
    });
    act(() =>
      view.result.current.handlePermanentDelete(
        { ...attachment, id: 'second-attachment' },
        'second-owner',
      ),
    );
    await act(async () => {
      pending.resolve();
      await operation;
    });
    expect(view.result.current.permDeleteItem?.id).toBe('second-attachment');
    expect(view.result.current.permDeleteItem?._objectId).toBe('second-owner');
  });
});
