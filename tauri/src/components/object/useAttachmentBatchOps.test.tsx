import { act, renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useAttachmentBatchOps } from './useAttachmentBatchOps';

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  showToast: vi.fn(),
  openWithPause: vi.fn(),
}));

vi.mock('@/lib/ipcClient', () => ({ invokeCommand: mocks.invoke }));
vi.mock('@/lib/platform', () => ({ isMobilePlatformSync: () => false }));
vi.mock('@/lib/dialog', () => ({ openWithPause: mocks.openWithPause }));
vi.mock('@/lib/logger', () => ({ logger: { warn: vi.fn() } }));
vi.mock('@/stores/uiStore', () => ({
  useUiStore: (selector: (state: { showToast: typeof mocks.showToast }) => unknown) =>
    selector({ showToast: mocks.showToast }),
}));
vi.mock('react-i18next', () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

beforeEach(() => {
  vi.clearAllMocks();
});

describe('RF-1017 attachment batch retry', () => {
  it.each([
    ['handleBatchDelete', 'attachment_batch_soft_delete'],
    ['handleBatchRestore', 'attachment_batch_restore'],
    ['handleBatchPermanentDelete', 'attachment_batch_delete'],
  ] as const)(
    '%s keeps the selection after failure and clears it after retry',
    async (handler, command) => {
      const loadAttachments = vi.fn().mockResolvedValue(undefined);
      const onCountChange = vi.fn();
      const key = 'object-a::attachment-1';
      const { result } = renderHook(() =>
        useAttachmentBatchOps({
          objectId: 'object-a',
          allVisibleKeys: [key],
          displayItems: [],
          loadAttachments,
          onCountChange,
        }),
      );
      act(() => result.current.toggleSelect(key));
      mocks.invoke
        .mockRejectedValueOnce(new Error('vault unavailable'))
        .mockResolvedValue(undefined);

      await act(async () => {
        await result.current[handler]();
      });
      expect(mocks.invoke).toHaveBeenCalledWith(command, {
        objectId: 'object-a',
        attachmentIds: ['attachment-1'],
      });
      expect(result.current.selectedIds).toEqual(new Set([key]));
      expect(loadAttachments).not.toHaveBeenCalled();
      expect(onCountChange).not.toHaveBeenCalled();
      expect(mocks.showToast).toHaveBeenLastCalledWith(
        expect.objectContaining({ type: 'warning' }),
      );

      await act(async () => {
        await result.current[handler]();
      });
      expect(result.current.selectedIds).toEqual(new Set());
      expect(loadAttachments).toHaveBeenCalledOnce();
      expect(onCountChange).toHaveBeenCalledOnce();
      expect(mocks.showToast).toHaveBeenLastCalledWith(
        expect.objectContaining({ type: 'success' }),
      );
    },
  );
});

describe('RF-1018 attachment batch download retry', () => {
  it('keeps only failed downloads selected and retries only those files', async () => {
    const firstKey = 'object-a::attachment-1';
    const secondKey = 'object-a::attachment-2';
    const { result } = renderHook(() =>
      useAttachmentBatchOps({
        objectId: 'object-a',
        allVisibleKeys: [firstKey, secondKey],
        displayItems: [
          {
            id: 'attachment-1',
            objectId: 'object-a',
            fileName: 'first.txt',
            mimeType: 'text/plain',
            sizeBytes: 5,
            createdAt: '2026-09-29',
            vaultPath: '/vault/first.txt',
          },
          {
            id: 'attachment-2',
            objectId: 'object-a',
            fileName: 'second.txt',
            mimeType: 'text/plain',
            sizeBytes: 6,
            createdAt: '2026-09-29',
            vaultPath: '/vault/second.txt',
          },
        ],
        loadAttachments: vi.fn(),
      }),
    );
    mocks.openWithPause.mockResolvedValue('C:/Downloads');
    mocks.invoke.mockImplementation((_command: string, args: { destPath: string }) => {
      if (args.destPath.endsWith('second.txt') && mocks.invoke.mock.calls.length === 2) {
        return Promise.reject(new Error('disk unavailable'));
      }
      return Promise.resolve();
    });
    act(() => {
      result.current.toggleSelect(firstKey);
      result.current.toggleSelect(secondKey);
    });

    await act(async () => {
      await result.current.handleBatchDownload();
    });
    expect(result.current.selectedIds).toEqual(new Set([secondKey]));
    expect(mocks.showToast).toHaveBeenLastCalledWith(expect.objectContaining({ type: 'warning' }));

    await act(async () => {
      await result.current.handleBatchDownload();
    });
    expect(mocks.invoke).toHaveBeenCalledTimes(3);
    expect(mocks.invoke).toHaveBeenLastCalledWith('attachment_download', {
      srcPath: '/vault/second.txt',
      destPath: 'C:/Downloads/second.txt',
    });
    expect(result.current.selectedIds).toEqual(new Set());
    expect(mocks.showToast).toHaveBeenLastCalledWith(expect.objectContaining({ type: 'success' }));
  });
});
