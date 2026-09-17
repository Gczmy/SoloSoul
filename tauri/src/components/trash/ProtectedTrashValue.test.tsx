import { act, fireEvent, render, screen, cleanup } from '@testing-library/react';
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
      <button onClick={() => void onVerify('test-password')}>verify</button>
      <button onClick={onClose}>cancel</button>
    </div>
  ),
}));

beforeEach(() => {
  vi.mocked(invoke).mockReset().mockResolvedValue(undefined);
  useAuthStore.setState({ isAuthenticated: true, currentAccount: { id: 'acc-a', name: 'A' } });
});
afterEach(() => {
  cleanup();
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
    expect(trashSensitivity(undefined, 'public')).toBe('public');
    expect(trashSensitivity('public', 'critical')).toBe('critical');
  });
  it.each(['internal', 'sensitive'] as const)(
    '%s 默认不将明文放入 DOM，揭示一分钟后隐藏',
    (level) => {
      vi.useFakeTimers();
      const view = render(field(level));
      expect(view.container.innerHTML).not.toContain('secret-value');
      reveal();
      expect(screen.getByText('secret-value')).toBeVisible();
      act(() => vi.advanceTimersByTime(60_000));
      expect(view.container.innerHTML).not.toContain('secret-value');
      expect(invoke).not.toHaveBeenCalled();
    },
  );
  it('public 直接显示，内容切换后不继承揭示状态', () => {
    const view = render(field('public'));
    expect(screen.getByText('secret-value')).toBeVisible();
    view.rerender(field('sensitive'));
    reveal();
    view.rerender(field('sensitive', 'new-secret'));
    expect(view.container.innerHTML).not.toContain('new-secret');
  });
  it('critical 只有主密码验证成功才能揭示', async () => {
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
  });
  it.each(['cancel', 'account', 'content', 'unmount'] as const)(
    '%s 后丢弃迟到验证',
    async (change) => {
      let finish!: (v: boolean) => void;
      vi.mocked(invoke).mockReturnValueOnce(
        new Promise<boolean>((r) => {
          finish = r;
        }),
      );
      const view = render(field());
      reveal();
      fireEvent.click(screen.getByText('verify'));
      if (change === 'cancel') fireEvent.click(screen.getByText('cancel'));
      if (change === 'account')
        act(() => useAuthStore.setState({ currentAccount: { id: 'acc-b', name: 'B' } }));
      if (change === 'content') view.rerender(field('critical', 'new-secret'));
      if (change === 'unmount') view.unmount();
      await act(async () => finish(true));
      expect(view.container.innerHTML).not.toContain('secret-value');
      expect(view.container.innerHTML).not.toContain('new-secret');
      expect(invoke).toHaveBeenCalledTimes(1);
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
