import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { act, render, screen, waitFor, fireEvent } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { SearchPopover } from './SearchPopover';

const { stableT, mockInvoke, mockAccount } = vi.hoisted(() => ({
  stableT: (key: string, options?: { defaultValue?: string }) => options?.defaultValue ?? key,
  mockInvoke: vi.fn(),
  mockAccount: { id: 'acc-1' },
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: stableT,
    i18n: { language: 'en', changeLanguage: vi.fn(() => Promise.resolve()) },
  }),
  I18nextProvider: ({ children }: { children: React.ReactNode }) => children,
}));

vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual('react-router-dom');
  return {
    ...actual,
    useNavigate: () => vi.fn(),
  };
});

vi.mock('@/lib/ipcClient', () => ({
  invokeCommand: (...args: unknown[]) => mockInvoke(...args),
}));

vi.mock('@/stores/authStore', () => ({
  useAuthStore: (selector: (s: unknown) => unknown) =>
    selector({ currentAccount: { id: mockAccount.id } }),
}));

vi.mock('@/stores/settingsStore', () => ({
  useSettingsStore: (selector: (s: unknown) => unknown) =>
    selector({ settings: { customPages: [] } }),
}));

vi.mock('@/hooks/useToastError', () => ({
  useToastError: () => ({ onError: vi.fn(), onSuccess: vi.fn() }),
}));

vi.mock('@/components/object/ObjectDetailModal', () => ({
  ObjectDetailModal: () => <div data-testid="object-detail-modal" />,
}));

vi.mock('react-dom', async () => {
  const actual = await vi.importActual('react-dom');
  return {
    ...actual,
    createPortal: (node: React.ReactNode) => node,
  };
});

import type { SearchItem } from '@/lib/searchShared';
import { searchCache } from '@/lib/searchCache';
import { setRequestSession } from '@/lib/sessionRequests';

const objectResult: SearchItem = {
  itemType: 'object',
  objectId: 'obj-1',
  name: '护照',
  typeId: 'identity',
  matchType: 'name',
  relevance: 1,
};

const pageResult: SearchItem = {
  itemType: 'page',
  objectId: 'identity',
  name: 'identity',
  typeId: 'identity',
  matchType: 'name',
  objectCount: 3,
  relevance: 1,
};

