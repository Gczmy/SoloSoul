import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { MemoryRouter, useNavigate } from 'react-router-dom';
import { DataManagementPage } from './DataManagementPage';
import { invoke } from '@tauri-apps/api/core';
import { prefetchRegistry } from '@/lib/prefetch/registry';

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

describe('DataManagementPage', () => {
  const navigate = vi.fn();

  beforeEach(() => {
    // 模块级 prefetch store 单例跨测试共享——重置避免 TTL 缓存污染
    prefetchRegistry.vaultStats.reset();
    vi.clearAllMocks();
    vi.mocked(useNavigate).mockReturnValue(navigate);
  });

  const mockStats = {
    profileCount: 1,
    totalSizeBytes: 1024 * 1024,
    profilesSize: 1024 * 512,
    objectsSize: 1024 * 256,
    trashSize: 0,
    snapshotsSize: 0,
    attachmentsSize: 1024 * 256,
    aiConversationsSize: 0,
  };

  it('calls get_vault_stats (not vault_get_stats)', async () => {
    vi.mocked(invoke).mockResolvedValue(mockStats);
    render(
      <MemoryRouter>
        <DataManagementPage />
      </MemoryRouter>,
    );

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('get_vault_stats');
    });
  });

  it('renders vault size and shows breakdown card when the size button is clicked', async () => {
    vi.mocked(invoke).mockResolvedValue(mockStats);
    render(
      <MemoryRouter>
        <DataManagementPage />
      </MemoryRouter>,
    );

    // Vault size display once stats load (previously stuck at '—' when the command failed)
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('get_vault_stats');
    });
    expect(screen.getByText('1.0 MB')).toBeInTheDocument();

    // Click the breakdown button -> card must appear (regression: card didn't render)
    const breakdownButton = screen.getByTitle('settings:view_breakdown');
    fireEvent.click(breakdownButton);
    expect(screen.getByText('settings:storage_breakdown')).toBeInTheDocument();
    expect(screen.getByText('common:total')).toBeInTheDocument();
  });
});
