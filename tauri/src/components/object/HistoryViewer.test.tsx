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
  it('does not use stale sensitivity from dynamic_group child items', () => {
    const props = {
      __fields: {
        contacts: { type: 'dynamic_group', sensitivityLevel: 'sensitive' },
      },
      contacts: [
        // 子项可能存有过期敏感度（例如模板同步前创建），不应被 flattenProperties 采用
        { id: 'c1', name: '手机', type: 'phone', value: '123', sensitivity: 'critical' },
        { id: 'c2', name: '邮箱', type: 'email', value: 'a@b.com', sensitivity: 'public' },
      ],
    };
    const result = flattenProperties(props as Record<string, unknown>);
    expect(result).toHaveLength(2);
    expect(result[0]).toMatchObject({ key: 'contacts', label: '手机' });
    expect(result[1]).toMatchObject({ key: 'contacts', label: '邮箱' });
    expect(result[0].sensitivity).toBeUndefined();
    expect(result[1].sensitivity).toBeUndefined();
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
  it('renders dynamic_group child sensitivity from snapshot __fields instead of current template', async () => {
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
            // 快照保存的是模板同步后的新敏感度
            __fields: { contacts: { type: 'dynamic_group', sensitivityLevel: 'critical' } },
            contacts: [
              // 子项仍保留旧敏感度，不应被采用
              { id: 'c1', name: '手机', type: 'phone', value: '123', sensitivity: 'sensitive' },
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
        getFieldSensitivity={() => 'public'}
        isFieldDeprecated={() => false}
        getFieldName={(k) => k}
        fieldOrder={['contacts']}
      />,
    );

    await waitFor(() => {
      expect(screen.getByText('手机')).toBeInTheDocument();
    });

    // 动态字段组子字段应使用快照 __fields 中的 critical，而不是子项的 sensitive 或外部回调的 public
    expect(screen.getByText('critical')).toBeInTheDocument();
    expect(screen.queryByText('sensitive')).not.toBeInTheDocument();
    expect(screen.queryByText('public')).not.toBeInTheDocument();
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

  it('falls back to snapshot __fields sensitivity when dynamic_group child lacks sensitivity', async () => {
    mockInvoke.mockImplementation(async (cmd) => {
      if (cmd === 'snapshot_list') {
        return [
          {
            id: 'snap-3',
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
            __fields: { contacts: { type: 'dynamic_group', sensitivityLevel: 'critical' } },
            contacts: [
              // 子项未保存 sensitivity，应使用快照 __fields 中的敏感度
              { id: 'c1', name: '手机', type: 'phone', value: '123' },
            ],
          },
          propertyLabels: {},
        };
      }
      return null;
    });

    render(
      <HistoryViewer
        objectId="obj-3"
        objectName="Test Object"
        collectionType="identity"
        onClose={() => {}}
        passwordVerify={async () => ({ ok: true, method: 'password' })}
        // 外部回调返回 public，用于验证不会被它覆盖
        getFieldSensitivity={() => 'public'}
        isFieldDeprecated={() => false}
        getFieldName={(k) => k}
        fieldOrder={['contacts']}
      />,
    );

    await waitFor(() => {
      expect(screen.getByText('手机')).toBeInTheDocument();
    });

    // 应使用快照 __fields 中的 critical，而不是外部 getFieldSensitivity 的 public
    expect(screen.getByText('critical')).toBeInTheDocument();
    expect(screen.queryByText('public')).not.toBeInTheDocument();
  });

  it('ignores stale child sensitivity after template sync and uses updated __fields', async () => {
    mockInvoke.mockImplementation(async (cmd) => {
      if (cmd === 'snapshot_list') {
        return [
          {
            id: 'snap-4',
            timestamp: Date.now(),
            triggeredBy: 'template_sync',
            diffSummary: 'diff_template_sync',
          },
        ];
      }
      if (cmd === 'snapshot_get_data') {
        return {
          name: 'Test Object',
          tags: [],
          properties: {
            // 模板同步后父字段敏感度从 critical 更新为 sensitive
            __fields: { contacts: { type: 'dynamic_group', sensitivityLevel: 'sensitive' } },
            // 子项仍保留同步前的旧敏感度 critical
            contacts: [
              { id: 'c1', name: '手机', type: 'phone', value: '123', sensitivity: 'critical' },
            ],
          },
          propertyLabels: { contacts: 'sensitive' },
        };
      }
      return null;
    });

    render(
      <HistoryViewer
        objectId="obj-4"
        objectName="Test Object"
        collectionType="identity"
        onClose={() => {}}
        passwordVerify={async () => ({ ok: true, method: 'password' })}
        getFieldSensitivity={() => 'public'}
        isFieldDeprecated={() => false}
        getFieldName={(k) => k}
        fieldOrder={['contacts']}
      />,
    );

    await waitFor(() => {
      expect(screen.getByText('手机')).toBeInTheDocument();
    });

    // 应使用快照 propertyLabels / __fields 中同步后的 sensitive，而不是子项里的旧 critical
    expect(screen.getByText('sensitive')).toBeInTheDocument();
    expect(screen.queryByText('critical')).not.toBeInTheDocument();
    expect(screen.queryByText('public')).not.toBeInTheDocument();
  });
});
