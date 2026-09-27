import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { useAuthStore } from '@/stores/authStore';
import type { UserTemplate } from '@/types/template';
import { TrashFieldList } from './TrashDetailSections';
import { SnapshotContent, SnapshotDataView } from './TrashSnapshotView';
import type { SnapshotEntry, TrashDetail } from './types';

// 仅替换密码对话框的交互边界；等级解析、保护值、Auth Store 和快照组件均使用生产实现。
vi.mock('@/components/forms/PasswordVerificationDialog', () => ({
  PasswordVerificationDialog: ({
    onVerify,
    onClose,
  }: {
    onVerify: (password: string) => Promise<boolean>;
    onClose: () => void;
  }) => (
    <div role="dialog">
      <button onClick={() => void onVerify('rf109-synthetic-password')}>RF109 verify</button>
      <button onClick={onClose}>RF109 cancel</button>
    </div>
  ),
}));

const ACCOUNT = { id: 'rf109-synthetic-account', name: 'RF109 synthetic account' };
const PRIVATE = 'RF109_PRIVATE_VALUE';
const PUBLIC = 'RF109_VISIBLE_PUBLIC_SIBLING';
const SURFACES = ['preview', 'snapshot'] as const;
type Surface = (typeof SURFACES)[number];

function trashItem(previewProperties: TrashDetail['previewProperties']): TrashDetail {
  return {
    id: 'rf109-trash-item',
    originalId: 'rf109-object',
    itemType: 'object',
    name: 'RF109 synthetic object',
    deletedAt: 0,
    deletedBy: 'user',
    originalLocation: 'identity',
    previewProperties,
    attachments: [],
    deletedAttachments: [],
    snapshots: [],
    childItems: [],
  };
}

function groupView(surface: Surface, value: unknown) {
  return surface === 'preview' ? (
    <TrashFieldList
      item={trashItem([
        {
          fieldId: 'group',
          key: 'Group',
          type: 'dynamic_group',
          sensitivityLevel: 'public',
          value,
        },
      ])}
    />
  ) : (
    <SnapshotDataView
      data={{
        properties: {
          __fields: { group: { name: 'Group', type: 'dynamic_group', sensitivityLevel: 'public' } },
          group: value,
        },
      }}
      detailTemplate={null}
    />
  );
}

function expectConcealed(value: string) {
  // 连属性/title/可访问名称一起检查，不能仅把正文改成 blur 或透明。
  expect(document.body.innerHTML).not.toContain(value);
}

function auditCalls() {
  return vi.mocked(invoke).mock.calls.filter(([command]) => command === 'log_write');
}

beforeEach(() => {
  vi.mocked(invoke).mockReset().mockResolvedValue(undefined);
  useAuthStore.setState({ currentAccount: null, isAuthenticated: false });
  useAuthStore.getState().completeUnlock(ACCOUNT);
});
afterEach(() => {
  cleanup();
  useAuthStore.setState({ currentAccount: null, isAuthenticated: false });
});

