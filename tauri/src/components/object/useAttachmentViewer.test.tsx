import { act, cleanup, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { pickFileToAttach, uploadSingleAttachment } from '@/lib/attachmentUpload';
import type { AttachmentItem } from '@/lib/attachmentUtils';
import { useUiStore } from '@/stores/uiStore';
import { useAttachmentViewer } from './useAttachmentViewer';

// 保留真实 Hook、IPC 封装和 Toast Store，只隔离原生文件/上传/拖拽边界。
vi.mock('@/lib/attachmentUpload', () => ({
  pickFileToAttach: vi.fn(),
  uploadSingleAttachment: vi.fn(),
}));
vi.mock('@/hooks/useDragToAttach', () => ({
  useDragToAttach: () => ({
    ref: { current: null },
    dragState: {
      isDraggingOver: false,
      isUploading: false,
      currentIndex: 0,
      totalFiles: 0,
      currentFileName: '',
      pendingFiles: 0,
    },
  }),
}));

const mockInvoke = vi.mocked(invoke);
const mockPick = vi.mocked(pickFileToAttach);
const mockUpload = vi.mocked(uploadSingleAttachment);
const objectId = 'object-a';

function attachment(id: string, deleted = false): AttachmentItem {
  return {
    id,
    objectId,
    fileName: `${id}.txt`,
    mimeType: 'text/plain',
    sizeBytes: 24,
    createdAt: '2026-09-28T00:00:00Z',
    vaultPath: `/synthetic-vault/${id}.txt`,
    deletedAt: deleted ? '2026-09-28T01:00:00Z' : null,
  };
}

const oldActive = [attachment('existing')];
const oldDeleted = [attachment('deleted', true)];
const newActive = [...oldActive, attachment('uploaded')];
const newDeleted = [attachment('new-deleted', true)];

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((yes) => {
    resolve = yes;
  });
  return { promise, resolve };
}

function setListResponse(
  active: () => Promise<AttachmentItem[]>,
  deleted: () => Promise<AttachmentItem[]>,
) {
  mockInvoke.mockImplementation((command, args) => {
    if (command !== 'attachment_list') throw new Error(`Unexpected IPC: ${command}`);
    const request = args as { objectId: string; showDeleted: boolean };
    expect(request.objectId).toBe(objectId);
    return request.showDeleted ? deleted() : active();
  });
}

function expectBothListRequests() {
  expect(mockInvoke).toHaveBeenCalledTimes(2);
  expect(mockInvoke).toHaveBeenCalledWith('attachment_list', { objectId, showDeleted: false });
  expect(mockInvoke).toHaveBeenCalledWith('attachment_list', { objectId, showDeleted: true });
}

async function renderLoadedViewer() {
  const onCountChange = vi.fn();
  const view = renderHook(() => useAttachmentViewer({ objectId, onClose: vi.fn(), onCountChange }));
  await act(async () => {
    await Promise.resolve();
  });
  expect(view.result.current).toMatchObject({
    items: oldActive,
    trashItems: oldDeleted,
    loading: false,
  });
  expectBothListRequests();
  mockInvoke.mockClear();
  return { ...view, onCountChange };
}

function toastTypes() {
  return useUiStore.getState().toasts.map((toast) => toast.type);
}

