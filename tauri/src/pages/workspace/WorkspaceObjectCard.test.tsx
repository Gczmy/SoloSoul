import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent, waitFor, within } from '@testing-library/react';
import { isAndroidSync } from '@/lib/platform';
vi.mock('@/lib/platform', async (original) => ({
  ...(await original<typeof import('@/lib/platform')>()),
  isAndroidSync: vi.fn(() => false),
}));
import { WorkspaceObjectCard } from './WorkspaceObjectCard';
import type { ObjectSummary } from '@/stores/objectStore';
import type { UserTemplate } from '@/types/template';
import { useAuthStore } from '@/stores/authStore';
import { resolveSemanticNeedsSync } from '@/lib/templateSync';

vi.mock('@/lib/templateSync', async (original) => ({
  ...(await original<typeof import('@/lib/templateSync')>()),
  resolveSemanticNeedsSync: vi.fn().mockResolvedValue(true),
}));

const baseObj: ObjectSummary = {
  id: 'obj-1',
  name: 'Test Object',
  typeId: 'identity',
  sensitivityLevel: 'internal',
  createdAt: new Date().toISOString(),
  updatedAt: new Date().toISOString(),
  properties: { username: 'alice' },
  tags: ['tag1'],
  templateId: 'tpl-1',
};

const userTemplates: UserTemplate[] = [
  {
    id: 'tpl-1',
    accountId: 'acc-1',
    name: 'Account',
    iconId: 'user',
    properties: [{ id: 'username', name: 'Username', type: 'text', sensitivityLevel: 'public' }],
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
  },
];

describe('WorkspaceObjectCard', () => {
  it('renders history and attachment count badges', () => {
    render(
      <WorkspaceObjectCard
        obj={baseObj}
        collectionLabel="Identity"
        userTemplates={userTemplates}
        snapshotCount={3}
        attachmentCount={2}
        onClick={vi.fn()}
        onHistory={vi.fn()}
        onAttachments={vi.fn()}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
      />,
    );

    expect(screen.getAllByTestId('count-badge-history')[0]).toHaveTextContent('3');
    expect(screen.getAllByTestId('count-badge-attachments')[0]).toHaveTextContent('2');
  });

  it('hides badges when counts are zero or undefined', () => {
    const { rerender } = render(
      <WorkspaceObjectCard
        obj={baseObj}
        collectionLabel="Identity"
        userTemplates={userTemplates}
        snapshotCount={0}
        attachmentCount={0}
        onClick={vi.fn()}
        onHistory={vi.fn()}
        onAttachments={vi.fn()}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
      />,
    );

    expect(screen.queryByTestId('count-badge-history')).not.toBeInTheDocument();
    expect(screen.queryByTestId('count-badge-attachments')).not.toBeInTheDocument();

    rerender(
      <WorkspaceObjectCard
        obj={baseObj}
        collectionLabel="Identity"
        userTemplates={userTemplates}
        onClick={vi.fn()}
        onHistory={vi.fn()}
        onAttachments={vi.fn()}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
      />,
    );

    expect(screen.queryByTestId('count-badge-history')).not.toBeInTheDocument();
    expect(screen.queryByTestId('count-badge-attachments')).not.toBeInTheDocument();
  });

  it('renders field chips from stored properties (incl. date values)', () => {
    const withDate: ObjectSummary = {
      ...baseObj,
      properties: {
        birthDate: '2024-12-31',
        meetTime: '',
        __fields: {
          birthDate: { name: '出生日期', type: 'date' },
          meetTime: { name: '会议时间', type: 'datetime' },
        },
        __templateName: '日程',
      },
    };
    const tplDates: UserTemplate[] = [
      {
        ...userTemplates[0],
        id: 'tpl-1',
        properties: [
          { id: 'birthDate', name: '出生日期', type: 'date', sensitivityLevel: 'public' },
          { id: 'meetTime', name: '会议时间', type: 'datetime', sensitivityLevel: 'public' },
        ],
      },
    ];
    const withDatePublic: ObjectSummary = {
      ...withDate,
      propertyLabels: { birthDate: 'public', meetTime: 'public' },
    };
    render(
      <WorkspaceObjectCard
        obj={withDatePublic}
        collectionLabel="Identity"
        userTemplates={tplDates}
        onClick={vi.fn()}
        onHistory={vi.fn()}
        onAttachments={vi.fn()}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
      />,
    );

    // 日期值必须以 chip 形式显示在卡片上（值为 2024-12-31）
    expect(screen.getByText('出生日期')).toBeInTheDocument();
    expect(screen.getByText('2024-12-31')).toBeInTheDocument();
    // 空字符串日期时间字段不显示 chip
    expect(screen.queryByText('会议时间')).not.toBeInTheDocument();
  });

  it('P118: passes the object to card callbacks (stable handler contract)', () => {
    const onClick = vi.fn();
    const onDelete = vi.fn();
    render(
      <WorkspaceObjectCard
        obj={baseObj}
        collectionLabel="Identity"
        userTemplates={userTemplates}
        onClick={onClick}
        onHistory={vi.fn()}
        onAttachments={vi.fn()}
        onEdit={vi.fn()}
        onDelete={onDelete}
      />,
    );

    // 点击卡片主体 → onClick 收到 obj
    fireEvent.click(screen.getByText('Test Object'));
    expect(onClick).toHaveBeenCalledWith(baseObj);

    // 点击删除按钮 → onDelete 收到 obj（移动端/桌面端两处按钮任意一个）
    fireEvent.click(screen.getAllByTitle('Move to trash')[0]);
    expect(onDelete).toHaveBeenCalledWith(baseObj);
  });
});

