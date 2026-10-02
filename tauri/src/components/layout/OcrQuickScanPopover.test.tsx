import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { OcrQuickScanPopover } from './OcrQuickScanPopover';
import { useOcrScanStore, type OcrScanEntry } from '@/stores/ocrScanStore';

const mocks = vi.hoisted(() => ({
  openWithPause: vi.fn(),
  invokeCommand: vi.fn(),
  onError: vi.fn(),
  ios: false,
}));

vi.mock('@/lib/platform', () => ({
  isIOSSync: () => mocks.ios,
  isMobilePlatformSync: () => mocks.ios,
  isMacOSSync: () => false,
  isAndroidSync: () => false,
}));
vi.mock('@/lib/dialog', () => ({ openWithPause: mocks.openWithPause }));
vi.mock('@/lib/ipcClient', () => ({ invokeCommand: mocks.invokeCommand }));
vi.mock('@/hooks/useToastError', () => ({
  useToastError: () => ({ onError: mocks.onError }),
}));
vi.mock('@/lib/prefetch/usePrefetchData', () => ({
  usePrefetchData: () => ({
    data: {
      tiers: [
        { tier: 'small', name: 'Small', description: 'Fast' },
        { tier: 'medium', name: 'Medium', description: 'Accurate' },
      ],
      statusMap: {
        small: { tier: 'small', installed: true, bundled: true },
        medium: { tier: 'medium', installed: true, bundled: false },
      },
    },
    loading: false,
    error: null,
  }),
}));

const originalPerformScan = useOcrScanStore.getState().performScan;
const scan = vi.fn(async (_path: string) => {});

function showPopover(onClose = vi.fn()) {
  return render(
    <MemoryRouter>
      <OcrQuickScanPopover position={{ top: 50 }} onClose={onClose} />
    </MemoryRouter>,
  );
}

beforeEach(() => {
  mocks.ios = false;
  vi.clearAllMocks();
  mocks.invokeCommand.mockResolvedValue(undefined);
  act(() => {
    useOcrScanStore.setState({
      scanMode: 'general',
      scanState: null,
      currentScanId: null,
      isScanning: false,
      activeTier: 'small',
      lastScanError: null,
      scanHistory: [],
      performScan: scan,
    });
  });
});

afterEach(() => {
  act(() => {
    useOcrScanStore.setState({ performScan: originalPerformScan, scanHistory: [] });
  });
});

describe('RF-1029 OCR quick scan interactions', () => {
  it('selects a PDF-capable file in general mode and image-only file in MRZ mode', async () => {
    mocks.openWithPause.mockResolvedValueOnce('C:/sample.pdf').mockResolvedValueOnce('C:/mrz.jpg');
    showPopover();

    fireEvent.click(screen.getByRole('button', { name: 'ocr:select_image_or_pdf' }));
    await waitFor(() => expect(scan).toHaveBeenCalledWith('C:/sample.pdf'));
    expect(mocks.openWithPause.mock.calls[0][0].filters[0].extensions).toContain('pdf');

    fireEvent.click(screen.getByRole('button', { name: 'ocr:scan_mode_mrz' }));
    fireEvent.click(screen.getByRole('button', { name: 'ocr:select_image' }));
    await waitFor(() => expect(scan).toHaveBeenCalledWith('C:/mrz.jpg'));
    expect(mocks.openWithPause.mock.calls[1][0].filters[0].extensions).not.toContain('pdf');
  });

  it('does not start a scan when the picker is cancelled or resolves after closing', async () => {
    let resolvePicker!: (path: string) => void;
    mocks.openWithPause
      .mockResolvedValueOnce(null)
      .mockImplementationOnce(() => new Promise<string>((resolve) => (resolvePicker = resolve)));
    const { unmount } = showPopover();

    fireEvent.click(screen.getByRole('button', { name: 'ocr:select_image_or_pdf' }));
    await waitFor(() => expect(mocks.openWithPause).toHaveBeenCalledTimes(1));
    expect(scan).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole('button', { name: 'ocr:select_image_or_pdf' }));
    await waitFor(() => expect(mocks.openWithPause).toHaveBeenCalledTimes(2));
    unmount();
    await act(async () => resolvePicker('C:/late.png'));
    expect(scan).not.toHaveBeenCalled();
  });

  it('updates the OCR tier only after the backend accepts it', async () => {
    showPopover();
    fireEvent.change(screen.getByRole('combobox'), { target: { value: 'medium' } });
    await waitFor(() => expect(useOcrScanStore.getState().activeTier).toBe('medium'));
    expect(mocks.invokeCommand).toHaveBeenCalledWith('ocr_set_active_tier', { tier: 'medium' });
  });

  it('reports picker and tier failures without scanning or changing the active tier', async () => {
    mocks.openWithPause.mockRejectedValueOnce(new Error('picker failed'));
    mocks.invokeCommand.mockRejectedValueOnce(new Error('tier failed'));
    showPopover();

    fireEvent.click(screen.getByRole('button', { name: 'ocr:select_image_or_pdf' }));
    await waitFor(() =>
      expect(mocks.onError).toHaveBeenCalledWith(expect.any(Error), 'ocr:select_image_failed'),
    );
    expect(scan).not.toHaveBeenCalled();

    fireEvent.change(screen.getByRole('combobox'), { target: { value: 'medium' } });
    await waitFor(() =>
      expect(mocks.onError).toHaveBeenCalledWith(expect.any(Error), 'ocr:set_tier_failed'),
    );
    expect(useOcrScanStore.getState().activeTier).toBe('small');
  });

  it('restores recent history and lets the user select another scan', () => {
    const entry = (id: string, fileName: string): OcrScanEntry => ({
      id,
      fileName,
      filePath: `C:/${fileName}`,
      timestamp: 1_700_000_000_000,
      mode: 'general',
      result: null,
      mrzResult: null,
      isDeleted: false,
    });
    act(() => {
      useOcrScanStore.setState({
        currentScanId: null,
        scanHistory: [entry('new', 'new.png'), entry('old', 'old.png')],
      });
    });
    showPopover();
    expect(useOcrScanStore.getState().currentScanId).toBe('new');

    fireEvent.click(screen.getByTitle('ocr:scan_history'));
    fireEvent.click(screen.getByTitle('old.png'));
    expect(useOcrScanStore.getState().currentScanId).toBe('old');
    expect(screen.queryByTitle('new.png')).not.toBeInTheDocument();
  });
});

describe('RF-203 iOS 快捷扫描', () => {
  it('禁用扫描和模型切换，保留历史入口并显示原因', () => {
    mocks.ios = true;
    showPopover();
    expect(screen.getByText('ocr:ios_ocr_unsupported')).toBeVisible();
    expect(screen.getByRole('button', { name: 'ocr:select_image' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'ocr:scan_mode_mrz' })).toBeDisabled();
    expect(screen.getByRole('combobox')).toBeDisabled();
    fireEvent.click(screen.getByRole('button', { name: 'ocr:select_image' }));
    expect(mocks.openWithPause).not.toHaveBeenCalled();
    expect(scan).not.toHaveBeenCalled();
  });
});