describe('SearchPopover (P027 渲染回归)', () => {
  beforeEach(() => {
    mockInvoke.mockReset();
    mockAccount.id = 'acc-1';
    searchCache.clear();
    localStorage.removeItem('solosoul_recent_searches:acc-1');
  });
  afterEach(() => {
    act(() => setRequestSession(null));
    vi.restoreAllMocks();
    localStorage.removeItem('solosoul_recent_searches:acc-1');
  });

  it('malformed recent-search storage cannot break submitting a new query', () => {
    localStorage.setItem('solosoul_recent_searches:acc-1', '{"unexpected":"object"}');
    render(
      <MemoryRouter>
        <SearchPopover onClose={vi.fn()} />
      </MemoryRouter>,
    );

    const input = screen.getByPlaceholderText('common:search_placeholder');
    fireEvent.change(input, { target: { value: 'safe query' } });
    fireEvent.keyDown(input, { key: 'Enter' });

    expect(localStorage.getItem('solosoul_recent_searches:acc-1')).toBe('["safe query"]');
  });

  it('rejects non-string recent entries before rendering them', () => {
    localStorage.setItem('solosoul_recent_searches:acc-1', '[{"secret":"value"},null,"valid"]');
    render(
      <MemoryRouter>
        <SearchPopover onClose={vi.fn()} />
      </MemoryRouter>,
    );

    expect(screen.getByText('valid')).toBeInTheDocument();
    expect(screen.queryByText('value')).not.toBeInTheDocument();
  });

  it('opens a search result even when recent-search storage rejects writes', async () => {
    mockInvoke.mockResolvedValue({ items: [objectResult], total: 1, hasMore: false });
    render(
      <MemoryRouter>
        <SearchPopover onClose={vi.fn()} />
      </MemoryRouter>,
    );
    fireEvent.change(screen.getByPlaceholderText('common:search_placeholder'), {
      target: { value: '护照' },
    });
    const result = await screen.findByText('护照');
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new Error('storage unavailable');
    });

    fireEvent.click(result);

    expect(screen.getByTestId('object-detail-modal')).toBeInTheDocument();
  });

  it('验证弹窗不触发结果打开或搜索外部关闭，取消后保持掩码', async () => {
    mockInvoke.mockImplementation(async (cmd: string) =>
      cmd === 'search_unified'
        ? {
            items: [
              {
                ...objectResult,
                matchType: 'fieldValue',
                matchedField: 'key',
                matchedValue: 'CRITICAL_MATCH',
                sensitivityLevels: ['public', 'critical'],
              },
            ],
            total: 1,
            hasMore: false,
          }
        : undefined,
    );
    const onClose = vi.fn();
    render(
      <MemoryRouter>
        <SearchPopover onClose={onClose} />
      </MemoryRouter>,
    );
    fireEvent.change(screen.getByPlaceholderText('common:search_placeholder'), {
      target: { value: 'secret-query' },
    });
    const reveal = await screen.findByText('••••••••');
    expect(reveal.closest('button')?.parentElement?.closest('button')).toBeNull();
    fireEvent.click(reveal);
    const dialog = await screen.findByRole('dialog');
    fireEvent.mouseDown(dialog);
    expect(onClose).not.toHaveBeenCalled();
    expect(screen.queryByTestId('object-detail-modal')).toBeNull();
    expect(document.body.innerHTML).not.toContain('CRITICAL_MATCH');
    fireEvent.keyDown(document, { key: 'Escape' });
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    expect(onClose).not.toHaveBeenCalled();
    expect(screen.getByText('••••••••')).toBeInTheDocument();
    fireEvent.keyDown(document, { key: 'Escape' });
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('输入关键词触发搜索并渲染结果行（SearchResultRow 提取后完整渲染）', async () => {
    mockInvoke.mockResolvedValue({ items: [objectResult, pageResult], total: 2, hasMore: false });

    render(
      <MemoryRouter>
        <SearchPopover onClose={vi.fn()} />
      </MemoryRouter>,
    );

    const input = screen.getByPlaceholderText('common:search_placeholder');
    fireEvent.change(input, { target: { value: '护照' } });

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith(
        'search_unified',
        expect.any(Object),
        expect.objectContaining({ requestIsCurrent: expect.any(Function) }),
      );
    });
    // 结果行渲染（对象名 + 页面标签 + 元信息）
    await waitFor(() => {
      expect(screen.getByText('护照')).toBeInTheDocument();
    });
    expect(screen.getByText('settings:search_type_object')).toBeInTheDocument();
    expect(screen.getByText('settings:search_type_page')).toBeInTheDocument();
  });

  it('空结果显示 no_results', async () => {
    mockInvoke.mockResolvedValue({ items: [], total: 0, hasMore: false });

    render(
      <MemoryRouter>
        <SearchPopover onClose={vi.fn()} />
      </MemoryRouter>,
    );

    fireEvent.change(screen.getByPlaceholderText('common:search_placeholder'), {
      target: { value: '不存在的词' },
    });
    await waitFor(() => {
      expect(screen.getByText('common:no_results')).toBeInTheDocument();
    });
  });

  it('底部设置入口渲染（footer 提取后保持）', () => {
    render(
      <MemoryRouter>
        <SearchPopover onClose={vi.fn()} />
      </MemoryRouter>,
    );
    expect(screen.getByText('navigation:settings')).toBeInTheDocument();
  });

  it('外部点击关闭，搜索触发器和输入框点击交给自身处理', () => {
    const onClose = vi.fn();
    render(
      <MemoryRouter>
        <div data-search-button>
          <button>搜索入口</button>
        </div>
        <button>其他工具</button>
        <SearchPopover onClose={onClose} />
      </MemoryRouter>,
    );
    fireEvent.mouseDown(screen.getByPlaceholderText('common:search_placeholder'));
    fireEvent.mouseDown(screen.getByText('搜索入口'));
    expect(onClose).not.toHaveBeenCalled();
    fireEvent.mouseDown(screen.getByText('其他工具'));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('结果打开对象详情后，详情点击和 Escape 不关闭底层搜索', async () => {
    mockInvoke.mockResolvedValue({ items: [objectResult], total: 1, hasMore: false });
    const onClose = vi.fn();
    render(
      <MemoryRouter>
        <SearchPopover onClose={onClose} />
      </MemoryRouter>,
    );
    fireEvent.change(screen.getByPlaceholderText('common:search_placeholder'), {
      target: { value: '护照' },
    });
    fireEvent.click(await screen.findByText('护照'));
    fireEvent.mouseDown(screen.getByTestId('object-detail-modal'));
    fireEvent.keyDown(document, { key: 'Escape' });
    expect(onClose).not.toHaveBeenCalled();
  });

  it('切换账户后关闭上一账户从搜索打开的对象详情', async () => {
    mockInvoke.mockResolvedValue({ items: [objectResult], total: 1, hasMore: false });
    const onClose = vi.fn();
    const view = (
      <MemoryRouter>
        <SearchPopover onClose={onClose} />
      </MemoryRouter>
    );
    const { rerender } = render(view);
    fireEvent.change(screen.getByPlaceholderText('common:search_placeholder'), {
      target: { value: '护照' },
    });
    fireEvent.click(await screen.findByText('护照'));
    expect(screen.getByTestId('object-detail-modal')).toBeInTheDocument();

    mockAccount.id = 'acc-2';
    rerender(
      <MemoryRouter>
        <SearchPopover onClose={onClose} />
      </MemoryRouter>,
    );
    expect(screen.queryByTestId('object-detail-modal')).toBeNull();
  });

  it('锁定会话后关闭从搜索打开的对象详情', async () => {
    act(() => setRequestSession('acc-1'));
    mockInvoke.mockResolvedValue({ items: [objectResult], total: 1, hasMore: false });
    render(
      <MemoryRouter>
        <SearchPopover onClose={vi.fn()} />
      </MemoryRouter>,
    );
    fireEvent.change(screen.getByPlaceholderText('common:search_placeholder'), {
      target: { value: '护照' },
    });
    fireEvent.click(await screen.findByText('护照'));
    expect(screen.getByTestId('object-detail-modal')).toBeInTheDocument();

    act(() => setRequestSession(null));
    expect(screen.queryByTestId('object-detail-modal')).toBeNull();
  });
});
