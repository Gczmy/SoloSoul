import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { OcrSettingsPage } from './OcrSettingsPage';

const mockShowToast = vi.fn();

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

vi.mock('@/lib/ipc', () => ({
  commands: {
    ocrListAvailableTiers: vi.fn(),
    ocrGetActiveTier: vi.fn(),
    ocrGetModelStatus: vi.fn(),
    ocrSetActiveTier: vi.fn(),
    ocrInstallBundledModel: vi.fn(),
    ocrDownloadModel: vi.fn(),
  },
}));

import { commands } from '@/lib/ipc';

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

const mockCmd = commands as unknown as {
  ocrListAvailableTiers: ReturnType<typeof vi.fn>;
  ocrGetActiveTier: ReturnType<typeof vi.fn>;
  ocrGetModelStatus: ReturnType<typeof vi.fn>;
  ocrSetActiveTier: ReturnType<typeof vi.fn>;
  ocrInstallBundledModel: ReturnType<typeof vi.fn>;
  ocrDownloadModel: ReturnType<typeof vi.fn>;
};

describe('OcrSettingsPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockCmd.ocrListAvailableTiers.mockResolvedValue([
      { tier: 'tiny', name: 'Tiny', description: 'Fast' },
      { tier: 'small', name: 'Small', description: 'Default' },
      { tier: 'medium', name: 'Medium', description: 'Accurate' },
    ]);
    mockCmd.ocrGetActiveTier.mockResolvedValue('small');
    mockCmd.ocrGetModelStatus.mockImplementation((tier: string) =>
      Promise.resolve({
        tier,
        installed: tier === 'small',
        bundled: tier === 'small',
      }),
    );
  });

  it('renders settings title and active model select', async () => {
    render(
      <MemoryRouter>
        <OcrSettingsPage />
      </MemoryRouter>,
    );

    expect(screen.getByTestId('app-shell')).toHaveAttribute('data-title', 'ocr:settings_title');
    expect(await screen.findByText('ocr:active_model')).toBeInTheDocument();
  });

  it('loads tiers and statuses on mount', async () => {
    render(
      <MemoryRouter>
        <OcrSettingsPage />
      </MemoryRouter>,
    );

    await waitFor(() => {
      expect(mockCmd.ocrListAvailableTiers).toHaveBeenCalled();
      expect(mockCmd.ocrGetActiveTier).toHaveBeenCalled();
      expect(mockCmd.ocrGetModelStatus).toHaveBeenCalledWith('small');
    });

    expect(await screen.findByText('Tiny')).toBeInTheDocument();
    expect(screen.getByText('Small')).toBeInTheDocument();
    expect(screen.getByText('Medium')).toBeInTheDocument();
  });

  it('changes active tier on select change', async () => {
    mockCmd.ocrSetActiveTier.mockResolvedValue(undefined);

    render(
      <MemoryRouter>
        <OcrSettingsPage />
      </MemoryRouter>,
    );

    const select = await screen.findByDisplayValue('Small — Default');
    fireEvent.change(select, { target: { value: 'medium' } });

    await waitFor(() => {
      expect(mockCmd.ocrSetActiveTier).toHaveBeenCalledWith('medium');
    });
  });

  it('installs bundled model when install clicked', async () => {
    mockCmd.ocrInstallBundledModel.mockResolvedValue(undefined);

    render(
      <MemoryRouter>
        <OcrSettingsPage />
      </MemoryRouter>,
    );

    await screen.findByText('Tiny');
    // Tiny is not installed/bundled in default mock, so no install button.
    // Change medium to bundled+not-installed to test install.
    mockCmd.ocrGetModelStatus.mockImplementation((tier: string) =>
      Promise.resolve({
        tier,
        installed: tier === 'small',
        bundled: tier === 'small' || tier === 'medium',
      }),
    );

    // Re-render with updated mock
    render(
      <MemoryRouter>
        <OcrSettingsPage />
      </MemoryRouter>,
    );

    await waitFor(() => {
      const installButtons = screen.queryAllByText('ocr:install');
      expect(installButtons.length).toBeGreaterThan(0);
    });

    fireEvent.click(screen.getAllByText('ocr:install')[0]);

    await waitFor(() => {
      expect(mockCmd.ocrInstallBundledModel).toHaveBeenCalled();
    });
  });
});
