import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { OcrSettingsPage } from './OcrSettingsPage';

const mockShowToast = vi.fn();

vi.mock('@/components/layout/PageShell', () => ({
  PageShell: ({
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

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

import { invoke } from '@tauri-apps/api/core';

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

const mockInvoke = vi.mocked(invoke);

describe('OcrSettingsPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockInvoke.mockImplementation(async (cmd: string, args?: unknown) => {
      if (cmd === 'ocr_list_available_tiers')
        return [
          { tier: 'tiny', name: 'Tiny', description: 'Fast' },
          { tier: 'small', name: 'Small', description: 'Default' },
          { tier: 'medium', name: 'Medium', description: 'Accurate' },
        ];
      if (cmd === 'ocr_get_active_tier') return 'small';
      if (cmd === 'ocr_get_model_status') {
        const tier = ((args as Record<string, unknown>).tier as string) ?? 'small';
        return { tier, installed: tier === 'small', bundled: tier === 'small' };
      }
      return undefined;
    });
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
      expect(mockInvoke).toHaveBeenCalledWith('ocr_list_available_tiers');
      expect(mockInvoke).toHaveBeenCalledWith('ocr_get_active_tier');
      expect(mockInvoke).toHaveBeenCalledWith('ocr_get_model_status', { tier: 'small' });
    });

    expect(await screen.findByText('Tiny')).toBeInTheDocument();
    expect(screen.getByText('Small')).toBeInTheDocument();
    expect(screen.getByText('Medium')).toBeInTheDocument();
  });

  it('changes active tier on select change', async () => {
    render(
      <MemoryRouter>
        <OcrSettingsPage />
      </MemoryRouter>,
    );

    const select = await screen.findByDisplayValue('Small — Default');
    fireEvent.change(select, { target: { value: 'medium' } });

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('ocr_set_active_tier', { tier: 'medium' });
    });
  });

  it('installs bundled model when install clicked', async () => {
    render(
      <MemoryRouter>
        <OcrSettingsPage />
      </MemoryRouter>,
    );

    await screen.findByText('Tiny');
    // Tiny is not installed/bundled in default mock, so no install button.
    // Re-render with updated mock status
    mockInvoke.mockImplementation(async (cmd: string, args?: unknown) => {
      if (cmd === 'ocr_list_available_tiers')
        return [
          { tier: 'tiny', name: 'Tiny', description: 'Fast' },
          { tier: 'small', name: 'Small', description: 'Default' },
          { tier: 'medium', name: 'Medium', description: 'Accurate' },
        ];
      if (cmd === 'ocr_get_active_tier') return 'small';
      if (cmd === 'ocr_get_model_status') {
        const tier = ((args as Record<string, unknown>).tier as string) ?? 'small';
        return {
          tier,
          installed: tier === 'small',
          bundled: tier === 'small' || tier === 'medium',
        };
      }
      return undefined;
    });

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
      expect(mockInvoke).toHaveBeenCalledWith('ocr_install_bundled_model', expect.anything());
    });
  });
});
