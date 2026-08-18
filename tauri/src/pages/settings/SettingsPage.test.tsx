import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { MemoryRouter, useNavigate } from 'react-router-dom';
import { SettingsPage } from './SettingsPage';
import { invoke } from '@tauri-apps/api/core';

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

vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual('react-router-dom');
  return {
    ...actual,
    useNavigate: vi.fn(),
  };
});

describe('SettingsPage', () => {
  const navigate = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useNavigate).mockReturnValue(navigate);
  });

  it('renders setting groups', async () => {
    vi.mocked(invoke).mockResolvedValue({ totalSizeBytes: 0 });
    render(
      <MemoryRouter>
        <SettingsPage />
      </MemoryRouter>,
    );

    expect(screen.getByTestId('app-shell')).toHaveAttribute('data-title', 'settings:title');
    expect(await screen.findByText('settings:items.theme_appearance')).toBeInTheDocument();
    expect(screen.getByText('settings:items.security_settings')).toBeInTheDocument();
    expect(screen.getByText('settings:items.data_management')).toBeInTheDocument();
    expect(screen.getByText('settings:items.about')).toBeInTheDocument();
  });

  it('navigates to sub-settings on card click', async () => {
    vi.mocked(invoke).mockResolvedValue({ totalSizeBytes: 0 });
    render(
      <MemoryRouter>
        <SettingsPage />
      </MemoryRouter>,
    );

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('get_vault_stats');
    });

    const themeCard = screen
      .getByText('settings:items.theme_appearance')
      .closest('[role="button"]') as HTMLElement;
    fireEvent.click(themeCard);
    expect(navigate).toHaveBeenCalledWith('/settings/appearance', { state: { from: '/settings' } });

    const aboutCard = screen
      .getByText('settings:items.about')
      .closest('[role="button"]') as HTMLElement;
    fireEvent.click(aboutCard);
    expect(navigate).toHaveBeenCalledWith('/about', { state: { from: '/settings' } });
  });

  it('calls back navigation', async () => {
    vi.mocked(invoke).mockResolvedValue({ totalSizeBytes: 0 });
    render(
      <MemoryRouter>
        <SettingsPage />
      </MemoryRouter>,
    );

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('get_vault_stats');
    });

    const backBtn = screen.getByTestId('back-btn');
    fireEvent.click(backBtn);
    expect(navigate).toHaveBeenCalledWith('/home');
  });

  it('displays vault size badge when loaded', async () => {
    vi.mocked(invoke).mockResolvedValue({ totalSizeBytes: 5 * 1024 * 1024 });
    render(
      <MemoryRouter>
        <SettingsPage />
      </MemoryRouter>,
    );

    await waitFor(() => {
      expect(screen.getByText('5.0 MB')).toBeInTheDocument();
    });
  });

  it('handles vault stats error silently', async () => {
    vi.mocked(invoke).mockRejectedValue(new Error('vault locked'));
    render(
      <MemoryRouter>
        <SettingsPage />
      </MemoryRouter>,
    );

    await waitFor(() => {
      expect(screen.queryByText('5.0 MB')).not.toBeInTheDocument();
    });
    expect(screen.getByText('settings:items.data_management')).toBeInTheDocument();
  });
});