it('Android footer shows template and all icon-only field levels, including summary-truncated/group fields', () => {
  vi.mocked(isAndroidSync).mockReturnValue(true);
  const { container } = render(
    <WorkspaceObjectCard
      obj={{
        ...baseObj,
        propertyLabels: {
          username: 'public',
          later: 'sensitive',
          __dynamic_group__: 'critical',
          internal: 'internal',
        },
        properties: {
          username: 'alice',
          __fields: { __dynamic_group__: { name: 'Group', type: 'dynamic_group' } },
        },
      }}
      collectionLabel="Identity"
      userTemplates={userTemplates}
      onClick={vi.fn()}
      onHistory={vi.fn()}
      onAttachments={vi.fn()}
      onEdit={vi.fn()}
      onDelete={vi.fn()}
    />,
  );
  expect(screen.getByText('Identity · Account')).toBeInTheDocument();
  const badges = container.querySelectorAll('.android-row-meta [title]');
  expect(badges).toHaveLength(4);
  for (const badge of badges) {
    expect(badge.querySelector('svg')).not.toBeNull();
    expect(badge.textContent).toBe('');
  }
  vi.mocked(isAndroidSync).mockReturnValue(false);
});

it('对象字段非法敏感度在 Android 汇总为 internal，合法 critical 不降级', () => {
  vi.mocked(isAndroidSync).mockReturnValue(true);
  const { container } = render(
    <WorkspaceObjectCard
      obj={{
        ...baseObj,
        properties: { username: 'alice', secret: '123' },
        propertyLabels: { username: 'unknown', secret: 'critical' },
      }}
      collectionLabel="Identity"
      userTemplates={userTemplates}
      onClick={vi.fn()}
      onHistory={vi.fn()}
      onAttachments={vi.fn()}
      onEdit={vi.fn()}
      onDelete={vi.fn()}
    />,
  );

  const badges = container.querySelectorAll('.android-row-meta [title]');
  expect(badges).toHaveLength(2);
  expect(screen.getByTitle('sensitivity_label: internal')).toBeInTheDocument();
  expect(screen.getByTitle('sensitivity_label: critical')).toBeInTheDocument();
  expect(screen.queryByTitle('sensitivity_label: unknown')).not.toBeInTheDocument();
  vi.mocked(isAndroidSync).mockReturnValue(false);
});

