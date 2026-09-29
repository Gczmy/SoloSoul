import { act, renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useAttachmentBatchOps } from './useAttachmentBatchOps';

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  showToast: vi.fn(),
}));

vi.mock('@/lib/ipcClient', () => ({ invokeCommand: mocks.invoke }));
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
