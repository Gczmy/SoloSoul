import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { SnapshotDataView } from './TrashDetailPanel';
import type { SnapshotDataViewProps } from './TrashDetailPanel';
import type { UserTemplate } from '@/types/template';

function renderSnapshot(props: Partial<SnapshotDataViewProps> = {}) {
  const defaultData: SnapshotDataViewProps['data'] = {
    name: 'Test Object',
    tags: [],
    properties: {},
  };
  return render(
    <SnapshotDataView
      data={defaultData}
      detailTemplate={null}
      currentPropertyLabels={undefined}
      {...props}
    />,
  );
}

describe('SnapshotDataView', () => {
  it('uses snapshot propertyLabels instead of current object labels', () => {
    renderSnapshot({
      data: {
        name: 'Test Object',
        tags: [],
        properties: {
          __fields: {
            fullName: { name: '姓名', type: 'text', sensitivityLevel: 'public' },
          },
          fullName: 'Alice',
        },
        propertyLabels: { fullName: 'internal' },
      },
      // 对象被删除时的当前敏感度是 critical，不应覆盖快照自身的 internal
      currentPropertyLabels: { fullName: 'critical' },
    });

    expect(screen.getByText('internal')).toBeInTheDocument();
    expect(screen.queryByText('critical')).not.toBeInTheDocument();
    expect(screen.queryByText('public')).not.toBeInTheDocument();
  });

  it('falls back to __fields sensitivity when snapshot propertyLabels is missing', () => {
    renderSnapshot({
      data: {
        name: 'Test Object',
        tags: [],
        properties: {
          __fields: {
            fullName: { name: '姓名', type: 'text', sensitivityLevel: 'sensitive' },
          },
          fullName: 'Bob',
        },
      },
      currentPropertyLabels: { fullName: 'critical' },
    });

    expect(screen.getByText('sensitive')).toBeInTheDocument();
    expect(screen.queryByText('critical')).not.toBeInTheDocument();
  });

  it('uses template default sensitivity when snapshot has neither propertyLabels nor __fields', () => {
    const detailTemplate = {
      id: 'tpl-1',
      name: 'ID Card',
      category: 'identity',
      icon: 'id',
      accountId: 'acc-1',
      createdAt: '2026-01-01T00:00:00Z',
      properties: [
        {
          id: 'fullName',
          name: '姓名',
          type: 'text',
          sensitivityLevel: 'private',
        },
      ],
    } as unknown as UserTemplate;

    renderSnapshot({
      data: {
        name: 'Test Object',
        tags: [],
        properties: {
          fullName: 'Carol',
        },
      },
      detailTemplate,
      currentPropertyLabels: { fullName: 'critical' },
    });

    expect(screen.getByText('private')).toBeInTheDocument();
    expect(screen.queryByText('critical')).not.toBeInTheDocument();
  });

  it('uses snapshot propertyLabels for dynamic_group parent sensitivity', () => {
    renderSnapshot({
      data: {
        name: 'Test Object',
        tags: [],
        properties: {
          __fields: {
            contacts: {
              name: '联系方式',
              type: 'dynamic_group',
              sensitivityLevel: 'public',
            },
          },
          contacts: [
            { id: 'c1', name: '手机', type: 'phone', value: '123' },
          ],
        },
        propertyLabels: { contacts: 'critical' },
      },
      currentPropertyLabels: { contacts: 'sensitive' },
    });

    expect(screen.getByText('critical')).toBeInTheDocument();
    expect(screen.queryByText('sensitive')).not.toBeInTheDocument();
    expect(screen.queryByText('public')).not.toBeInTheDocument();
  });
});
