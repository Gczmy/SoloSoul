import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import { SyncConflictDialog } from './SyncConflictDialog';

const { stableT } = vi.hoisted(() => ({
  stableT: (key: string, options?: { defaultValue?: string }) => options?.defaultValue ?? key,
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: stableT,
    i18n: { language: 'en', changeLanguage: vi.fn(() => Promise.resolve()) },
  }),
  I18nextProvider: ({ children }: { children: React.ReactNode }) => children,
}));

vi.mock('@/lib/ipcClient', () => ({
  invokeCommand: vi.fn(() => Promise.resolve(undefined)),
}));

vi.mock('@/stores/syncStore', () => ({
  useSyncStore: {
    getState: () => ({
      loadConflictDetail: vi.fn(() => Promise.resolve(undefined)),
    }),
  },
}));

import type { SyncConflictSummary, SyncConflictDetail, SyncConflictStrategy } from '@/lib/ipc';

const conflict: SyncConflictSummary = {
  id: 'conf-1',
  table: 'objects',
  record_id: 'obj-1',
  local_hlc: { wall_time_ms: 100, counter: 1, node_id: 'aaaa' },
  remote_hlc: { wall_time_ms: 90, counter: 1, node_id: 'bbbb' },
  winner: 'local',
  created_at: '2026-01-01T00:00:00Z',
};

const detail: SyncConflictDetail = {
  id: 'conf-1',
  table: 'objects',
  record_id: 'obj-1',
  local_hlc: { wall_time_ms: 100, counter: 1, node_id: 'aaaa' },
  remote_hlc: { wall_time_ms: 90, counter: 1, node_id: 'bbbb' },
  local_data: { name: '张三', age: 30 },
  remote_data: { name: '李四', age: 30 },
  remote_deleted: false,
  winner: 'local',
  created_at: '2026-01-01T00:00:00Z',
};

describe('SyncConflictDialog (P027 渲染回归)', () => {
  const onClose = vi.fn();
  const onResolve = vi.fn();

  beforeEach(() => {
    onClose.mockClear();
    onResolve.mockClear();
  });

  it('isOpen=false 时不渲染内容', () => {
    render(
      <SyncConflictDialog
        isOpen={false}
        conflicts={[conflict]}
        detail={null}
        isLoading={false}
        onClose={onClose}
        onResolve={onResolve}
      />,
    );
    expect(screen.queryByText('No unresolved conflicts.')).not.toBeInTheDocument();
  });

  it('渲染冲突列表并默认选中第一条（ConflictFieldRow 提取后字段 diff 完整渲染）', async () => {
    render(
      <SyncConflictDialog
        isOpen
        conflicts={[conflict]}
        detail={detail}
        isLoading={false}
        onClose={onClose}
        onResolve={onResolve}
      />,
    );

    // 冲突列表行（record_id 在列表行与详情头部各出现一次）
    expect(screen.getAllByText('obj-1').length).toBeGreaterThanOrEqual(1);
    // 字段级 diff 行（ConflictFieldRow）：name 无 i18n 命中时 humanize 为 Name，本地/远程值渲染
    await waitFor(() => {
      expect(screen.getByText('Name')).toBeInTheDocument();
    });
    expect(screen.getByText('张三')).toBeInTheDocument();
    expect(screen.getByText('李四')).toBeInTheDocument();
    // 未变化字段 age 标题化后也渲染（非只看差异模式）
    expect(screen.getByText('Age')).toBeInTheDocument();
  });

  it('切换「只看差异」后隐藏未变化字段行', async () => {
    render(
      <SyncConflictDialog
        isOpen
        conflicts={[conflict]}
        detail={detail}
        isLoading={false}
        onClose={onClose}
        onResolve={onResolve}
      />,
    );
    await waitFor(() => {
      expect(screen.getByText('Name')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole('checkbox'));
    // age 未变化 → 被过滤（标题化后为 Age）
    expect(screen.queryByText('Age')).not.toBeInTheDocument();
    // name 有差异 → 仍显示
    expect(screen.getByText('Name')).toBeInTheDocument();
  });

  it('解析按钮触发 onResolve（Keep Local）', async () => {
    render(
      <SyncConflictDialog
        isOpen
        conflicts={[conflict]}
        detail={detail}
        isLoading={false}
        onClose={onClose}
        onResolve={onResolve}
      />,
    );
    await waitFor(() => {
      expect(screen.getByText('Name')).toBeInTheDocument();
    });
    fireEvent.click(screen.getByText('Keep Local'));
    expect(onResolve).toHaveBeenCalledWith('conf-1', 'keep_local' satisfies SyncConflictStrategy);
  });
});
