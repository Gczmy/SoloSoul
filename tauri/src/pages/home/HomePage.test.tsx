import { describe, it, expect, vi, beforeEach } from 'vitest';
import { act, render, screen, fireEvent, waitFor, within } from '@testing-library/react';
import { MemoryRouter, Navigate, useNavigate } from 'react-router-dom';
import { invoke } from '@tauri-apps/api/core';
import { HomePage } from './HomePage';
import { useSettingsStore, type CustomPage } from '@/stores/settingsStore';

vi.mock('@/components/layout/PageShell', () => ({
  PageShell: ({ children, title }: { children: React.ReactNode; title: string }) => (
    <div data-testid="app-shell" data-title={title}>
      {children}
    </div>
  ),
}));

vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual('react-router-dom');
  return {
    ...actual,
    useNavigate: vi.fn(),
  };
});

const { mockUseAuthStore } = vi.hoisted(() => ({ mockUseAuthStore: vi.fn() }));
vi.mock('@/stores/authStore', () => ({
  useAuthStore: (selector: unknown) => mockUseAuthStore(selector),
}));

// 隔离 HomePage 的相册打开/关闭状态（PhotoAlbumOverlay 的分层返回守卫另有独立测试）
vi.mock('@/components/attachment/PhotoAlbumOverlay', () => ({
  PhotoAlbumOverlay: ({ items, onClose }: { items: unknown[]; onClose: () => void }) => (
    <div data-testid="home-album-overlay" data-count={items.length}>
      <button data-testid="home-album-close" onClick={onClose}>
        close
      </button>
    </div>
  ),
}));

vi.mock('@/components/layout/CustomPageEditPopover', () => ({
  CustomPageEditPopover: ({ page, onClose }: { page: CustomPage; onClose: () => void }) => (
    <div role="dialog" aria-label="edit custom page">
      <span>{page.name}</span>
      <button onClick={onClose}>Close edit</button>
    </div>
  ),
}));

