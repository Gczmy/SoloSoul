import { describe, it, expect, vi, beforeEach } from 'vitest';
import { act, render, screen, fireEvent, waitFor } from '@testing-library/react';
import { BrowserRouter } from 'react-router-dom';
import { ObjectDetailModal } from './ObjectDetailModal';
import type { ObjectData } from '@/stores/objectStore';

// ── 依赖 mock ────────────────────────────────────────────────────────────
// P020 二次复核：modal 不再经全局 getObject action（会置 isLoading 闪列表），
// 改为直接 invoke('object_get')；默认 mock 返回 null → fetchedObj=null → 回退传入 object。
vi.mock('@/lib/ipcClient', () => ({
  invokeCommand: vi.fn().mockResolvedValue(null),
}));

vi.mock('@/lib/platform', () => ({
  isAndroidSync: vi.fn(() => false),
  isMobilePlatformSync: vi.fn(() => false),
}));

vi.mock('@/lib/logger', () => ({
  logger: { warn: vi.fn(), error: vi.fn(), info: vi.fn(), debug: vi.fn() },
}));

vi.mock('@/stores/authStore', () => ({
  useAuthStore: (selector: (s: { currentAccount: { id: string } | null }) => unknown) =>
    selector({ currentAccount: { id: 'acc-1' } }),
}));

vi.mock('@/stores/templateStore', () => ({
  useTemplateStore: (
    selector: (s: { templates: unknown[]; loadTemplates: () => Promise<void> }) => unknown,
  ) => selector({ templates: [], loadTemplates: vi.fn().mockResolvedValue(undefined) }),
}));

vi.mock('@/stores/settingsStore', () => ({
  useSettingsStore: (selector: (s: { settings: { customPages: unknown[] } }) => unknown) =>
    selector({ settings: { customPages: [] } }),
}));

vi.mock('@/stores/objectStore', () => ({
  useObjectStore: {
    getState: () => ({
      deleteObject: vi.fn().mockResolvedValue(undefined),
      currentObjectCache: {},
    }),
    // P020 二次复核：直接 invoke 成功后写缓存（不置 isLoading）；测试中 invoke 返回
    // null 不触发，此处仅保证 API 存在。
    setState: vi.fn(),
  },
}));

vi.mock('@/hooks/useDragToAttach', () => ({
  useDragToAttach: () => ({ ref: { current: null }, dragState: 'idle' }),
}));

// ── 样例对象（提供 object prop，跳过拉取）─────────────────────────────────
const sampleObj = {
  id: 'obj-1',
  accountId: 'acc-1',
  name: '护照',
  typeId: 'travel',
  properties: {
    full_name: '张三',
    passport_number: 'E12345678',
    __fields: { full_name: { type: 'text' } },
  },
  sensitivityLevel: 'internal',
  tags: ['旅行', '重要'],
  createdAt: '2026-01-01T00:00:00Z',
  updatedAt: '2026-01-02T00:00:00Z',
} satisfies ObjectData;

