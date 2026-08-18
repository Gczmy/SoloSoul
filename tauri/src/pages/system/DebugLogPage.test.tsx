import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { DebugLogPage } from './DebugLogPage';
import { invoke } from '@tauri-apps/api/core';
import { prefetchRegistry } from '@/lib/prefetch/registry';

vi.mock('@/components/layout/PageShell', () => ({
  PageShell: ({ children, title }: { children: React.ReactNode; title: string }) => (
    <div data-testid="app-shell" data-title={title}>
      {children}
    </div>
  ),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('@tauri-apps/plugin-dialog', () => ({
  save: vi.fn(),
}));

vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual('react-router-dom');
  return {
    ...actual,
    useNavigate: vi.fn(),
  };
});

describe('DebugLogPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // 模块级 prefetch store 单例跨测试缓存，需重置隔离。
    prefetchRegistry.logs.reset();
  });

  it('renders loading placeholder initially', () => {
    vi.mocked(invoke).mockImplementation(() => new Promise(() => {}));
    render(
      <MemoryRouter>
        <DebugLogPage />
      </MemoryRouter>,
    );
    expect(screen.getByTestId('loading-placeholder')).toBeInTheDocument();
    expect(screen.queryByText('settings:loading_logs_debug')).not.toBeInTheDocument();
  });

  it('renders log entries after loading', async () => {
    const logs = [
      {
        id: 1,
        timestamp: '2024-06-01T10:30:00.000Z',
        actionType: 'create',
        entityType: 'profile',
        entityId: 'p1',
        entityName: 'Test Profile',
        performedBy: 'user',
        details: null,
      },
      {
        id: 2,
        timestamp: '2024-06-01T11:00:00.000Z',
        actionType: 'delete',
        entityType: 'profile',
        entityId: 'p2',
        entityName: 'Old Profile',
        performedBy: 'user',
        details: 'cleanup',
      },
    ];
    vi.mocked(invoke).mockResolvedValue(logs);

    render(
      <MemoryRouter>
        <DebugLogPage />
      </MemoryRouter>,
    );

    await waitFor(() => {
      expect(screen.getByText('CREATE')).toBeInTheDocument();
    });

    expect(screen.getByText('DELETE')).toBeInTheDocument();
    expect(screen.getByText('2 settings:entries_count')).toBeInTheDocument();
  });

  it('shows empty state when no logs', async () => {
    vi.mocked(invoke).mockResolvedValue([]);

    render(
      <MemoryRouter>
        <DebugLogPage />
      </MemoryRouter>,
    );

    await waitFor(() => {
      expect(screen.getByText('settings:no_log_entries_debug')).toBeInTheDocument();
    });
  });

  it('refreshes logs when refresh button is clicked', async () => {
    vi.mocked(invoke).mockResolvedValue([]);

    render(
      <MemoryRouter>
        <DebugLogPage />
      </MemoryRouter>,
    );

    await waitFor(() => {
      expect(screen.getByText('settings:no_log_entries_debug')).toBeInTheDocument();
    });

    const newLogs = [
      {
        id: 3,
        timestamp: '2024-06-02T10:00:00.000Z',
        actionType: 'update',
        entityType: 'object',
        entityId: 'o1',
        entityName: 'Obj',
        performedBy: 'user',
        details: null,
      },
    ];
    vi.mocked(invoke).mockResolvedValue(newLogs);

    const refreshBtn = screen.getByText('settings:refresh').closest('button') as HTMLButtonElement;
    fireEvent.click(refreshBtn);

    await waitFor(() => {
      expect(screen.getByText('UPDATE')).toBeInTheDocument();
    });
  });

  it('handles export flow', async () => {
    vi.mocked(invoke).mockResolvedValue([]);
    const { save } = await import('@tauri-apps/plugin-dialog');
    vi.mocked(save).mockResolvedValue('/path/to/export.json');

    render(
      <MemoryRouter>
        <DebugLogPage />
      </MemoryRouter>,
    );

    await waitFor(() => {
      expect(screen.getByText('settings:export')).toBeInTheDocument();
    });

    const exportBtn = screen.getByText('settings:export').closest('button') as HTMLButtonElement;
    fireEvent.click(exportBtn);

    await waitFor(() => {
      expect(save).toHaveBeenCalledWith(
        expect.objectContaining({
          defaultPath: 'debug_log_export.json',
        }),
      );
    });
    expect(invoke).toHaveBeenCalledWith('log_export', { exportPath: '/path/to/export.json' });
  });

  it('handles export cancellation', async () => {
    vi.mocked(invoke).mockResolvedValue([]);
    const { save } = await import('@tauri-apps/plugin-dialog');
    vi.mocked(save).mockResolvedValue(null);

    render(
      <MemoryRouter>
        <DebugLogPage />
      </MemoryRouter>,
    );

    await waitFor(() => {
      expect(screen.getByText('settings:export')).toBeInTheDocument();
    });

    const exportBtn = screen.getByText('settings:export').closest('button') as HTMLButtonElement;
    fireEvent.click(exportBtn);

    await waitFor(() => {
      expect(save).toHaveBeenCalled();
    });
    // log_export should not be called when save returns null
    const logExportCalls = vi.mocked(invoke).mock.calls.filter(([cmd]) => cmd === 'log_export');
    expect(logExportCalls).toHaveLength(0);
  });
});
