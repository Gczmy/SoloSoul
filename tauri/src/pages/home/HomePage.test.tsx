import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor, within } from '@testing-library/react';
import { MemoryRouter, Navigate, useNavigate } from 'react-router-dom';
import { invoke } from '@tauri-apps/api/core';
import { HomePage } from './HomePage';

vi.mock('@/components/layout/AppShell', () => ({
  AppShell: ({ children, title }: { children: React.ReactNode; title: string }) => (
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

// 隔离 HomePage 的相册历史标记/返回守卫行为（PhotoAlbumOverlay 自身另有独立测试）
vi.mock('@/components/attachment/PhotoAlbumOverlay', () => ({
  PhotoAlbumOverlay: ({ items }: { items: unknown[] }) => (
    <div data-testid="home-album-overlay" data-count={items.length} />
  ),
}));

describe('HomePage', () => {
  const navigate = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
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
    // 3 个活跃附件（2 张图片 + 1 个 PDF）：附件角标 3、照片角标 2
    const listAllResult = {
      pages: [
        {
          pageName: '页面一',
          objects: [
            {
              objectName: '对象一',
              attachments: [
                { id: 'a1', fileName: 'a.png', mimeType: 'image/png' },
                { id: 'a2', fileName: 'b.pdf', mimeType: 'application/pdf' },
              ],
            },
            {
              objectName: '对象二',
              attachments: [{ id: 'a3', fileName: 'c.jpg', mimeType: 'image/jpeg' }],
            },
          ],
        },
      ],
      trashPages: [],
    };
    mockUseAuthStore.mockImplementation(
      (selector: (s: { currentAccount: { id: string; name: string } | null }) => unknown) =>
        selector({ currentAccount: { id: 'acc-1', name: 'Gczmy' } }),
    );
    vi.mocked(invoke).mockImplementation(async (cmd: string) =>
      cmd === 'attachment_list_all' ? listAllResult : undefined,
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
    const listAllResult = {
      pages: [{ pageName: 'P', objects: [{ attachments: [{ id: 'a1' }] }] }],
      trashPages: [],
    };
    mockUseAuthStore.mockImplementation(
      (selector: (s: { currentAccount: { id: string; name: string } | null }) => unknown) =>
        selector({ currentAccount: { id: 'acc-1', name: 'Gczmy' } }),
    );
    let callCount = 0;
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'attachment_list_all') {
        callCount += 1;
        return listAllResult;
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

  it('首页照片集打开时压入历史标记，Android 硬件返回关闭相册而非跳回上一路由', async () => {
    const listAllResult = {
      pages: [
        {
          pageName: 'P',
          objects: [{ attachments: [{ id: 'a1', fileName: 'a.png', mimeType: 'image/png' }] }],
        },
      ],
      trashPages: [],
    };
    mockUseAuthStore.mockImplementation(
      (selector: (s: { currentAccount: { id: string; name: string } | null }) => unknown) =>
        selector({ currentAccount: { id: 'acc-1', name: 'Gczmy' } }),
    );
    vi.mocked(invoke).mockImplementation(async (cmd: string) =>
      cmd === 'attachment_list_all' ? listAllResult : undefined,
    );

    render(
      <MemoryRouter>
        <HomePage />
      </MemoryRouter>,
    );

    // 点击照片集快捷入口 → 相册打开
    const albumCard = screen.getByText('Photo Album').closest('[role="button"]') as HTMLElement;
    fireEvent.click(albumCard);
    await waitFor(() => {
      expect(screen.getByTestId('home-album-overlay')).toBeInTheDocument();
    });

    // 打开时压入了同名历史标记（URL 不变），供硬件返回先弹出
    const state = window.history.state as { solosoulHomeAlbum?: boolean };
    expect(state.solosoulHomeAlbum).toBe(true);

    // 模拟 Android 硬件返回：浏览器弹出标记条目并触发 popstate
    fireEvent.popState(window);
    await waitFor(() => {
      expect(screen.queryByTestId('home-album-overlay')).not.toBeInTheDocument();
    });
    // 相册关闭后仍在首页（未发生路由跳转）
    expect(screen.getByText('Photo Album')).toBeInTheDocument();
    expect(navigate).not.toHaveBeenCalled();
  });
});
