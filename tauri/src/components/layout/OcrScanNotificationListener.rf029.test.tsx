import { platformCapabilityStore } from '@/lib/platformCapabilities';
import { capabilityFixture } from '@/lib/__fixtures__/platformCapabilityFixture';
import { act, cleanup, render, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { EventCallback } from '@tauri-apps/api/event';
import type { OcrJobEvent } from '@/lib/ocrScanOperation';
import { useAuthStore } from '@/stores/authStore';
import { useOcrScanStore } from '@/stores/ocrScanStore';
import { OcrScanNotificationListener } from './OcrScanNotificationListener';

const mocks = vi.hoisted(() => ({ ipc: vi.fn(), listen: vi.fn(), toast: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: mocks.ipc }));
vi.mock('@tauri-apps/api/event', () => ({ listen: mocks.listen }));
vi.mock('@/stores/uiStore', () => ({
  useUiStore: (selector: (state: { showToast: typeof mocks.toast }) => unknown) =>
    selector({ showToast: mocks.toast }),
}));
vi.mock('@/lib/logger', () => ({
  logger: { warn: vi.fn(), error: vi.fn(), info: vi.fn(), debug: vi.fn() },
}));

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}
const RESULT = { text: 'RF029 synthetic notification result', confidence: 0.9, boxes: [] };
let listener: EventCallback<OcrJobEvent>;
let releases: (() => void)[];
let works: Promise<void>[];
function pending<T>(fallback: T) {
  const reply = deferred<T>();
  releases.push(() => reply.resolve(fallback));
  return reply;
}
function start(mode: 'general' | 'mrz') {
  useOcrScanStore.getState().setScanMode(mode);
  let work!: Promise<void>;
  act(() => {
    work = useOcrScanStore.getState().performScan('C:/RF029-synthetic/notify.png');
  });
  works.push(work);
  return work;
}
function scanCalls() {
  return mocks.ipc.mock.calls.filter(
    ([name]) => name === 'ocr_scan_image' || name === 'ocr_scan_mrz',
  );
}
async function task(index = 0) {
  await waitFor(() => expect(scanCalls().length).toBeGreaterThan(index));
  return scanCalls()[index][1].taskId as string;
}
function event(taskId: string, state: OcrJobEvent['state'], sequence: number) {
  act(() =>
    listener({
      event: 'ocr-job-state',
      id: 1,
      payload: {
        taskId,
        accountId: 'rf029-a',
        sessionGeneration: 9,
        state,
        sequence,
      },
    }),
  );
}
beforeEach(() => {
  platformCapabilityStore.setState({ capabilities: capabilityFixture('macos'), loaded: true });
  localStorage.clear();
  useAuthStore.setState({ currentAccount: null, isAuthenticated: false });
  useOcrScanStore.getState().clearOnVaultLock();
  useAuthStore.getState().completeUnlock({ id: 'rf029-a', name: 'Synthetic A' });
  mocks.ipc.mockReset();
  mocks.toast.mockReset();
  mocks.listen.mockReset().mockImplementation((_event, callback) => {
    listener = callback;
    return Promise.resolve(vi.fn());
  });
  releases = [];
  works = [];
});
afterEach(async () => {
  await act(async () => {
    releases.forEach((release) => release());
    await Promise.allSettled(works);
  });
  cleanup();
  useOcrScanStore.getState().clearOnVaultLock();
  useAuthStore.setState({ currentAccount: null, isAuthenticated: false });
});

describe('RF029 用户操作完成通知', () => {
  it('两处监听也只通知一次，MRZ completed 及后继进度不会提前通知', async () => {
    const mrz = pending<null>(null);
    const image = pending(RESULT);
    mocks.ipc.mockImplementation((command) =>
      command === 'ocr_scan_mrz' ? mrz.promise : image.promise,
    );
    const view = render(
      <>
        <OcrScanNotificationListener />
        <OcrScanNotificationListener />
      </>,
    );
    const work = start('mrz');
    const first = await task();
    event(first, 'completed', 1);
    expect(useOcrScanStore.getState().isScanning).toBe(true);
    expect(mocks.toast).not.toHaveBeenCalled();
    await act(async () => {
      mrz.resolve(null);
    });
    const second = await task(1);
    expect(second).not.toBe(first);
    expect(mocks.toast).not.toHaveBeenCalled();
    event(second, 'completed', 1);
    expect(mocks.toast).not.toHaveBeenCalled();
    await act(async () => {
      image.resolve(RESULT);
      await work;
    });
    expect(mocks.toast).toHaveBeenCalledTimes(1);
    expect(mocks.toast).toHaveBeenCalledWith(
      expect.objectContaining({ type: 'success', message: 'scan_complete_notification' }),
    );
    expect(useOcrScanStore.getState().scanHistory[0].result).toEqual(RESULT);
    event(second, 'completed', 2);
    view.rerender(
      <>
        <OcrScanNotificationListener />
        <OcrScanNotificationListener />
      </>,
    );
    expect(mocks.toast).toHaveBeenCalledTimes(1);
  });

  it('锁定清空及旧事件、旧 invoke 成功均不触发完成通知', async () => {
    const reply = pending(RESULT);
    mocks.ipc.mockReturnValue(reply.promise);
    render(<OcrScanNotificationListener />);
    const work = start('general');
    const taskId = await task();
    act(() => {
      useAuthStore.setState({ currentAccount: null, isAuthenticated: false });
      useOcrScanStore.getState().clearOnVaultLock();
      useAuthStore.getState().completeUnlock({ id: 'rf029-b', name: 'Synthetic B' });
    });
    expect(mocks.toast).not.toHaveBeenCalled();
    event(taskId, 'completed', 3);
    await act(async () => {
      reply.resolve(RESULT);
      await work;
    });
    expect(mocks.toast).not.toHaveBeenCalled();
    expect(useOcrScanStore.getState().scanHistory).toEqual([]);
    expect(useOcrScanStore.getState().lastCompletion).toBeNull();
  });

  it('cancelRequested 和 cancelled 事件均不提前结束，invoke 确认取消后也不报成功', async () => {
    const reply = pending(RESULT);
    mocks.ipc.mockImplementation((command) =>
      command === 'ocr_cancel_scan' ? Promise.resolve(true) : reply.promise,
    );
    render(<OcrScanNotificationListener />);
    const work = start('general');
    const taskId = await task();
    act(() => useOcrScanStore.getState().cancelScan());
    event(taskId, 'cancelRequested', 1);
    event(taskId, 'cancelled', 2);
    expect(useOcrScanStore.getState().isScanning).toBe(true);
    expect(useOcrScanStore.getState().scanState).toBe('cancelRequested');
    expect(mocks.toast).not.toHaveBeenCalled();
    await act(async () => {
      reply.reject(new Error('__OCR_CANCELLED__'));
      await work;
    });
    expect(useOcrScanStore.getState().isScanning).toBe(false);
    expect(useOcrScanStore.getState().scanState).toBe('cancelled');
    expect(mocks.toast).not.toHaveBeenCalled();
  });
});