describe('HomePage', () => {
  const navigate = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    useSettingsStore.setState((state) => ({
      settings: { ...state.settings, customPages: [] },
    }));
    vi.mocked(useNavigate).mockReturnValue(navigate);
    // 默认无账户：与既有欢迎卡片断言（common:welcome_back）保持一致
    mockUseAuthStore.mockImplementation((selector: (s: { currentAccount: null }) => unknown) =>
      selector({ currentAccount: null }),
    );
  });

  it('renders welcome card and section cards', () => {
    render(
      <MemoryRouter>
        <HomePage />
      </MemoryRouter>,
    );

    expect(screen.getByTestId('app-shell')).toBeInTheDocument();
    expect(screen.getByText('common:welcome_back')).toBeInTheDocument();
    expect(screen.getByText('common:vault_description')).toBeInTheDocument();
    expect(screen.getByText('navigation:identity')).toBeInTheDocument();
    expect(screen.getByText('navigation:travel')).toBeInTheDocument();
    expect(screen.getByText('navigation:financial')).toBeInTheDocument();
    expect(screen.getByText('navigation:professional')).toBeInTheDocument();
    expect(screen.getByText('navigation:help')).toBeInTheDocument();
  });

  it('navigates to workspace section on card click', () => {
    render(
      <MemoryRouter>
        <HomePage />
      </MemoryRouter>,
    );

    const identityCard = screen
      .getByText('navigation:identity')
      .closest('[role="button"]') as HTMLElement;
    fireEvent.click(identityCard);
    expect(navigate).toHaveBeenCalledWith('/workspace?section=identity');

    const travelCard = screen
      .getByText('navigation:travel')
      .closest('[role="button"]') as HTMLElement;
    fireEvent.click(travelCard);
    expect(navigate).toHaveBeenCalledWith('/workspace?section=travel');
  });

  it('自定义页面短按进入工作区，长按只打开该页编辑且关闭后仍可进入', () => {
    const customPage: CustomPage = {
      id: 'custom-1',
      name: 'Personal Notes',
      iconId: 'star',
      createdAt: '2026-01-01',
      sortOrder: 0,
    };
    useSettingsStore.setState((state) => ({
      settings: {
        ...state.settings,
        customPages: [
          customPage,
          { ...customPage, id: 'deleted', name: 'Deleted', deletedAt: '2026-01-02' },
        ],
      },
    }));
    render(
      <MemoryRouter>
        <HomePage />
      </MemoryRouter>,
    );

    expect(screen.queryByText('Deleted')).not.toBeInTheDocument();
    const card = screen.getByText('Personal Notes').closest('[role="button"]') as HTMLElement;
    fireEvent.click(card);
    expect(navigate).toHaveBeenCalledWith('/workspace/custom/custom-1');
    navigate.mockClear();

    vi.useFakeTimers();
    try {
      fireEvent.mouseDown(card);
      act(() => vi.advanceTimersByTime(500));
      fireEvent.mouseUp(card);
      fireEvent.click(card);
      expect(screen.getByRole('dialog', { name: 'edit custom page' })).toHaveTextContent(
        'Personal Notes',
      );
      expect(navigate).not.toHaveBeenCalled();

      fireEvent.click(screen.getByRole('button', { name: 'Close edit' }));
      expect(screen.queryByRole('dialog', { name: 'edit custom page' })).not.toBeInTheDocument();
      fireEvent.click(card);
      expect(navigate).toHaveBeenCalledWith('/workspace/custom/custom-1');
    } finally {
      vi.useRealTimers();
    }
  });

  it('navigates to help on help card click', () => {
    render(
      <MemoryRouter>
        <HomePage />
      </MemoryRouter>,
    );

    const helpCard = screen.getByText('navigation:help').closest('[role="button"]') as HTMLElement;
    fireEvent.click(helpCard);
    expect(navigate).toHaveBeenCalledWith('/help', { state: { fromHome: true } });
  });

  it('附件管理与照片集快捷卡片显示数量角标', async () => {
    // 3 个活跃附件（2 张图片 + 1 个 PDF）：附件角标 3、照片角标 2（后端轻量计数返回）
    const countStats = { attachmentCount: 3, photoCount: 2 };
    mockUseAuthStore.mockImplementation(
      (selector: (s: { currentAccount: { id: string; name: string } | null }) => unknown) =>
        selector({ currentAccount: { id: 'acc-1', name: 'Gczmy' } }),
    );
    vi.mocked(invoke).mockImplementation(async (cmd: string) =>
      cmd === 'attachment_count_stats' ? countStats : undefined,
    );

    render(
      <MemoryRouter>
        <HomePage />
      </MemoryRouter>,
    );

    const attachmentsCard = screen
      .getByText('navigation:attachments')
      .closest('[role="button"]') as HTMLElement;
    await waitFor(() => {
      expect(within(attachmentsCard).getByText('3')).toBeInTheDocument();
    });

    const albumCard = screen.getByText('Photo Album').closest('[role="button"]') as HTMLElement;
    expect(within(albumCard).getByText('2')).toBeInTheDocument();
  });

  it('从其他页面返回首页时重新加载角标计数', async () => {
    const countStats = { attachmentCount: 1, photoCount: 0 };
    mockUseAuthStore.mockImplementation(
      (selector: (s: { currentAccount: { id: string; name: string } | null }) => unknown) =>
        selector({ currentAccount: { id: 'acc-1', name: 'Gczmy' } }),
    );
    let callCount = 0;
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'attachment_count_stats') {
        callCount += 1;
        return countStats;
      }
      return undefined;
    });

    // HomePage 始终直接渲染（不经 Route 匹配，保持挂载），仅路由位置随导航变化
    function Harness({ nav }: { nav: 'home' | 'away' | 'back' }) {
      return (
        <MemoryRouter initialEntries={['/']}>
          {nav === 'away' && <Navigate to="/settings/attachments" replace />}
          {nav === 'back' && <Navigate to="/" replace />}
          <HomePage />
        </MemoryRouter>
      );
    }

    const { rerender } = render(<Harness nav="home" />);
    // 挂载时加载一次
    await waitFor(() => expect(callCount).toBe(1));

    // 离开首页（位置变化被守卫跳过，不加载）
    rerender(<Harness nav="away" />);
    // 返回首页（位置回到 '/'，重新加载）
    rerender(<Harness nav="back" />);
    await waitFor(() => expect(callCount).toBe(2));
  });

  it('首页照片集打开/关闭相册均不产生路由跳转（分层返回守卫在 PhotoAlbumOverlay 内）', async () => {
    const listAllResult = {
      pages: [
        {
          pageName: 'P',
          objects: [{ attachments: [{ id: 'a1', fileName: 'a.png', mimeType: 'image/png' }] }],
        },
      ],
      trashPages: [],
    };
    const countStats = { attachmentCount: 1, photoCount: 1 };
    mockUseAuthStore.mockImplementation(
      (selector: (s: { currentAccount: { id: string; name: string } | null }) => unknown) =>
        selector({ currentAccount: { id: 'acc-1', name: 'Gczmy' } }),
    );
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'attachment_count_stats') return countStats;
      if (cmd === 'attachment_list_all') return listAllResult;
      return undefined;
    });

    render(
      <MemoryRouter>
        <HomePage />
      </MemoryRouter>,
    );

    // 点击照片集快捷入口 → 相册打开（不产生路由跳转）
    const albumCard = screen.getByText('Photo Album').closest('[role="button"]') as HTMLElement;
    fireEvent.click(albumCard);
    await waitFor(() => {
      expect(screen.getByTestId('home-album-overlay')).toBeInTheDocument();
    });
    expect(navigate).not.toHaveBeenCalled();

    // 关闭相册（mock overlay 的 onClose）后仍在首页（未发生路由跳转）
    fireEvent.click(screen.getByTestId('home-album-close'));
    await waitFor(() => {
      expect(screen.queryByTestId('home-album-overlay')).not.toBeInTheDocument();
    });
    expect(screen.getByText('Photo Album')).toBeInTheDocument();
    expect(navigate).not.toHaveBeenCalled();
  });
});
