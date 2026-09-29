import { describe, it, expect, vi, beforeEach } from 'vitest';
import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { MemoryRouter, useParams, useSearchParams, useNavigate } from 'react-router-dom';
import { ObjectWorkspacePage } from './ObjectWorkspacePage';
import { invoke } from '@tauri-apps/api/core';
import { useAuthStore } from '@/stores/authStore';
import { useTemplateStore } from '@/stores/templateStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { useUiStore } from '@/stores/uiStore';
import type { UserTemplate } from '@/types/template';

vi.mock('@/components/layout/PageShell', () => ({
  PageShell: ({
    children,
    title,
    actions,
  }: {
    children: React.ReactNode;
    title: string;
    actions: React.ReactNode;
  }) => (
    <div data-testid="page-shell" data-title={title}>
      {actions}
      {children}
    </div>
  ),
}));

vi.mock('@/components/object/ObjectDetailModal', () => ({
  ObjectDetailModal: ({
    object,
    onClose,
    onEdit,
  }: {
    object: { id: string; name: string };
    onClose: () => void;
    onEdit: () => void;
  }) => (
    <div role="dialog" aria-label="object detail" data-object-id={object.id}>
      <span>{object.name}</span>
      <button onClick={onEdit}>Edit detail</button>
      <button onClick={onClose}>Close detail</button>
    </div>
  ),
}));
vi.mock('@/components/object/HistoryViewer', () => ({
  HistoryViewer: ({
    objectId,
    getFieldSensitivity,
    getFieldName,
    onClose,
  }: {
    objectId: string;
    getFieldSensitivity: (key: string) => string;
    getFieldName: (key: string) => string;
    onClose: () => void;
  }) => (
    <div role="dialog" aria-label="object history" data-object-id={objectId}>
      <span>{getFieldName('dateOfBirth')}</span>
      <span>{getFieldSensitivity('dateOfBirth')}</span>
      <button onClick={onClose}>Close history</button>
    </div>
  ),
}));
vi.mock('@/components/object/AttachmentViewer', () => ({
  AttachmentViewer: ({ objectId, onClose }: { objectId: string; onClose: () => void }) => (
    <div role="dialog" aria-label="object attachments" data-object-id={objectId}>
      <button onClick={onClose}>Close attachments</button>
    </div>
  ),
}));

// 稳定 t（setup.ts 的 mock 每次渲染返回新函数，会让依赖 t 的 effect 无限重跑）
const { stableT } = vi.hoisted(() => ({
  stableT: (key: string, options?: { defaultValue?: string }) => options?.defaultValue ?? key,
}));
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: stableT,
    i18n: { language: 'zh', changeLanguage: vi.fn(() => Promise.resolve()) },
  }),
  I18nextProvider: ({ children }: { children: React.ReactNode }) => children,
}));

vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual('react-router-dom');
  return {
    ...actual,
    useParams: vi.fn(),
    useSearchParams: vi.fn(),
    useNavigate: vi.fn(),
  };
});

// 与真实身份模板一致：dateOfBirth 为 internal 敏感度（卡片应掩码显示占位符）
const identityTemplate: UserTemplate = {
  id: 'identity',
  accountId: 'acc1',
  name: '身份信息',
  iconId: 'identity',
  category: 'identity',
  createdAt: '2026-08-22T00:00:00Z',
  updatedAt: '2026-08-22T00:00:00Z',
  properties: [
    { id: 'fullName', name: '姓名', type: 'text', sensitivityLevel: 'public' },
    { id: 'dateOfBirth', name: '出生日期', type: 'date', sensitivityLevel: 'internal' },
  ],
};