describe('RF109 真实回收站字段保护入口', () => {
  it.each(['', null])('预览显式非法等级 %s 不得回退到对象 public 标签', (level) => {
    // 模拟历史/损坏元数据，故意跨过 DTO 的合法字符串类型约束。
    const field = {
      fieldId: 'secret',
      key: 'Secret',
      type: 'text',
      value: PRIVATE,
      sensitivityLevel: level,
    } as unknown as TrashDetail['previewProperties'][number];
    render(
      <TrashFieldList item={{ ...trashItem([field]), propertyLabels: { secret: 'public' } }} />,
    );
    expectConcealed(PRIVATE);
    expect(screen.getByRole('button', { name: /^Secret:/ })).toBeVisible();
    expect(invoke).not.toHaveBeenCalled();
  });

  it.each(['propertyLabels', '__fields'] as const)(
    '快照 %s 的显式非法等级不得回退到公开定义或模板',
    (source) => {
      const template: UserTemplate = {
        id: 'rf109-public-template',
        accountId: ACCOUNT.id,
        name: 'Synthetic template',
        createdAt: '2026-01-01T00:00:00Z',
        properties: [{ id: 'secret', name: 'Secret', type: 'text', sensitivityLevel: 'public' }],
      };
      const data = {
        ...(source === 'propertyLabels' ? { propertyLabels: { secret: null } } : {}),
        properties: {
          secret: PRIVATE,
          __fields: {
            secret: {
              name: 'Secret',
              type: 'text',
              sensitivityLevel: source === '__fields' ? '' : 'public',
            },
          },
        },
      };
      render(<SnapshotDataView data={data} detailTemplate={template} />);
      expectConcealed(PRIVATE);
      expect(screen.getByRole('button', { name: /^Secret:/ })).toBeVisible();
      expect(invoke).not.toHaveBeenCalled();
    },
  );

  it.each(SURFACES)(
    '%s 的 public 父组不公开缺失/null/未知等级子字段，显式 public 兄弟仍可见',
    (surface) => {
      const children = [
        { id: 'missing', name: 'Missing', type: 'text', value: 'RF109_MISSING_SECRET' },
        {
          id: 'null',
          name: 'Null',
          type: 'text',
          sensitivityLevel: null,
          value: 'RF109_NULL_SECRET',
        },
        {
          id: 'invalid',
          name: 'Invalid',
          type: 'text',
          sensitivityLevel: 'invalid',
          value: 'RF109_INVALID_SECRET',
        },
        {
          id: 'public',
          name: 'Public sibling',
          type: 'text',
          sensitivityLevel: 'public',
          value: PUBLIC,
        },
      ];
      render(groupView(surface, children));
      expect(screen.getByText(PUBLIC)).toBeVisible();
      expect(screen.queryByRole('button', { name: /^Public sibling:/ })).not.toBeInTheDocument();
      for (const value of ['RF109_MISSING_SECRET', 'RF109_NULL_SECRET', 'RF109_INVALID_SECRET']) {
        expect.soft(document.body.innerHTML).not.toContain(value);
      }
      expect(invoke).not.toHaveBeenCalled();
    },
  );

  it.each([
    ['preview', 'array'],
    ['preview', 'json'],
    ['snapshot', 'array'],
    ['snapshot', 'json'],
  ] as const)(
    '%s 的 %s 嵌套公开组包含 critical 后代时，整段值必须经过验证',
    async (surface, encoding) => {
      const descendants = [
        {
          id: 'critical',
          name: 'Protected descendant',
          type: 'text',
          sensitivityLevel: 'critical',
          value: PRIVATE,
        },
      ];
      const children = [
        {
          id: 'nested',
          name: 'Nested',
          type: 'dynamic_group',
          sensitivityLevel: 'public',
          value: encoding === 'json' ? JSON.stringify(descendants) : descendants,
        },
        {
          id: 'public',
          name: 'Public sibling',
          type: 'text',
          sensitivityLevel: 'public',
          value: PUBLIC,
        },
      ];
      render(groupView(surface, encoding === 'json' ? JSON.stringify(children) : children));
      expect(screen.getByText(PUBLIC)).toBeVisible();
      expectConcealed(PRIVATE);
      fireEvent.click(screen.getByRole('button', { name: /^Nested:/ }));
      expect(screen.getByRole('dialog')).toBeVisible();
      expectConcealed(PRIVATE);
      expect(invoke).not.toHaveBeenCalled();
      vi.mocked(invoke).mockImplementation(async (command) => command === 'verify_password');
      await act(async () => fireEvent.click(screen.getByRole('button', { name: 'RF109 verify' })));
      expect(screen.getByText((text) => text.includes(PRIVATE))).toBeVisible();
      expect(auditCalls()).toHaveLength(1);
      expect(auditCalls()[0][1]).toEqual({
        request: {
          actionType: 'critical_field_login',
          entityType: 'auth',
          entityId: null,
          entityName: null,
          details: 'source=trash fieldName=Nested',
        },
      });
    },
  );

  it('模板预览仍公开字段类型，不显示业务值或要求验证', () => {
    const item = trashItem([
      { key: 'Phone definition', type: 'phone', sensitivityLevel: 'critical', value: PRIVATE },
    ]);
    render(<TrashFieldList item={{ ...item, itemType: 'template' }} />);
    expect(screen.getByText('editor:field_types.phone')).toBeVisible();
    expectConcealed(PRIVATE);
    expect(screen.queryByRole('button')).not.toBeInTheDocument();
    expect(invoke).not.toHaveBeenCalled();
  });

  it('schemaOnly 快照仍公开定义内容而不显示揭示按钮', () => {
    render(
      <SnapshotDataView
        schemaOnly
        detailTemplate={null}
        data={{
          properties: {
            __fields: { kind: { name: 'Field kind', type: 'text', sensitivityLevel: 'critical' } },
            kind: 'phone',
          },
        }}
      />,
    );
    expect(screen.getByText('phone')).toBeVisible();
    expect(screen.queryByRole('button')).not.toBeInTheDocument();
    expect(invoke).not.toHaveBeenCalled();
  });

  it.each(['snapshot', 'lock'] as const)(
    '%s 变化后旧验证不得揭示或审计，当前快照仍可重新验证',
    async (change) => {
      let finish!: (ok: boolean) => void;
      const pending = new Promise<boolean>((resolve) => {
        finish = resolve;
      });
      let verifications = 0;
      vi.mocked(invoke).mockImplementation(async (command) => {
        if (command === 'verify_password') return ++verifications === 1 ? pending : true;
        return undefined;
      });
      const snapshots: SnapshotEntry[] = ['one', 'two'].map((id, index) => ({
        id,
        timestamp: 1_700_000_000_000 + index,
        triggeredBy: 'manual',
        diffSummary: 'RF109 synthetic snapshot',
      }));
      // 两快照故意正文完全相同，测试必须依赖真实 snapshot ID 隔离，不能只依赖值变化。
      const data = {
        properties: {
          __fields: { pin: { name: 'PIN', type: 'text', sensitivityLevel: 'critical' } },
          pin: PRIVATE,
        },
      };
      const viewAt = (index: number) => (
        <SnapshotContent
          _detailId="rf109-trash-item"
          snapshots={snapshots}
          currentSnapIdx={index}
          data={data}
          loading={false}
          detailTemplate={null}
          onChangeSnapshot={vi.fn()}
        />
      );
      const view = render(viewAt(0));
      try {
        fireEvent.click(screen.getByRole('button', { name: /^PIN:/ }));
        fireEvent.click(screen.getByRole('button', { name: 'RF109 verify' }));
        expect(verifications).toBe(1);
        if (change === 'snapshot') view.rerender(viewAt(1));
        else
          act(() => {
            useAuthStore.setState({ currentAccount: null, isAuthenticated: false });
            useAuthStore.getState().completeUnlock(ACCOUNT);
          });
        await act(async () => finish(true));
        expectConcealed(PRIVATE);
        expect(auditCalls()).toHaveLength(0);
        fireEvent.click(screen.getByRole('button', { name: /^PIN:/ }));
        await act(async () =>
          fireEvent.click(screen.getByRole('button', { name: 'RF109 verify' })),
        );
        expect(screen.getByText(PRIVATE)).toBeVisible();
        expect(verifications).toBe(2);
        expect(auditCalls()).toHaveLength(1);
      } finally {
        // 断言失败也释放真实授权边界的受控 Promise，不留下悬停验证。
        await act(async () => finish(false));
      }
    },
  );
});
