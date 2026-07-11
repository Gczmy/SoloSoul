import { describe, it, expect, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { flattenProperties, HistoryViewer } from './HistoryViewer';
import type { SensitivityLevel } from '@/components/ui/SensitivityBadge';
import * as invokeModule from '@tauri-apps/api/core';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

const mockInvoke = vi.mocked(invokeModule.invoke);

describe('flattenProperties', () => {
  it('extracts sensitivity from dynamic_group child items', () => {
    const props = {
      __fields: {
        contacts: { type: 'dynamic_group' },
      },
      contacts: [
        { id: 'c1', name: '手机', type: 'phone', value: '123', sensitivity: 'critical' },
        { id: 'c2', name: '邮箱', type: 'email', value: 'a@b.com', sensitivity: 'sensitive' },
      ],
    };
    const result = flattenProperties(props as Record<string, unknown>);
    expect(result).toHaveLength(2);
    expect(result[0]).toMatchObject({ key: 'contacts', label: '手机', sensitivity: 'critical' });
    expect(result[1]).toMatchObject({ key: 'contacts', label: '邮箱', sensitivity: 'sensitive' });
  });

  it('returns empty array for empty dynamic_group', () => {
    const props = {
      __fields: { contacts: { type: 'dynamic_group' } },
      contacts: [],
    };
    expect(flattenProperties(props as Record<string, unknown>)).toEqual([]);
  });

  it('keeps regular fields without sensitivity', () => {
    const props = {
      name: 'Alice',
      age: 30,
    };
    const result = flattenProperties(props as Record<string, unknown>);
    expect(result).toEqual([
      { key: 'name', value: 'Alice' },
      { key: 'age', value: '30' },
    ]);
  });
});

describe('HistoryViewer', () => {
  it('renders dynamic_group child sensitivity from snapshot instead of current template', async () => {
    mockInvoke.mockImplementation(async (cmd) => {
      if (cmd === 'snapshot_list') {
        return [
          {
            id: 'snap-1',
            timestamp: Date.now(),
            triggeredBy: 'user_edit',
            diffSummary: 'diff_updated',
          },
        ];
      }
      if (cmd === 'snapshot_get_data') {
        return {
          name: 'Test Object',
          tags: [],
          properties: {
            __fields: { contacts: { type: 'dynamic_group' } },
            contacts: [
              { id: 'c1', name: '手机', type: 'phone', value: '123', sensitivity: 'critical' },
            ],
          },
          propertyLabels: {},
        };
      }
      return null;
    });

    render(
      <HistoryViewer
        objectId="obj-1"
        objectName="Test Object"
        collectionType="identity"
        onClose={() => {}}
        passwordVerify={async () => ({ ok: true, method: 'password' })}
        getFieldSensitivity={() => 'sensitive'}
        isFieldDeprecated={() => false}
        getFieldName={(k) => k}
        fieldOrder={['contacts']}
      />,
    );

    await waitFor(() => {
      expect(screen.getByText('手机')).toBeInTheDocument();
    });

    // 动态字段组子字段应使用快照中的 critical，而不是 getFieldSensitivity 返回的 sensitive
    expect(screen.getByText('critical')).toBeInTheDocument();
    expect(screen.queryByText('sensitive')).not.toBeInTheDocument();
  });

  it('renders regular field sensitivity from snapshot propertyLabels', async () => {
    mockInvoke.mockImplementation(async (cmd) => {
      if (cmd === 'snapshot_list') {
        return [
          {
            id: 'snap-2',
            timestamp: Date.now(),
            triggeredBy: 'user_edit',
            diffSummary: 'diff_updated',
          },
        ];
      }
      if (cmd === 'snapshot_get_data') {
        return {
          name: 'Test Object',
          tags: [],
          properties: {
            fullName: 'Alice',
          },
          propertyLabels: { fullName: 'critical' },
        };
      }
      return null;
    });

    render(
      <HistoryViewer
        objectId="obj-2"
        objectName="Test Object"
        collectionType="identity"
        onClose={() => {}}
        passwordVerify={async () => ({ ok: true, method: 'password' })}
        getFieldSensitivity={() => 'sensitive'}
        isFieldDeprecated={() => false}
        getFieldName={(k) => k}
      />,
    );

    await waitFor(() => {
      expect(screen.getByText('fullName')).toBeInTheDocument();
    });

    expect(screen.getByText('critical')).toBeInTheDocument();
    expect(screen.queryByText('sensitive')).not.toBeInTheDocument();
  });
});