describe('RF-910 attachment upload refresh feedback', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    mockInvoke.mockReset();
    mockPick.mockReset().mockResolvedValue('/synthetic-input/uploaded.txt');
    mockUpload.mockReset().mockResolvedValue('uploaded');
    setListResponse(
      () => Promise.resolve(oldActive),
      () => Promise.resolve(oldDeleted),
    );
  });

  afterEach(() => {
    cleanup();
    for (const toast of useUiStore.getState().toasts) useUiStore.getState().dismissToast(toast.id);
    vi.clearAllTimers();
    vi.useRealTimers();
  });

  it.each(['active', 'deleted'] as const)(
    'warns when the %s refresh fails after a successful upload',
    async (failedList) => {
      const { result, onCountChange } = await renderLoadedViewer();
      setListResponse(
        () =>
          failedList === 'active'
            ? Promise.reject(new Error('active list unavailable'))
            : Promise.resolve(newActive),
        () =>
          failedList === 'deleted'
            ? Promise.reject(new Error('deleted list unavailable'))
            : Promise.resolve(newDeleted),
      );

      await act(async () => {
        await result.current.handleAdd();
      });

      expect(mockUpload).toHaveBeenCalledExactlyOnceWith('/synthetic-input/uploaded.txt', objectId);
      expectBothListRequests();
      expect(toastTypes()).toEqual(['success', 'warning']);
      expect(useUiStore.getState().toasts[1].message).toMatch(/list.*refresh/i);
      expect(result.current).toMatchObject({
        items: oldActive,
        trashItems: oldDeleted,
        loading: false,
        uploading: false,
      });
      expect(onCountChange).toHaveBeenCalledTimes(1);
    },
  );

  it('replaces both lists after a successful upload and refresh, with one count notification', async () => {
    const { result, onCountChange } = await renderLoadedViewer();
    setListResponse(
      () => Promise.resolve(newActive),
      () => Promise.resolve(newDeleted),
    );

    await act(async () => {
      await result.current.handleAdd();
    });

    expect(mockUpload).toHaveBeenCalledExactlyOnceWith('/synthetic-input/uploaded.txt', objectId);
    expectBothListRequests();
    expect(result.current).toMatchObject({
      items: newActive,
      trashItems: newDeleted,
      loading: false,
      uploading: false,
    });
    expect(toastTypes()).toEqual(['success']);
    expect(onCountChange).toHaveBeenCalledTimes(1);
  });

  it('ends a stalled refresh after 15 seconds and ignores its late rows while retaining the successful upload notification', async () => {
    const { result, onCountChange } = await renderLoadedViewer();
    const active = deferred<AttachmentItem[]>();
    const deleted = deferred<AttachmentItem[]>();
    setListResponse(
      () => active.promise,
      () => deleted.promise,
    );
    let adding!: Promise<void>;
    await act(async () => {
      adding = result.current.handleAdd();
      await Promise.resolve();
    });
    expectBothListRequests();
    expect(toastTypes()).toEqual(['success']);
    expect(result.current).toMatchObject({ loading: true, uploading: true });

    await act(async () => {
      await vi.advanceTimersByTimeAsync(14_999);
    });
    expect(result.current).toMatchObject({ loading: true, uploading: true });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1);
      await adding;
    });
    expect(result.current).toMatchObject({
      items: oldActive,
      trashItems: oldDeleted,
      loading: false,
      uploading: false,
    });
    // 成功 Toast 的正常展示时间已结束，刷新超时应新增 warning，而非上传失败。
    expect(toastTypes()).toEqual(['warning']);
    expect(onCountChange).toHaveBeenCalledTimes(1);

    await act(async () => {
      active.resolve(newActive);
      deleted.resolve(newDeleted);
      await Promise.all([active.promise, deleted.promise]);
    });
    expect(result.current).toMatchObject({
      items: oldActive,
      trashItems: oldDeleted,
      loading: false,
      uploading: false,
    });
    expect(toastTypes()).toEqual(['warning']);
    expect(onCountChange).toHaveBeenCalledTimes(1);
  });

  it('does not upload, refresh or notify when the file picker is cancelled', async () => {
    const { result, onCountChange } = await renderLoadedViewer();
    mockPick.mockResolvedValueOnce(null);
    await act(async () => {
      await result.current.handleAdd();
    });
    expect(mockPick).toHaveBeenCalledTimes(1);
    expect(mockUpload).not.toHaveBeenCalled();
    expect(mockInvoke).not.toHaveBeenCalled();
    expect(onCountChange).not.toHaveBeenCalled();
    expect(toastTypes()).toEqual([]);
    expect(result.current).toMatchObject({
      items: oldActive,
      trashItems: oldDeleted,
      loading: false,
      uploading: false,
    });
  });

  it('reports upload failure without refreshing lists or announcing a count change', async () => {
    const { result, onCountChange } = await renderLoadedViewer();
    mockUpload.mockRejectedValueOnce(new Error('upload refused'));
    await act(async () => {
      await result.current.handleAdd();
    });
    expect(mockInvoke).not.toHaveBeenCalled();
    expect(onCountChange).not.toHaveBeenCalled();
    expect(toastTypes()).toEqual(['error']);
    expect(result.current).toMatchObject({
      items: oldActive,
      trashItems: oldDeleted,
      loading: false,
      uploading: false,
    });
  });

  it('settles an ordinary initial load failure without an unhandled rejection or an upload notification', async () => {
    const onCountChange = vi.fn();
    setListResponse(
      () => Promise.reject(new Error('initial list unavailable')),
      () => Promise.resolve(oldDeleted),
    );
    const { result } = renderHook(() =>
      useAttachmentViewer({ objectId, onClose: vi.fn(), onCountChange }),
    );
    await act(async () => {
      await Promise.resolve();
    });
    expectBothListRequests();
    expect(result.current).toMatchObject({
      items: [],
      trashItems: [],
      loading: false,
      uploading: false,
    });
    expect(onCountChange).not.toHaveBeenCalled();
    expect(mockUpload).not.toHaveBeenCalled();
    expect(toastTypes()).toEqual([]);
  });
});
