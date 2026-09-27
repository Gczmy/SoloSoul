import { act, fireEvent, render, screen, cleanup, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { useAuthStore } from '@/stores/authStore';
import { ProtectedTrashValue, trashSensitivity } from './ProtectedTrashValue';
import { SnapshotDataView } from './TrashSnapshotView';
import { TrashFieldList } from './TrashDetailSections';
import type { TrashDetail } from './types';

vi.mock('@/components/forms/PasswordVerificationDialog', () => ({
  PasswordVerificationDialog: ({
    onVerify,
    onClose,
  }: {
    onVerify: (s: string) => Promise<boolean>;
    onClose: () => void;
  }) => (
    <div role="dialog">
      <button
        onClick={async () => {
          // 对齐真实 usePasswordVerificationFlows：验证成功后自动关闭对话框。
          if (await onVerify('test-password')) onClose();
        }}
      >
        verify
      </button>
      <button onClick={onClose}>cancel</button>
    </div>
  ),
}));

beforeEach(() => {
  vi.mocked(invoke).mockReset().mockResolvedValue(undefined);
  useAuthStore.setState({ isAuthenticated: false, currentAccount: null });
  useAuthStore.getState().completeUnlock({ id: 'acc-a', name: 'A' });
});
afterEach(() => {
  cleanup();
  useAuthStore.setState({ isAuthenticated: false, currentAccount: null });
  vi.useRealTimers();
});

function field(
  sensitivity: 'public' | 'internal' | 'sensitive' | 'critical' = 'critical',
  value = 'secret-value',
) {
  return (
    <ProtectedTrashValue identity="field" label="PIN" value={value} sensitivity={sensitivity} />
  );
}
function reveal() {
  fireEvent.click(screen.getByRole('button', { name: /PIN:/ }));
}

describe('回收站字段保护', () => {
  it('未知敏感度保护为 internal，子字段不得降低父组保护', () => {
    expect(trashSensitivity(undefined)).toBe('internal');
    expect(trashSensitivity('invalid', 'public')).toBe('internal');
    expect(trashSensitivity(undefined, 'public')).toBe('internal');
    expect(trashSensitivity('public', 'critical')).toBe('critical');
  });
  it.each(['internal', 'sensitive'] as const)(
    '%s 默认不将明文放入 DOM，揭示一分钟后隐藏',
    async (level) => {
      vi.useFakeTimers();
      const view = render(field(level));
      expect(view.container.innerHTML).not.toContain('secret-value');
      await act(async () => reveal());
      expect(screen.getByText('secret-value')).toBeVisible();
      act(() => vi.advanceTimersByTime(60_000));
      expect(view.container.innerHTML).not.toContain('secret-value');
      expect(invoke).not.toHaveBeenCalled();
    },
  );
  it('public 直接显示，内容切换后不继承揭示状态', async () => {
    const view = render(field('public'));
    expect(screen.getByText('secret-value')).toBeVisible();
    view.rerender(field('sensitive'));
    await act(async () => reveal());
    expect(screen.getByText('secret-value')).toBeVisible();
    view.rerender(field('sensitive', 'new-secret'));
    expect(view.container.innerHTML).not.toContain('new-secret');
  });
  it('critical 只有主密码验证成功才能揭示，成功自动关闭对话框不重新隐藏', async () => {
    const view = render(field());
    reveal();
    vi.mocked(invoke).mockResolvedValueOnce(false);
    await act(async () => fireEvent.click(screen.getByText('verify')));
    expect(view.container.innerHTML).not.toContain('secret-value');
    vi.mocked(invoke).mockResolvedValueOnce(true);
    await act(async () => fireEvent.click(screen.getByText('verify')));
    expect(invoke).toHaveBeenCalledWith('verify_password', {
      accountId: 'acc-a',
      password: 'test-password',
    });
    expect(screen.getByText('secret-value')).toBeVisible();
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    expect(
      vi.mocked(invoke).mock.calls.filter(([command]) => command === 'log_write'),
    ).toHaveLength(1);
  });
  it('critical 审计失败按 best effort 处理，已获准值仍展示且对话框正常关闭', async () => {
    vi.mocked(invoke).mockImplementation(async (command) => {
      if (command === 'verify_password') return true;
      if (command === 'log_write') throw new Error('RF109 synthetic audit failure');
      return undefined;
    });
    render(field());
    reveal();
    await act(async () => fireEvent.click(screen.getByText('verify')));
    expect(screen.getByText('secret-value')).toBeVisible();
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    expect(
      vi.mocked(invoke).mock.calls.filter(([command]) => command === 'log_write'),
    ).toHaveLength(1);
    expect(invoke).toHaveBeenCalledWith('log_write', {
      request: {
        actionType: 'critical_field_login',
        entityType: 'auth',
        entityId: null,
        entityName: null,
        details: 'source=trash fieldName=PIN',
      },
    });
  });
  it.each(['cancel', 'lock'] as const)(
    'log_write 已发出但等待返回时 %s，迟到审计不能揭示且新授权仍可用',
    async (change) => {
      let finishAudit!: () => void;
      const pendingAudit = new Promise<void>((resolve) => {
        finishAudit = resolve;
      });
      let audits = 0;
      vi.mocked(invoke).mockImplementation(async (command) => {
        if (command === 'verify_password') return true;
        if (command === 'log_write') {
          audits += 1;
          if (audits === 1) await pendingAudit;
        }
        return undefined;
      });
      const calls = (command: string) =>
        vi.mocked(invoke).mock.calls.filter(([name]) => name === command);
      const view = render(field());
      try {
        reveal();
        fireEvent.click(screen.getByText('verify'));
        // 先证明真实适配器已通过密码验证并派发审计，再改变用户意图/会话。
        await waitFor(() => expect(calls('log_write')).toHaveLength(1));
        expect(calls('verify_password')).toHaveLength(1);
        expect(view.container.innerHTML).not.toContain('secret-value');
        if (change === 'cancel') fireEvent.click(screen.getByText('cancel'));
        else act(() => useAuthStore.setState({ isAuthenticated: false }));
        expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
        await act(async () => finishAudit());
        expect(view.container.innerHTML).not.toContain('secret-value');
        expect(calls('verify_password')).toHaveLength(1);
        // 这条审计已发送；只要求迟到结果不授权，不声称可以撤销它。
        expect(calls('log_write')).toHaveLength(1);
        if (change === 'lock')
          act(() => useAuthStore.getState().completeUnlock({ id: 'acc-a', name: 'A' }));
        expect(view.container.innerHTML).not.toContain('secret-value');
        reveal();
        await act(async () => fireEvent.click(screen.getByText('verify')));
        expect(screen.getByText('secret-value')).toBeVisible();
        expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
        expect(calls('verify_password')).toHaveLength(2);
        expect(calls('log_write')).toHaveLength(2);
      } finally {
        await act(async () => finishAudit());
      }
    },
  );
  it.each(['internal', 'critical'] as const)(
    '锁定但保留 accountId 后新的 %s 揭示不能验证、审计或显示原文',
    async (level) => {
      const view = render(field(level));
      act(() => useAuthStore.setState({ isAuthenticated: false }));
      expect(useAuthStore.getState().currentAccount?.id).toBe('acc-a');
      await act(async () => reveal());
      const verify = screen.queryByRole('button', { name: 'verify' });
      if (verify) await act(async () => fireEvent.click(verify));
      expect(view.container.innerHTML).not.toContain('secret-value');
      expect(invoke).not.toHaveBeenCalled();
    },
  );
  it.each(['cancel', 'account', 'lock', 'content', 'unmount'] as const)(
    '%s 后丢弃迟到验证',
    async (change) => {
      let finish!: (v: boolean) => void;
      vi.mocked(invoke).mockReturnValueOnce(
        new Promise<boolean>((r) => {
          finish = r;
        }),
      );
      const view = render(field());
      try {
        reveal();
        fireEvent.click(screen.getByText('verify'));
        expect(invoke).toHaveBeenCalledWith('verify_password', {
          accountId: 'acc-a',
          password: 'test-password',
        });
        if (change === 'cancel') fireEvent.click(screen.getByText('cancel'));
        if (change === 'account')
          act(() => useAuthStore.setState({ currentAccount: { id: 'acc-b', name: 'B' } }));
        if (change === 'lock') act(() => useAuthStore.setState({ isAuthenticated: false }));
        if (change === 'content') view.rerender(field('critical', 'new-secret'));
        if (change === 'unmount') view.unmount();
        await act(async () => finish(true));
        expect(view.container.innerHTML).not.toContain('secret-value');
        expect(view.container.innerHTML).not.toContain('new-secret');
        expect(invoke).toHaveBeenCalledTimes(1);
        expect(
          vi.mocked(invoke).mock.calls.filter(([command]) => command === 'log_write'),
        ).toHaveLength(0);
      } finally {
        await act(async () => finish(false));
      }
    },
  );
  it('普通预览、动态字段和快照均保护值，模板定义仍可读', () => {
    const item: TrashDetail = {
      id: 'trash-a',
      itemType: 'object',
      originalId: 'obj-a',
      name: 'Object',
      deletedAt: 0,
      deletedBy: 'user',
      originalLocation: 'page',
      attachments: [],
      deletedAttachments: [],
      snapshots: [],
      childItems: [],
      previewProperties: [
        { key: 'scalar', type: 'text', value: 'scalar-secret', sensitivityLevel: 'internal' },
        {
          key: 'group',
          type: 'dynamic_group',
          sensitivityLevel: 'sensitive',
          value: [
            { id: 'child', name: 'child', value: 'child-secret', sensitivityLevel: 'public' },
          ],
        },
      ],
    };
    const view = render(<TrashFieldList item={item} />);
    expect(view.container.innerHTML).not.toContain('scalar-secret');
    expect(view.container.innerHTML).not.toContain('child-secret');
    view.rerender(<TrashFieldList item={{ ...item, itemType: 'template' }} />);
    expect(screen.getByText('editor:field_types.text')).toBeVisible();
    expect(screen.queryByRole('button')).not.toBeInTheDocument();
    const data = {
      properties: {
        scalar: 'scalar-secret',
        group: [{ name: 'child', value: 'child-secret', sensitivityLevel: 'critical' }],
        __fields: { group: { type: 'dynamic_group', sensitivityLevel: 'public' } },
      },
    };
    view.rerender(<SnapshotDataView data={data} detailTemplate={null} />);
    expect(view.container.innerHTML).not.toContain('scalar-secret');
    expect(view.container.innerHTML).not.toContain('child-secret');
    fireEvent.click(screen.getByRole('button', { name: /child:/ }));
    expect(screen.getByRole('dialog')).toBeVisible();
    view.rerender(<SnapshotDataView data={data} detailTemplate={null} schemaOnly />);
    expect(screen.getByText('child-secret')).toBeVisible();
  });
});