describe('ObjectDetailModal', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('渲染头部（对象名/关闭按钮）、标签与底部操作栏', () => {
    render(
      <BrowserRouter>
        <ObjectDetailModal object={sampleObj} onClose={vi.fn()} />
      </BrowserRouter>,
    );

    expect(screen.getByTestId('object-detail-modal')).toBeInTheDocument();
    expect(screen.getByText('护照')).toBeInTheDocument();
    expect(screen.getByTestId('object-detail-close')).toBeInTheDocument();
    // 标签 Pills（ObjectDetailTags 提取后仍正常渲染）
    expect(screen.getByText('旅行')).toBeInTheDocument();
    expect(screen.getByText('重要')).toBeInTheDocument();
    // 底部操作栏（ObjectDetailFooter 提取后仍正常渲染；t 返回 key）
    expect(screen.getByText('common:history')).toBeInTheDocument();
    expect(screen.getByText('common:attachments')).toBeInTheDocument();
    // 使用真实共享保护 hook，模板缺失时仍不能把内部字段原值写入 DOM。
    expect(screen.queryByText('张三')).not.toBeInTheDocument();
    expect(screen.queryByText('E12345678')).not.toBeInTheDocument();
    // 删除确认对话框初始不渲染
    expect(screen.queryByText('common:object_delete_confirm_title')).not.toBeInTheDocument();
  });

  it('点击关闭按钮触发 onClose', () => {
    const onClose = vi.fn();
    render(
      <BrowserRouter>
        <ObjectDetailModal object={sampleObj} onClose={onClose} />
      </BrowserRouter>,
    );

    fireEvent.click(screen.getByTestId('object-detail-close'));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('点击删除打开确认对话框，取消后关闭（ObjectDetailDeleteDialog 提取后链路完整）', () => {
    render(
      <BrowserRouter>
        <ObjectDetailModal object={sampleObj} onClose={vi.fn()} />
      </BrowserRouter>,
    );

    // 打开确认对话框
    fireEvent.click(screen.getByText('common:delete'));
    expect(screen.getByText('common:object_delete_confirm_title')).toBeInTheDocument();
    expect(screen.getByText('common:object_delete_confirm_body')).toBeInTheDocument();

    // 取消 → 对话框关闭
    fireEvent.click(screen.getByText('common:cancel'));
    expect(screen.queryByText('common:object_delete_confirm_title')).not.toBeInTheDocument();
  });

  it('P020 二次复核：传入完整 ObjectData（含 accountId）时不再重复拉取 object_get', async () => {
    const { invokeCommand } = await import('@/lib/ipcClient');
    const invokeMock = vi.mocked(invokeCommand);
    invokeMock.mockClear();
    render(
      <BrowserRouter>
        <ObjectDetailModal object={sampleObj} onClose={vi.fn()} />
      </BrowserRouter>,
    );
    // 完整数据直接可用：不触发 object_get
    expect(invokeMock.mock.calls.filter(([command]) => command === 'object_get')).toEqual([]);
  });

  it('P020 二次复核：传入摘要（无 accountId）时直接 invoke object_get 拉取完整对象（不经全局 action）', async () => {
    const { invokeCommand } = await import('@/lib/ipcClient');
    const invokeMock = vi.mocked(invokeCommand);
    invokeMock.mockClear();
    const summary = {
      id: 'obj-1',
      name: '护照',
      typeId: 'travel',
      sensitivityLevel: 'internal',
      createdAt: '2026-01-01T00:00:00Z',
      updatedAt: '2026-01-02T00:00:00Z',
      properties: { full_name: '张三' }, // 截断摘要形态
    };
    render(
      <BrowserRouter>
        <ObjectDetailModal object={summary} onClose={vi.fn()} />
      </BrowserRouter>,
    );
    expect(invokeMock).toHaveBeenCalledWith(
      'object_get',
      expect.objectContaining({ accountId: 'acc-1', objectId: 'obj-1' }),
      undefined,
    );
    // object_get 返回 Rust Option 的 null → 回退摘要展示，不崩溃
    expect(screen.getByTestId('object-detail-modal')).toBeInTheDocument();
  });

  it('keeps a complete replacement object when the previous summary fetch resolves late', async () => {
    const { invokeCommand } = await import('@/lib/ipcClient');
    const invokeMock = vi.mocked(invokeCommand);
    let resolveOld!: (value: ObjectData) => void;
    const oldRequest = new Promise<ObjectData>((resolve) => {
      resolveOld = resolve;
    });
    invokeMock.mockImplementation((command) => {
      if (command === 'object_get') return oldRequest;
      return Promise.resolve(null);
    });
    const oldSummary = {
      id: 'obj-a',
      name: '摘要 A',
      typeId: 'travel',
      sensitivityLevel: 'internal' as const,
      createdAt: sampleObj.createdAt,
      updatedAt: sampleObj.updatedAt,
    };
    const replacement = { ...sampleObj, id: 'obj-b', name: '完整 B' };
    const view = render(
      <BrowserRouter>
        <ObjectDetailModal object={oldSummary} onClose={vi.fn()} />
      </BrowserRouter>,
    );
    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('object_get', expect.anything(), undefined),
    );

    view.rerender(
      <BrowserRouter>
        <ObjectDetailModal object={replacement} onClose={vi.fn()} />
      </BrowserRouter>,
    );
    expect(screen.getByRole('dialog', { name: '完整 B' })).toBeInTheDocument();

    await act(async () => {
      resolveOld({ ...sampleObj, id: 'obj-a', name: '完整 A' });
      await oldRequest;
    });
    expect(screen.getByRole('dialog', { name: '完整 B' })).toBeInTheDocument();
    expect(screen.queryByRole('dialog', { name: '完整 A' })).not.toBeInTheDocument();
  });

  it('shows the new summary while its fetch waits instead of the previous full object', async () => {
    const { invokeCommand } = await import('@/lib/ipcClient');
    const invokeMock = vi.mocked(invokeCommand);
    let resolveNext!: (value: ObjectData) => void;
    const nextRequest = new Promise<ObjectData>((resolve) => {
      resolveNext = resolve;
    });
    invokeMock.mockImplementation((command, args) => {
      if (command === 'object_get' && (args as { objectId: string }).objectId === 'obj-b') {
        return nextRequest;
      }
      return Promise.resolve(null);
    });
    const view = render(
      <BrowserRouter>
        <ObjectDetailModal object={sampleObj} onClose={vi.fn()} />
      </BrowserRouter>,
    );
    const nextSummary = {
      id: 'obj-b',
      name: '摘要 B',
      typeId: 'travel',
      sensitivityLevel: 'internal' as const,
      createdAt: sampleObj.createdAt,
      updatedAt: sampleObj.updatedAt,
    };
    view.rerender(
      <BrowserRouter>
        <ObjectDetailModal object={nextSummary} onClose={vi.fn()} />
      </BrowserRouter>,
    );
    expect(screen.getByRole('dialog', { name: '摘要 B' })).toBeInTheDocument();
    expect(screen.queryByRole('dialog', { name: sampleObj.name })).not.toBeInTheDocument();

    await act(async () => {
      resolveNext({ ...sampleObj, id: 'obj-b', name: '完整 B' });
      await nextRequest;
    });
    expect(screen.getByRole('dialog', { name: '完整 B' })).toBeInTheDocument();
  });
});
