import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { SearchPopover } from './SearchPopover';

const { stableT, mockInvoke } = vi.hoisted(() => ({
  stableT: (key: string, options?: { defaultValue?: string }) => options?.defaultValue ?? key,
  mockInvoke: vi.fn(),
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
    selector({ currentAccount: { id: 'acc-1' } }),
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
      expect(mockInvoke).toHaveBeenCalledWith('search_unified', expect.any(Object));
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
});
