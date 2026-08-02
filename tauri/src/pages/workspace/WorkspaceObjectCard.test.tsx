import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { WorkspaceObjectCard } from './WorkspaceObjectCard';
import type { ObjectSummary } from '@/stores/objectStore';
import type { UserTemplate } from '@/types/template';

const baseObj: ObjectSummary = {
  id: 'obj-1',
  name: 'Test Object',
  collectionType: 'identity',
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
