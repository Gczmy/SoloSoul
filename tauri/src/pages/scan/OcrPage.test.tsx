import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { OcrPage } from './OcrPage';

const mockNavigate = vi.fn();
const mockShowToast = vi.fn();
const mockCreateObject = vi.fn();
const mockOpen = vi.fn();

vi.mock('@/components/layout/AppShell', () => ({
  AppShell: ({
    children,
    title,
    onBack,
  }: {
    children: React.ReactNode;
    title: string;
    onBack?: () => void;
  }) => (
    <div data-testid="app-shell" data-title={title}>
      {onBack && (
        <button data-testid="back-btn" onClick={onBack}>
          Back
        </button>
      )}
      {children}
    </div>
  ),
}));

vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual('react-router-dom');
  return {
    ...actual,
    useNavigate: () => mockNavigate,
    useLocation: () => ({ state: {} }),
  };
});

vi.mock('@/lib/ipc', () => ({
  commands: {
    ocrListAvailableTiers: vi.fn(),
    ocrGetActiveTier: vi.fn(),
    ocrGetModelStatus: vi.fn(),
    ocrSetActiveTier: vi.fn(),
    ocrInstallBundledModel: vi.fn(),
    ocrDownloadModel: vi.fn(),
    ocrScanImage: vi.fn(),
  },
}));

import { commands } from '@/lib/ipc';

vi.mock('@/stores/authStore', () => ({
  useAuthStore: (selector: (s: { currentAccount: { id: string } | null }) => unknown) =>
    selector({ currentAccount: { id: 'test-account' } }),
}));

vi.mock('@/stores/objectStore', () => ({
  useObjectStore: () => ({
    createObject: mockCreateObject,
  }),
}));

vi.mock('@/hooks/useToastError', () => ({
  useToastError: () => ({
    onError: (e: unknown, ctx: string) => {
      mockShowToast({ type: 'error', message: `${ctx}: ${e}` });
    },
    onSuccess: (msg: string) => {
      mockShowToast({ type: 'success', message: msg });
    },
  }),
}));

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: () => mockOpen(),
}));

const mockCmd = commands as unknown as {
  ocrListAvailableTiers: ReturnType<typeof vi.fn>;
  ocrGetActiveTier: ReturnType<typeof vi.fn>;
  ocrGetModelStatus: ReturnType<typeof vi.fn>;
  ocrSetActiveTier: ReturnType<typeof vi.fn>;
  ocrInstallBundledModel: ReturnType<typeof vi.fn>;
  ocrDownloadModel: ReturnType<typeof vi.fn>;
  ocrScanImage: ReturnType<typeof vi.fn>;
};

describe('OcrPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockCmd.ocrListAvailableTiers.mockResolvedValue([
      { tier: 'tiny', name: 'Tiny', description: 'Fast' },
      { tier: 'small', name: 'Small', description: 'Default' },
      { tier: 'medium', name: 'Medium', description: 'Accurate' },
    ]);
    mockCmd.ocrGetActiveTier.mockResolvedValue('small');
    mockCmd.ocrGetModelStatus.mockResolvedValue({
      tier: 'small',
      installed: true,
      bundled: true,
    });
  });

  it('renders scanner title and select image button', async () => {
    render(
      <MemoryRouter>
        <OcrPage />
      </MemoryRouter>,
    );

    expect(screen.getByTestId('app-shell')).toHaveAttribute('data-title', 'ocr:title');
    expect(await screen.findByText('ocr:select_image')).toBeInTheDocument();
  });

  it('loads model tiers and status on mount', async () => {
    render(
      <MemoryRouter>
        <OcrPage />
      </MemoryRouter>,
    );

    await waitFor(() => {
      expect(mockCmd.ocrListAvailableTiers).toHaveBeenCalled();
      expect(mockCmd.ocrGetActiveTier).toHaveBeenCalled();
      expect(mockCmd.ocrGetModelStatus).toHaveBeenCalledWith('small');
    });
  });

  it('scans selected image and displays result', async () => {
    mockOpen.mockResolvedValue('/test/image.png');
    mockCmd.ocrScanImage.mockResolvedValue({
      text: 'Hello World',
      confidence: 0.95,
      boxes: [{ text: 'Hello World', confidence: 0.95, points: [] }],
    });

    render(
      <MemoryRouter>
        <OcrPage />
      </MemoryRouter>,
    );

    await waitFor(() => {
      expect(screen.getByText('ocr:select_image')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText('ocr:select_image'));

    await waitFor(() => {
      expect(mockCmd.ocrScanImage).toHaveBeenCalledWith('/test/image.png');
    });

    const results = await screen.findAllByText('Hello World');
    expect(results.length).toBeGreaterThanOrEqual(1);
  });

  it('imports scan result as object', async () => {
    mockOpen.mockResolvedValue('/test/image.png');
    mockCmd.ocrScanImage.mockResolvedValue({
      text: 'Hello World',
      confidence: 0.95,
      boxes: [{ text: 'Hello World', confidence: 0.95, points: [] }],
    });
    mockCreateObject.mockResolvedValue({});

    render(
      <MemoryRouter>
        <OcrPage />
      </MemoryRouter>,
    );

    fireEvent.click(await screen.findByText('ocr:select_image'));
    await screen.findAllByText('Hello World');

    fireEvent.click(screen.getByText('ocr:import_as_object'));

    await waitFor(() => {
      expect(mockCreateObject).toHaveBeenCalledWith({
        accountId: 'test-account',
        name: 'image.png',
        collectionType: 'document',
        properties: { ocrText: 'Hello World' },
      });
    });
  });

  it('shows not-installed toast when active model is not installed', async () => {
    mockCmd.ocrGetActiveTier.mockResolvedValue('tiny');
    mockCmd.ocrGetModelStatus.mockResolvedValue({
      tier: 'tiny',
      installed: false,
      bundled: true,
    });
    mockOpen.mockResolvedValue('/test/image.png');

    render(
      <MemoryRouter>
        <OcrPage />
      </MemoryRouter>,
    );

    fireEvent.click(await screen.findByText('ocr:select_image'));

    await waitFor(() => {
      expect(mockShowToast).toHaveBeenCalledWith(
        expect.objectContaining({
          type: 'error',
          message: expect.stringContaining('ocr:scan_model_not_installed'),
        }),
      );
    });
    expect(mockCmd.ocrScanImage).not.toHaveBeenCalled();
  });

  it('shows error toast when scan fails', async () => {
    mockOpen.mockResolvedValue('/test/image.png');
    mockCmd.ocrScanImage.mockRejectedValue(new Error('model not found'));

    render(
      <MemoryRouter>
        <OcrPage />
      </MemoryRouter>,
    );

    fireEvent.click(await screen.findByText('ocr:select_image'));

    await waitFor(() => {
      expect(mockShowToast).toHaveBeenCalledWith(
        expect.objectContaining({
          type: 'error',
          message: expect.stringContaining('ocr:scan_failed'),
        }),
      );
    });
  });
});
