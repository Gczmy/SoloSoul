import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { MemoryRouter } from 'react-router-dom';
import { useAuthStore } from '@/stores/authStore';
import { useOcrScanStore } from '@/stores/ocrScanStore';
import { useUiStore } from '@/stores/uiStore';
import { OcrScanNotificationListener } from '@/components/layout/OcrScanNotificationListener';
import { OcrQuickScanPopover } from '@/components/layout/OcrQuickScanPopover';
import { OcrPage } from './OcrPage';

const mocks = vi.hoisted(() => ({
  ipc: vi.fn(),
  open: vi.fn(),
  error: vi.fn(),
  success: vi.fn(),
  mobile: false,
}));
vi.mock('@tauri-apps/api/core', () => ({ invoke: mocks.ipc }));
vi.mock('@/lib/dialog', () => ({ openWithPause: mocks.open }));
vi.mock('@/lib/logger', () => ({
  logger: { warn: vi.fn(), error: vi.fn(), info: vi.fn(), debug: vi.fn() },
}));
vi.mock('@/hooks/useToastError', () => ({
  useToastError: () => ({ onError: mocks.error, onSuccess: mocks.success }),
}));
vi.mock('@/lib/platform', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/lib/platform')>()),
  isMobilePlatformSync: () => mocks.mobile,
  isMacOSSync: () => false,
}));
vi.mock('@/hooks/useOcrModelManager', () => ({
  useOcrModelManager: () => ({
    tiers: [{ tier: 'small', name: 'Small', description: 'Synthetic test model' }],
    activeTier: 'small',
    statusMap: { small: { installed: true, bundled: true, tier: 'small' } },
    loading: false,
    installingTier: null,
    downloadingTier: null,
    downloadUrl: '',
    setDownloadUrl: vi.fn(),
    handleTierChange: vi.fn(),
    handleInstallBundled: vi.fn(),
    handleDownload: vi.fn(),
  }),
}));
vi.mock('@/lib/prefetch/registry', () => ({ prefetchRegistry: { ocrModel: {} } }));
vi.mock('@/lib/prefetch/usePrefetchData', () => ({
  usePrefetchData: () => ({
    data: {
      tiers: [{ tier: 'small', name: 'Small', description: 'Synthetic test model' }],
      statusMap: { small: { installed: true, bundled: true, tier: 'small' } },
    },
    loading: false,
    error: null,
  }),
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
let releases: (() => void)[];
function scanCalls() {
  return mocks.ipc.mock.calls.filter(
    ([name]) => name === 'ocr_scan_image' || name === 'ocr_scan_mrz',
  );
}
// 导航容器的边界夹具：完成定位后传入有效坐标，真实关闭按钮改变 Store 并卸载卡片。
// Store、Popover、扫描 helper、通知消费者均保留生产实现。
function QuickScanFixture() {
  const open = useOcrScanStore((state) => state.isCardOpen);
  return (
    <>
      <button data-ocr-button onClick={() => useOcrScanStore.getState().setCardOpen(true)}>
        RF029 open quick scan
      </button>
      {open && (
        <OcrQuickScanPopover
          position={{ top: 24 }}
          onClose={() => useOcrScanStore.getState().setCardOpen(false)}
        />
      )}
      <OcrScanNotificationListener />
      <OcrScanNotificationListener />
    </>
  );
}
function clearToasts() {
  for (const toast of useUiStore.getState().toasts) useUiStore.getState().dismissToast(toast.id);
}
function switchAccount() {
  act(() => {
    useAuthStore.setState({ isAuthenticated: false, currentAccount: null });
    useOcrScanStore.getState().clearOnVaultLock();
    useAuthStore.getState().completeUnlock({ id: 'rf029-b', name: 'Synthetic B' });
  });
}
beforeEach(() => {
  mocks.ipc.mockReset().mockResolvedValue(undefined);
  mocks.open.mockReset();
  mocks.error.mockReset();
  mocks.success.mockReset();
  mocks.mobile = false;
  clearToasts();
  localStorage.clear();
  useAuthStore.setState({ isAuthenticated: false, currentAccount: null });
  useOcrScanStore.getState().clearOnVaultLock();
  useAuthStore.getState().completeUnlock({ id: 'rf029-a', name: 'Synthetic A' });
  releases = [];
});
afterEach(async () => {
  await act(async () => {
    releases.forEach((release) => release());
  });
  cleanup();
  clearToasts();
  useOcrScanStore.getState().clearOnVaultLock();
  useAuthStore.setState({ isAuthenticated: false, currentAccount: null });
});

describe('RF029 OCR 入口等待期间的会话隔离', () => {
  it.each([
    ['page', 'switch'],
    ['page', 'unmount'],
    ['quick', 'switch'],
    ['quick', 'unmount'],
  ] as const)('%s 文件选择悬停后 %s，不发起跨会话或卸载后的扫描', async (entry, change) => {
    const selected = deferred<string | null>();
    releases.push(() => selected.resolve(null));
    mocks.open.mockReturnValue(selected.promise);
    const view = render(
      <MemoryRouter>
        {entry === 'page' ? (
          <OcrPage />
        ) : (
          <OcrQuickScanPopover position={{ top: 24 }} onClose={vi.fn()} />
        )}
      </MemoryRouter>,
    );
    const selectFile = await screen.findByRole('button', { name: 'ocr:select_image_or_pdf' });
    expect(selectFile).toBeVisible();
    fireEvent.click(selectFile);
    await waitFor(() => expect(mocks.open).toHaveBeenCalledTimes(1));
    if (change === 'switch') switchAccount();
    else view.unmount();
    await act(async () => {
      selected.resolve('C:/RF029-synthetic/old-account-picker.png');
    });
    expect(scanCalls()).toHaveLength(0);
    expect(mocks.error).not.toHaveBeenCalled();
    expect(mocks.success).not.toHaveBeenCalled();
    expect(useOcrScanStore.getState().scanHistory).toEqual([]);
  });

  it.each(['while-running', 'after-completion'] as const)(
    '快捷卡 %s 时关闭：后台完成只通知一次，卡内完成不补通知',
    async (closeAt) => {
      const result = { text: 'RF029 synthetic quick-card result', confidence: 0.95, boxes: [] };
      const reply = deferred<typeof result>();
      releases.push(() => reply.resolve(result));
      mocks.open.mockResolvedValue('C:/RF029-synthetic/quick-card.png');
      mocks.ipc.mockImplementation((command) =>
        command === 'ocr_scan_image' ? reply.promise : Promise.resolve(true),
      );
      useOcrScanStore.getState().setScanMode('general');
      render(
        <MemoryRouter>
          <QuickScanFixture />
        </MemoryRouter>,
      );
      fireEvent.click(screen.getByRole('button', { name: 'RF029 open quick scan' }));
      const selectFile = screen.getByRole('button', { name: 'ocr:select_image_or_pdf' });
      expect(selectFile).toBeVisible();
      fireEvent.click(selectFile);
      await waitFor(() => expect(scanCalls()).toHaveLength(1));
      expect(useOcrScanStore.getState().isScanning).toBe(true);
      expect(useUiStore.getState().toasts).toHaveLength(0);
      if (closeAt === 'while-running') {
        fireEvent.click(screen.getByRole('button', { name: 'common:close' }));
        expect(
          screen.queryByRole('button', { name: 'ocr:select_image_or_pdf' }),
        ).not.toBeInTheDocument();
        expect(useOcrScanStore.getState().isCardOpen).toBe(false);
        expect(useOcrScanStore.getState().isScanning).toBe(true);
      }
      await act(async () => {
        reply.resolve(result);
      });
      await waitFor(() => expect(useOcrScanStore.getState().isScanning).toBe(false));
      expect(useOcrScanStore.getState().scanHistory[0].result).toEqual(result);
      if (closeAt === 'after-completion') {
        expect(screen.getByText(result.text)).toBeVisible();
        expect(useUiStore.getState().toasts).toHaveLength(0);
        fireEvent.click(screen.getByRole('button', { name: 'common:close' }));
      }
      const expectedCount = closeAt === 'while-running' ? 1 : 0;
      expect(useUiStore.getState().toasts).toHaveLength(expectedCount);
      if (expectedCount)
        expect(useUiStore.getState().toasts[0]).toEqual(
          expect.objectContaining({
            type: 'success',
            message: 'scan_complete_notification',
          }),
        );
      expect(mocks.ipc).not.toHaveBeenCalledWith('ocr_cancel_scan', expect.anything());
      // 重新打开、关闭也不能重复消费这次已接纳的终态。
      fireEvent.click(screen.getByRole('button', { name: 'RF029 open quick scan' }));
      expect(screen.getByText(result.text)).toBeVisible();
      fireEvent.click(screen.getByRole('button', { name: 'common:close' }));
      expect(useUiStore.getState().toasts).toHaveLength(expectedCount);
      expect(scanCalls()).toHaveLength(1);
    },
  );
  it('页面相机悬停后切账户，旧照片不得启动新账户扫描', async () => {
    mocks.mobile = true;
    const photo = deferred<string | null>();
    releases.push(() => photo.resolve(null));
    mocks.ipc.mockImplementation((name) =>
      name === 'mobile_ocr_take_photo' ? photo.promise : Promise.resolve(undefined),
    );
    render(
      <MemoryRouter>
        <OcrPage />
      </MemoryRouter>,
    );
    fireEvent.click(await screen.findByRole('button', { name: 'ocr:take_photo' }));
    await waitFor(() => expect(mocks.ipc).toHaveBeenCalledWith('mobile_ocr_take_photo'));
    switchAccount();
    await act(async () => {
      photo.resolve('C:/RF029-synthetic/old-camera.png');
    });
    expect(scanCalls()).toHaveLength(0);
    expect(mocks.error).not.toHaveBeenCalled();
    expect(mocks.success).not.toHaveBeenCalled();
  });

  it('页面取消后仍等待 invoke，取消完成不显示错误或成功提示', async () => {
    const result = { text: 'RF029 synthetic cancelled OCR', confidence: 1, boxes: [] };
    const reply = deferred<typeof result>();
    releases.push(() => reply.resolve(result));
    mocks.ipc.mockImplementation((name) =>
      name === 'ocr_cancel_scan' ? Promise.resolve(true) : reply.promise,
    );
    render(
      <MemoryRouter
        initialEntries={[
          {
            pathname: '/scan',
            state: { filePath: 'C:/RF029-synthetic/cancel.png' },
          },
        ]}
      >
        <OcrPage />
      </MemoryRouter>,
    );
    await waitFor(() => expect(scanCalls()).toHaveLength(1));
    fireEvent.click(screen.getByRole('button', { name: 'scan_cancel' }));
    expect(screen.getByRole('status')).toHaveTextContent('scan_cancelling');
    expect(screen.getByRole('button', { name: 'scan_cancel' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'ocr:select_image_or_pdf' })).toBeDisabled();
    await act(async () => {
      reply.reject(new Error('__OCR_CANCELLED__'));
    });
    expect(screen.getByRole('status')).toHaveTextContent('scan_cancelled');
    expect(screen.getByRole('button', { name: 'ocr:select_image_or_pdf' })).toBeEnabled();
    expect(mocks.error).not.toHaveBeenCalled();
    expect(mocks.success).not.toHaveBeenCalled();
  });
  it('附件自动扫描在页面卸载时请求取消，迟到 invoke 成功没有页面副作用', async () => {
    const result = { text: 'RF029 synthetic attachment OCR', confidence: 1, boxes: [] };
    const reply = deferred<typeof result>();
    releases.push(() => reply.resolve(result));
    mocks.ipc.mockImplementation((name) =>
      name === 'ocr_cancel_scan' ? Promise.resolve(true) : reply.promise,
    );
    const view = render(
      <MemoryRouter
        initialEntries={[
          {
            pathname: '/scan',
            state: { filePath: 'C:/RF029-synthetic/attachment.png' },
          },
        ]}
      >
        <OcrPage />
      </MemoryRouter>,
    );
    await waitFor(() => expect(scanCalls()).toHaveLength(1));
    const taskId = scanCalls()[0][1].taskId;
    expect(taskId).toEqual(expect.any(String));
    view.unmount();
    await waitFor(() => expect(mocks.ipc).toHaveBeenCalledWith('ocr_cancel_scan', { taskId }));
    await act(async () => {
      reply.resolve(result);
    });
    expect(scanCalls()).toHaveLength(1);
    expect(mocks.error).not.toHaveBeenCalled();
    expect(mocks.success).not.toHaveBeenCalled();
  });
});