it('模板字段非法敏感度在 Android 汇总为 internal', () => {
  vi.mocked(isAndroidSync).mockReturnValue(true);
  const storedTemplate = {
    ...userTemplates[0],
    properties: [{ id: 'username', name: 'Username', type: 'text', sensitivityLevel: 'unknown' }],
  } as unknown as UserTemplate;
  render(
    <WorkspaceObjectCard
      obj={baseObj}
      collectionLabel="Identity"
      userTemplates={[storedTemplate]}
      onClick={vi.fn()}
      onHistory={vi.fn()}
      onAttachments={vi.fn()}
      onEdit={vi.fn()}
      onDelete={vi.fn()}
    />,
  );

  expect(screen.getByTitle('sensitivity_label: internal')).toBeInTheDocument();
  vi.mocked(isAndroidSync).mockReturnValue(false);
});

it('desktop template update actions target the card object without opening its detail', async () => {
  vi.mocked(isAndroidSync).mockReturnValue(false);
  vi.mocked(resolveSemanticNeedsSync).mockResolvedValue(true);
  useAuthStore.setState({ currentAccount: { id: 'acc-1', name: 'A' } });
  const onClick = vi.fn();
  const onSync = vi.fn();
  const onDismissSync = vi.fn();
  render(
    <WorkspaceObjectCard
      obj={baseObj}
      collectionLabel="Identity"
      userTemplates={userTemplates}
      templateHashMap={new Map([['tpl-1', 'new-hash']])}
      onClick={onClick}
      onHistory={vi.fn()}
      onAttachments={vi.fn()}
      onEdit={vi.fn()}
      onDelete={vi.fn()}
      onSync={onSync}
      onDismissSync={onDismissSync}
    />,
  );

  const hint = await screen.findByText('editor:template_updated_hint');
  const banner = hint.parentElement as HTMLElement;
  fireEvent.click(within(banner).getByRole('button', { name: 'common:yes' }));
  fireEvent.click(within(banner).getByRole('button', { name: 'common:no' }));
  expect(onSync).toHaveBeenCalledExactlyOnceWith(baseObj);
  expect(onDismissSync).toHaveBeenCalledExactlyOnceWith(baseObj);
  expect(onClick).not.toHaveBeenCalled();
  expect(resolveSemanticNeedsSync).toHaveBeenCalledWith('acc-1', 'obj-1');
});

it('Android template update actions route through the object menu without opening its detail', async () => {
  vi.mocked(isAndroidSync).mockReturnValue(true);
  vi.mocked(resolveSemanticNeedsSync).mockResolvedValue(true);
  useAuthStore.setState({ currentAccount: { id: 'acc-1', name: 'A' } });
  const onClick = vi.fn();
  const onSync = vi.fn();
  const onDismissSync = vi.fn();
  render(
    <WorkspaceObjectCard
      obj={baseObj}
      collectionLabel="Identity"
      userTemplates={userTemplates}
      templateHashMap={new Map([['tpl-1', 'new-hash']])}
      onClick={onClick}
      onHistory={vi.fn()}
      onAttachments={vi.fn()}
      onEdit={vi.fn()}
      onDelete={vi.fn()}
      onSync={onSync}
      onDismissSync={onDismissSync}
    />,
  );

  await waitFor(() =>
    expect(screen.getByLabelText('editor:template_updated_hint')).toBeInTheDocument(),
  );
  const openActions = () =>
    fireEvent.click(screen.getByRole('button', { name: 'material.object_actions' }));
  openActions();
  fireEvent.click(screen.getByRole('button', { name: 'editor:template_updated_hint' }));
  openActions();
  fireEvent.click(screen.getByRole('button', { name: 'material.skip_sync' }));
  expect(onSync).toHaveBeenCalledExactlyOnceWith(baseObj);
  expect(onDismissSync).toHaveBeenCalledExactlyOnceWith(baseObj);
  expect(onClick).not.toHaveBeenCalled();
});