describe('ObjectWorkspacePage card field display', () => {
  const navigate = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useNavigate).mockReturnValue(navigate);
    vi.mocked(useParams).mockReturnValue({});
    vi.mocked(useSearchParams).mockReturnValue([new URLSearchParams('section=identity'), vi.fn()]);
    useAuthStore.setState({
      currentAccount: { id: 'acc1', name: 'Acc' },
      isAuthenticated: true,
    });
    useTemplateStore.setState({
      templates: [identityTemplate],
      loadTemplates: vi.fn().mockResolvedValue(undefined),
    });
    useSettingsStore.setState((s) => ({
      settings: { ...s.settings, customPages: [] },
      removeCustomPage: vi.fn(),
    }));
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'template_list') return [identityTemplate];
      if (cmd === 'template_hash_map') return { identity: 'abc123' };
      if (cmd === 'object_list') {
        // 真实 object_list 截断结果：含 __fields，dateOfBirth 值完整保留
        return [
          {
            id: 'obj1',
            name: '张三',
            typeId: 'identity',
            sensitivityLevel: 'internal',
            createdAt: '2026-08-22T00:00:00Z',
            updatedAt: '2026-08-22T00:00:00Z',
            templateId: 'identity',
            templateType: 'user',
            properties: {
              fullName: '张三',
              dateOfBirth: '2024-12-31',
              __fields: {
                fullName: { name: '姓名', type: 'text' },
                dateOfBirth: { name: '出生日期', type: 'date' },
              },
              __templateName: '身份信息',
            },
            propertyLabels: { fullName: 'public', dateOfBirth: 'internal' },
            tags: [],
          },
        ];
      }
      if (cmd === 'snapshot_count_batch') return {};
      if (cmd === 'attachment_count_batch') return {};
      if (cmd === 'biometric_check_availability') return { available: false, configured: false };
      if (cmd === 'vault_list_accounts') return [];
      return undefined;
    });
  });

  it('shows the date field chip on the card; internal sensitivity value is masked', async () => {
    render(
      <MemoryRouter>
        <ObjectWorkspacePage />
      </MemoryRouter>,
    );

    // 等对象列表加载完成、卡片出现（对象名 + 姓名 chip 均为「张三」）
    await waitFor(() => {
      expect(screen.getAllByText('张三').length).toBeGreaterThan(0);
    });

    // 日期字段 chip 必须渲染（label 出现）——值按 internal 敏感度掩码
    expect(screen.getByText('出生日期')).toBeInTheDocument();
    // internal 字段值掩码为占位圆点（P036 设计），而不是空白或消失
    expect(screen.getAllByText('••••••••').length).toBeGreaterThan(0);
    // public 字段（姓名）原样显示
    expect(screen.getAllByText('张三').length).toBeGreaterThan(1);
  });

  it('routes card and action clicks to the matching object without opening unrelated panels', async () => {
    render(
      <MemoryRouter>
        <ObjectWorkspacePage />
      </MemoryRouter>,
    );
    const card = await screen.findByTestId('workspace-object-card');

    fireEvent.click(within(card).getAllByTitle('History')[0]);
    const history = screen.getByRole('dialog', { name: 'object history' });
    expect(history).toHaveAttribute('data-object-id', 'obj1');
    expect(within(history).getByText('internal')).toBeInTheDocument();
    expect(within(history).getByText('出生日期')).toBeInTheDocument();
    expect(screen.queryByRole('dialog', { name: 'object detail' })).toBeNull();
    fireEvent.click(within(history).getByRole('button', { name: 'Close history' }));

    fireEvent.click(within(card).getAllByTitle('Attachments')[0]);
    const attachments = screen.getByRole('dialog', { name: 'object attachments' });
    expect(attachments).toHaveAttribute('data-object-id', 'obj1');
    fireEvent.click(within(attachments).getByRole('button', { name: 'Close attachments' }));

    fireEvent.click(within(card).getAllByTitle('Edit')[0]);
    expect(navigate).toHaveBeenCalledWith('/editor/obj1');
    expect(screen.queryByRole('dialog', { name: 'object detail' })).toBeNull();

    fireEvent.click(within(card).getAllByTitle('Move to trash')[0]);
    const confirm = screen.getByRole('dialog');
    expect(within(confirm).getByRole('button', { name: 'delete' })).toBeInTheDocument();
    fireEvent.click(within(confirm).getByRole('button', { name: 'cancel' }));
    expect(screen.queryByRole('dialog')).toBeNull();

    fireEvent.click(within(card).getAllByText('张三')[0]);
    const detail = screen.getByRole('dialog', { name: 'object detail' });
    expect(detail).toHaveAttribute('data-object-id', 'obj1');
    fireEvent.click(within(detail).getByRole('button', { name: 'Edit detail' }));
    expect(navigate).toHaveBeenLastCalledWith('/editor/obj1');
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('对象删除失败时保留确认框并允许重试', async () => {
    let rejectDelete!: (error: Error) => void;
    const pendingDelete = new Promise<void>((_resolve, reject) => {
      rejectDelete = reject;
    });
    const defaultInvoke = vi.mocked(invoke).getMockImplementation()!;
    let attempts = 0;
    vi.mocked(invoke).mockImplementation((command, args) =>
      command === 'object_delete'
        ? attempts++ === 0
          ? pendingDelete
          : Promise.resolve()
        : defaultInvoke(command, args),
    );
    render(
      <MemoryRouter>
        <ObjectWorkspacePage />
      </MemoryRouter>,
    );

    const card = await screen.findByTestId('workspace-object-card');
    fireEvent.click(within(card).getAllByTitle('Move to trash')[0]);
    fireEvent.click(within(screen.getByRole('dialog')).getByRole('button', { name: 'delete' }));
    expect(attempts).toBe(1);
    expect(screen.getByRole('dialog')).toBeInTheDocument();
    expect(
      within(screen.getByRole('dialog')).getByRole('button', { name: 'delete' }),
    ).toBeDisabled();

    await act(async () => {
      rejectDelete(new Error('delete denied'));
      await pendingDelete.catch(() => undefined);
    });
    await waitFor(() =>
      expect(
        useUiStore
          .getState()
          .toasts.some(
            (toast) => toast.type === 'error' && toast.message.includes('delete denied'),
          ),
      ).toBe(true),
    );
    expect(screen.getByRole('dialog')).toBeInTheDocument();
    fireEvent.click(within(screen.getByRole('dialog')).getByRole('button', { name: 'delete' }));
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
    expect(attempts).toBe(2);
  });

  it('keeps a custom page open and reports an error when deleting it fails', async () => {
    let rejectDelete!: (error: Error) => void;
    const pendingDelete = new Promise<void>((_resolve, reject) => {
      rejectDelete = reject;
    });
    const removeCustomPage = vi
      .fn()
      .mockResolvedValue(undefined)
      .mockReturnValueOnce(pendingDelete);
    useSettingsStore.setState((s) => ({
      settings: {
        ...s.settings,
        customPages: [
          {
            id: 'page-1',
            name: '测试页面',
            iconId: 'document',
            createdAt: '2026-01-01',
            sortOrder: 0,
          },
        ],
      },
      removeCustomPage,
    }));
    vi.mocked(useParams).mockReturnValue({ pageId: 'page-1' });
    vi.mocked(useSearchParams).mockReturnValue([new URLSearchParams(), vi.fn()]);
    render(
      <MemoryRouter>
        <ObjectWorkspacePage />
      </MemoryRouter>,
    );
    await waitFor(() =>
      expect(screen.getByTestId('page-shell')).toHaveAttribute('data-title', '测试页面'),
    );
    fireEvent.click(screen.getByRole('button', { name: 'delete' }));
    const dialog = screen.getByRole('dialog');
    fireEvent.click(within(dialog).getByRole('button', { name: 'delete' }));
    await waitFor(() => expect(removeCustomPage).toHaveBeenCalledWith('acc1', 'page-1'));
    expect(within(dialog).getByRole('button', { name: 'delete' })).toBeDisabled();
    fireEvent.keyDown(document, { key: 'Escape' });
    expect(screen.getByRole('dialog')).toBeInTheDocument();
    expect(removeCustomPage).toHaveBeenCalledTimes(1);

    await act(async () => {
      rejectDelete(new Error('db locked'));
      await pendingDelete.catch(() => undefined);
    });
    expect(navigate).not.toHaveBeenCalledWith('/');
    expect(screen.getByRole('dialog')).toBeInTheDocument();
    await waitFor(() =>
      expect(
        useUiStore
          .getState()
          .toasts.some((toast) => toast.type === 'error' && toast.message.includes('db locked')),
      ).toBe(true),
    );

    fireEvent.click(within(screen.getByRole('dialog')).getByRole('button', { name: 'delete' }));
    await waitFor(() => expect(navigate).toHaveBeenCalledWith('/'));
    expect(removeCustomPage).toHaveBeenCalledTimes(2);
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it.each(['succeeds', 'fails'] as const)(
    'switching accounts isolates an in-flight page deletion that %s from the new page',
    async (outcome) => {
      let resolveA!: () => void;
      let rejectA!: (error: Error) => void;
      let resolveB!: () => void;
      const pendingA = new Promise<void>((resolve, reject) => {
        resolveA = resolve;
        rejectA = reject;
      });
      const pendingB = new Promise<void>((resolve) => {
        resolveB = resolve;
      });
      const removeCustomPage = vi.fn((accountId: string) =>
        accountId === 'acc1' ? pendingA : pendingB,
      );
      useSettingsStore.setState((s) => ({
        settings: {
          ...s.settings,
          customPages: [
            {
              id: 'page-a',
              name: 'Page A',
              iconId: 'document',
              createdAt: '2026-01-01',
              sortOrder: 0,
            },
            {
              id: 'page-b',
              name: 'Page B',
              iconId: 'document',
              createdAt: '2026-01-01',
              sortOrder: 1,
            },
          ],
        },
        removeCustomPage,
      }));
      vi.mocked(useParams).mockReturnValue({ pageId: 'page-a' });
      vi.mocked(useSearchParams).mockReturnValue([new URLSearchParams(), vi.fn()]);
      const view = render(
        <MemoryRouter>
          <ObjectWorkspacePage />
        </MemoryRouter>,
      );
      await waitFor(() =>
        expect(screen.getByTestId('page-shell')).toHaveAttribute('data-title', 'Page A'),
      );
      fireEvent.click(screen.getByRole('button', { name: 'delete' }));
      fireEvent.click(within(screen.getByRole('dialog')).getByRole('button', { name: 'delete' }));
      await waitFor(() => expect(removeCustomPage).toHaveBeenCalledWith('acc1', 'page-a'));

      vi.mocked(useParams).mockReturnValue({ pageId: 'page-b' });
      act(() => useAuthStore.setState({ currentAccount: { id: 'acc2', name: 'B' } }));
      // 账户切换会清空 A 的设置；模拟 B 的自定义页完成加载。
      act(() =>
        useSettingsStore.setState((s) => ({
          settings: {
            ...s.settings,
            customPages: [
              {
                id: 'page-b',
                name: 'Page B',
                iconId: 'document',
                createdAt: '2026-01-01',
                sortOrder: 1,
              },
            ],
          },
        })),
      );
      view.rerender(
        <MemoryRouter>
          <ObjectWorkspacePage />
        </MemoryRouter>,
      );
      await waitFor(() =>
        expect(screen.getByTestId('page-shell')).toHaveAttribute('data-title', 'Page B'),
      );
      expect(screen.queryByRole('dialog')).not.toBeInTheDocument();

      fireEvent.click(screen.getByRole('button', { name: 'delete' }));
      fireEvent.click(within(screen.getByRole('dialog')).getByRole('button', { name: 'delete' }));
      await waitFor(() => expect(removeCustomPage).toHaveBeenCalledWith('acc2', 'page-b'));
      await act(async () => {
        if (outcome === 'succeeds') resolveA();
        else rejectA(new Error('A page failed'));
        await pendingA.catch(() => undefined);
      });
      expect(navigate).not.toHaveBeenCalledWith('/');
      expect(screen.getByRole('dialog')).toBeInTheDocument();
      expect(
        useUiStore.getState().toasts.some((toast) => toast.message.includes('A page failed')),
      ).toBe(false);

      await act(async () => {
        resolveB();
        await pendingB;
      });
      expect(navigate).toHaveBeenCalledWith('/');
    },
  );
});
