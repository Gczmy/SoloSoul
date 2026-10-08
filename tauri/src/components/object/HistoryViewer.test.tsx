import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, waitFor, fireEvent, act, cleanup } from '@testing-library/react';
import { flattenProperties, HistoryViewer } from './HistoryViewer';
import * as invokeModule from '@tauri-apps/api/core';
import { setRequestSession } from '@/lib/sessionRequests';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

const mockInvoke = vi.mocked(invokeModule.invoke);

describe('flattenProperties', () => {
  it('does not use stale sensitivity from dynamic_group child items', () => {
    const props = {
      __fields: {
        contacts: { type: 'dynamic_group', sensitivityLevel: 'sensitive', name: '联系方式' },
      },
      contacts: [
        // 子项可能存有过期敏感度（例如模板同步前创建），不应被 flattenProperties 采用
        { id: 'c1', name: '手机', type: 'phone', value: '123', sensitivity: 'critical' },
        { id: 'c2', name: '邮箱', type: 'email', value: 'a@b.com', sensitivity: 'public' },
      ],
    };
    const result = flattenProperties(props as Record<string, unknown>);
    expect(result).toHaveLength(1);
    expect(result[0]).toMatchObject({
      kind: 'dynamicGroup',
      key: 'contacts',
      label: '联系方式',
      children: [
        { label: '手机', value: '123', type: 'phone' },
        { label: '邮箱', value: 'a@b.com', type: 'email' },
      ],
    });
    expect(result[0].sensitivity).toBeUndefined();
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
      { kind: 'field', key: 'name', value: 'Alice', label: undefined },
      { kind: 'field', key: 'age', value: '30', label: undefined },
    ]);
  });

  it('extracts snapshot-specific field name from __fields for regular fields', () => {
    const props = {
      __fields: {
        f1: { name: '旧字段名', type: 'multiline' },
      },
      f1: 'a',
    };
    const result = flattenProperties(props as Record<string, unknown>);
    expect(result).toEqual([
      { kind: 'field', key: 'f1', value: 'a', label: '旧字段名', type: 'multiline' },
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
        typeId="identity"
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
        typeId="identity"
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
        typeId="identity"
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
        typeId="identity"
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

  it('renders old field name from snapshot __fields after template rename sync', async () => {
    mockInvoke.mockImplementation(async (cmd) => {
      if (cmd === 'snapshot_list') {
        return [
          {
            id: 'snap-rename',
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
          // 旧快照：字段 ID 为 f1，当时字段名为 "1"，值为 "a"
          properties: {
            __fields: {
              f1: { name: '1', type: 'multiline' },
            },
            f1: 'a',
          },
          propertyLabels: {},
        };
      }
      return null;
    });

    render(
      <HistoryViewer
        objectId="obj-rename"
        objectName="Test Object"
        typeId="identity"
        onClose={() => {}}
        passwordVerify={async () => ({ ok: true, method: 'password' })}
        getFieldSensitivity={() => 'internal'}
        isFieldDeprecated={() => false}
        // 当前模板/对象已将字段名改为 "2"
        getFieldName={(k) => (k === 'f1' ? '2' : k)}
        fieldOrder={['f1']}
      />,
    );

    await waitFor(() => {
      expect(screen.getByText('1')).toBeInTheDocument();
    });

    // 历史记录应显示快照中的旧字段名 "1"，而不是当前模板的 "2"
    expect(screen.queryByText('2')).not.toBeInTheDocument();
    expect(screen.queryByText('a')).not.toBeInTheDocument();
    await act(async () => fireEvent.click(screen.getByText('••••••••')));
    expect(screen.getByText('a')).toBeInTheDocument();
  });

  it('renders dynamic_group sensitivity from snapshot propertyLabels even when __fields lacks sensitivityLevel', async () => {
    // 复现：对象创建时 __fields 不含 sensitivityLevel，但 propertyLabels 含动态字段组敏感度；
    // 修改模板敏感度后，旧快照仍应显示旧敏感度，不应被当前对象回调覆盖。
    mockInvoke.mockImplementation(async (cmd) => {
      if (cmd === 'snapshot_list') {
        return [
          {
            id: 'snap-pre-sync',
            timestamp: Date.now() - 1000,
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
            __fields: {
              // 创建时 __fields 没有 sensitivityLevel（仅同步后才写入）
              contacts: { name: '联系方式', type: 'dynamic_group' },
            },
            contacts: [{ id: 'c1', name: '手机', type: 'phone', value: '123' }],
          },
          propertyLabels: { contacts: 'critical' },
        };
      }
      return null;
    });

    render(
      <HistoryViewer
        objectId="obj-dg"
        objectName="Test Object"
        typeId="identity"
        onClose={() => {}}
        passwordVerify={async () => ({ ok: true, method: 'password' })}
        // 当前对象/模板已同步为 sensitive
        getFieldSensitivity={() => 'sensitive'}
        isFieldDeprecated={() => false}
        getFieldName={(k) => k}
        fieldOrder={['contacts']}
      />,
    );

    await waitFor(() => {
      expect(screen.getByText('手机')).toBeInTheDocument();
    });

    // 旧快照应显示 critical，而不是当前对象的 sensitive
    expect(screen.getByText('critical')).toBeInTheDocument();
    expect(screen.queryByText('sensitive')).not.toBeInTheDocument();
  });

  it('shows internal fields directly without a reveal control', async () => {
    mockInvoke.mockImplementation(async (cmd) => {
      if (cmd === 'snapshot_list') {
        return [
          {
            id: 'snap-internal',
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
            phone: '13800138000',
            __fields: { phone: { name: '手机', type: 'phone', sensitivityLevel: 'internal' } },
          },
          propertyLabels: { phone: 'internal' },
        };
      }
      return null;
    });

    render(
      <HistoryViewer
        objectId="obj-internal"
        objectName="Test Object"
        typeId="identity"
        onClose={() => {}}
        passwordVerify={async () => ({ ok: true, method: 'password' })}
        getFieldSensitivity={() => 'internal'}
        isFieldDeprecated={() => false}
        getFieldName={(k) => k}
        fieldOrder={['phone']}
      />,
    );

    await waitFor(() => {
      expect(screen.getByText('手机')).toBeInTheDocument();
    });

    expect(screen.getByText('13800138000')).toBeInTheDocument();
    expect(screen.queryByText('••••••••')).not.toBeInTheDocument();
  });

  it('renders sensitive field as a placeholder without protected text in DOM', async () => {
    mockInvoke.mockImplementation(async (cmd) => {
      if (cmd === 'snapshot_list') {
        return [
          {
            id: 'snap-sensitive',
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
            secret: 'hidden-value',
            __fields: {
              secret: { name: '密钥', type: 'text', sensitivityLevel: 'sensitive' },
            },
          },
          propertyLabels: { secret: 'sensitive' },
        };
      }
      return null;
    });

    render(
      <HistoryViewer
        objectId="obj-sensitive"
        objectName="Test Object"
        typeId="identity"
        onClose={() => {}}
        passwordVerify={async () => ({ ok: true, method: 'password' })}
        getFieldSensitivity={() => 'sensitive'}
        isFieldDeprecated={() => false}
        getFieldName={(k) => k}
        fieldOrder={['secret']}
      />,
    );

    await waitFor(() => {
      expect(screen.getByText('密钥')).toBeInTheDocument();
    });

    // 占位态 DOM、title 和可访问名称不含敏感值。
    expect(document.body.innerHTML).not.toContain('hidden-value');
    expect(screen.getByText('••••••••').tagName).toBe('BUTTON');
  });

  it('localizes __dynamic_group__ label even when snapshot __fields name is the raw key', async () => {
    mockInvoke.mockImplementation(async (cmd) => {
      if (cmd === 'snapshot_list') {
        return [
          {
            id: 'snap-dg-raw',
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
            __fields: {
              // 某些旧快照/模板中动态字段组名称存成了原始 key
              __dynamic_group__: {
                name: '__dynamic_group__',
                type: 'dynamic_group',
                sensitivityLevel: 'internal',
              },
            },
            __dynamic_group__: [
              { id: 'c1', name: '新字段2', type: 'text', value: '1' },
              { id: 'c2', name: '新字段', type: 'text', value: '1' },
            ],
          },
          propertyLabels: {},
        };
      }
      return null;
    });

    render(
      <HistoryViewer
        objectId="obj-dg-raw"
        objectName="Test Object"
        typeId="identity"
        onClose={() => {}}
        passwordVerify={async () => ({ ok: true, method: 'password' })}
        getFieldSensitivity={() => 'internal'}
        isFieldDeprecated={() => false}
        getFieldName={(k) => k}
        fieldOrder={['__dynamic_group__']}
      />,
    );

    await waitFor(() => {
      expect(screen.getByText('新字段2')).toBeInTheDocument();
    });

    // 应显示国际化后的“动态字段组”，而不是原始 __dynamic_group__
    expect(screen.queryByText('__dynamic_group__')).not.toBeInTheDocument();
    expect(screen.getByText('动态字段组')).toBeInTheDocument();
  });

  it('sensitive 字段揭示后显示自动隐藏倒计时，掩码时不显示', async () => {
    mockInvoke.mockImplementation(async (cmd) => {
      if (cmd === 'snapshot_list') {
        return [
          {
            id: 'snap-cd',
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
            secret: 'hidden-value',
            __fields: {
              secret: { name: '密钥', type: 'text', sensitivityLevel: 'sensitive' },
            },
          },
          propertyLabels: { secret: 'sensitive' },
        };
      }
      return null;
    });

    render(
      <HistoryViewer
        objectId="obj-countdown"
        objectName="Test Object"
        typeId="identity"
        onClose={() => {}}
        passwordVerify={async () => ({ ok: true, method: 'password' })}
        getFieldSensitivity={() => 'sensitive'}
        isFieldDeprecated={() => false}
        getFieldName={(k) => k}
        fieldOrder={['secret']}
      />,
    );

    await waitFor(() => {
      expect(screen.getByText('密钥')).toBeInTheDocument();
    });

    // 掩码态：无倒计时，原文未渲染。
    expect(screen.queryByTestId('history-reveal-countdown')).not.toBeInTheDocument();

    // 点击值揭示（sensitive 直接揭示，无需验证）→ 明文 + 倒计时出现
    await act(async () => fireEvent.click(screen.getByText('••••••••')));
    expect(screen.getByTestId('history-reveal-countdown')).toHaveTextContent('60s');
  });
});

describe('RF-105 concealed history values', () => {
  afterEach(() => vi.useRealTimers());
  it.each([false, true])(
    'critical verification ok=%s preserves masking, auditing and TTL',
    async (ok) => {
      mockInvoke.mockImplementation(async (cmd) => {
        if (cmd === 'snapshot_list')
          return [
            {
              id: 'protected',
              timestamp: Date.now(),
              triggeredBy: 'user_edit',
              diffSummary: 'diff_updated',
            },
          ];
        if (cmd === 'snapshot_get_data')
          return {
            properties: {
              key: 'CRITICAL_SECRET',
              __fields: { key: { name: 'Key', type: 'text', sensitivityLevel: 'critical' } },
            },
            propertyLabels: { key: 'critical' },
          };
        return null;
      });
      const verify = vi.fn().mockResolvedValue({ ok, method: 'password' });

      const { container } = render(
        <HistoryViewer
          objectId="protected"
          onClose={() => {}}
          passwordVerify={verify}
          objectName="Protected object"
          getFieldSensitivity={() => 'critical'}
          isFieldDeprecated={() => false}
          getFieldName={() => 'Key'}
        />,
      );
      const button = await screen.findByText('••••••••');
      expect(container.innerHTML).not.toContain('CRITICAL_SECRET');
      expect(button).toHaveAttribute('type', 'button');
      expect(button).toHaveAccessibleName();
      vi.useFakeTimers();
      await act(async () => {
        fireEvent.click(button, { detail: 0 });
      });
      expect(verify).toHaveBeenCalledTimes(1);
      if (!ok) {
        expect(container.innerHTML).not.toContain('CRITICAL_SECRET');
        expect(mockInvoke.mock.calls.filter(([cmd]) => cmd === 'log_write')).toHaveLength(0);
      } else {
        expect(screen.getByText('CRITICAL_SECRET')).toBeInTheDocument();
        expect(mockInvoke.mock.calls.filter(([cmd]) => cmd === 'log_write')).toHaveLength(1);
        await act(async () => {
          vi.advanceTimersByTime(60_001);
        });
        expect(container.innerHTML).not.toContain('CRITICAL_SECRET');
        expect(screen.getByText('••••••••')).toBeInTheDocument();
      }
    },
  );
});

describe('RF-107 snapshot protection and identity', () => {
  beforeEach(() => vi.clearAllMocks());
  const entry = (id: string) => ({ id, timestamp: 1, triggeredBy: 'user_edit', diffSummary: '' });
  const props = {
    objectId: 'object',
    objectName: 'Object',
    onClose: vi.fn(),
    passwordVerify: vi.fn(async () => ({ ok: true, method: 'password' as const })),
    getFieldSensitivity: vi.fn(() => 'public' as const),
    getFieldName: vi.fn(() => 'CURRENT_TEMPLATE_NAME'),
    isFieldDeprecated: vi.fn(() => true),
    fieldOrder: ['second', 'first'],
  };
  afterEach(() => {
    cleanup();
    setRequestSession(null);
    vi.useRealTimers();
  });
  function serve(data: Record<string, unknown>) {
    mockInvoke.mockImplementation(async (cmd) => {
      if (cmd === 'snapshot_list') return [entry('one')];
      if (cmd === 'snapshot_get_data') return data;
      return null;
    });
  }

  it.each(['public', 'internal', 'sensitive', 'critical', 'unknown'])(
    'snapshot %s follows shared policy without consulting current template',
    async (level) => {
      serve({
        properties: {
          first: 'ORIGINAL_VALUE',
          __fields: { first: { name: 'Historical name', sensitivityLevel: 'public' } },
        },
        propertyLabels: { first: level },
      });
      const verify = vi.fn().mockResolvedValue({ ok: false, method: 'password' });
      const { container, rerender } = render(<HistoryViewer {...props} passwordVerify={verify} />);
      await screen.findByText('Historical name');
      expect(container.innerHTML.includes('ORIGINAL_VALUE')).toBe(
        level === 'public' || level === 'internal',
      );
      // 模板删除/改名/重排/更改等级不影响已加载快照。
      rerender(
        <HistoryViewer
          {...props}
          passwordVerify={verify}
          getFieldName={() => 'RENAMED'}
          getFieldSensitivity={() => 'critical'}
          fieldOrder={[]}
        />,
      );
      expect(screen.getByText('Historical name')).toBeInTheDocument();
      expect(container.innerHTML).not.toContain('RENAMED');
      if (level !== 'public' && level !== 'internal') {
        await act(async () => fireEvent.click(screen.getByText('••••••••')));
        expect(container.innerHTML.includes('ORIGINAL_VALUE')).toBe(level !== 'critical');
        expect(verify).toHaveBeenCalledTimes(level === 'critical' ? 1 : 0);
      }
    },
  );

  it('keeps historical definition order and uses key/internal when snapshot metadata is absent', async () => {
    serve({
      properties: {
        second: 'TWO',
        first: 'ONE',
        __fields: {
          first: { name: 'Old first', sensitivityLevel: 'public' },
          second: { name: 'Old second', sensitivityLevel: 'public' },
        },
      },
    });
    const { container, unmount } = render(<HistoryViewer {...props} />);
    await screen.findByText('Old first');
    expect(container.textContent!.indexOf('Old first')).toBeLessThan(
      container.textContent!.indexOf('Old second'),
    );
    unmount();
    serve({ properties: { legacy_key: 'LEGACY_SECRET' } });
    render(<HistoryViewer {...props} />);
    await screen.findByText('legacy_key');
    expect(document.body.innerHTML).not.toContain('LEGACY_SECRET');
    expect(screen.queryByText('CURRENT_TEMPLATE_NAME')).toBeNull();
  });

  it('inherits parent protection and independently protects critical children of public groups', async () => {
    serve({
      properties: {
        __fields: {
          parent: { type: 'dynamic_group', sensitivityLevel: 'sensitive' },
          mixed: { type: 'dynamic_group', sensitivityLevel: 'public' },
        },
        parent: [
          { id: 'p', name: 'Inherited child', value: 'PARENT_SECRET', sensitivityLevel: 'public' },
        ],
        mixed: [
          { id: 'open', name: 'Open child', value: 'OPEN_VALUE', sensitivityLevel: 'public' },
          {
            id: 'locked',
            name: 'Locked child',
            value: 'CHILD_SECRET',
            sensitivityLevel: 'critical',
          },
        ],
      },
    });
    const verify = vi
      .fn()
      .mockResolvedValueOnce({ ok: false, method: 'pin' })
      .mockResolvedValue({ ok: true, method: 'pin' });
    const { container } = render(<HistoryViewer {...props} passwordVerify={verify} />);
    await screen.findByText('Open child');
    expect(screen.getByText('OPEN_VALUE')).toBeInTheDocument();
    expect(container.innerHTML).not.toContain('PARENT_SECRET');
    expect(container.innerHTML).not.toContain('CHILD_SECRET');
    await act(async () => fireEvent.click(screen.getAllByText('••••••••')[1]));
    expect(container.innerHTML).not.toContain('CHILD_SECRET');
    await act(async () => fireEvent.click(screen.getAllByText('••••••••')[1]));
    expect(screen.getByText('CHILD_SECRET')).toBeInTheDocument();
    expect(container.innerHTML).not.toContain('PARENT_SECRET');
    expect(mockInvoke.mock.calls.filter(([cmd]) => cmd === 'log_write')).toHaveLength(1);
  });

  it.each(['snapshot', 'object', 'session', 'unmount'] as const)(
    '%s change rejects late critical verification and auditing',
    async (change) => {
      setRequestSession('a');
      let resolve!: (result: { ok: boolean; method: 'password' }) => void;
      const verify = vi.fn(
        () =>
          new Promise<{ ok: boolean; method: 'password' }>((finish) => {
            resolve = finish;
          }),
      );
      mockInvoke.mockImplementation(async (cmd, args) => {
        if (cmd === 'snapshot_list') return [entry('one'), entry('two')];
        if (cmd === 'snapshot_get_data')
          return {
            properties: { secret: `SECRET_${(args as { snapshotId: string }).snapshotId}` },
            propertyLabels: { secret: 'critical' },
          };
        return null;
      });
      const { rerender, unmount } = render(<HistoryViewer {...props} passwordVerify={verify} />);
      fireEvent.click(await screen.findByText('••••••••'));
      if (change === 'snapshot') {
        vi.useFakeTimers();
        fireEvent.click(screen.getByTitle('Previous'));
        await act(async () => vi.advanceTimersByTime(151));
        vi.useRealTimers();
      } else if (change === 'object')
        rerender(<HistoryViewer {...props} objectId="other" passwordVerify={verify} />);
      else if (change === 'session')
        act(() => {
          setRequestSession(null);
          setRequestSession('a');
        });
      else unmount();
      await act(async () => resolve({ ok: true, method: 'password' }));
      expect(document.body.innerHTML).not.toContain('SECRET_');
      expect(mockInvoke.mock.calls.filter(([cmd]) => cmd === 'log_write')).toHaveLength(0);
    },
  );

  it('prefetches neighbors once and reuses data without preserving reveal authorization', async () => {
    mockInvoke.mockImplementation(async (cmd, args) => {
      if (cmd === 'snapshot_list') return [entry('one'), entry('two'), entry('three')];
      if (cmd === 'snapshot_get_data')
        return {
          properties: { field: `VALUE_${(args as { snapshotId: string }).snapshotId}` },
          propertyLabels: { field: 'internal' },
        };
      return null;
    });
    const { unmount } = render(<HistoryViewer {...props} />);
    await screen.findByText('VALUE_one');
    const loads = () => mockInvoke.mock.calls.filter(([cmd]) => cmd === 'snapshot_get_data');
    await waitFor(() => expect(loads()).toHaveLength(2));
    fireEvent.click(screen.getByTitle('Previous'));
    expect(screen.getByText('VALUE_two')).toBeInTheDocument();
    await waitFor(() => expect(loads()).toHaveLength(3));
    fireEvent.click(screen.getByTitle('Next'));
    expect(screen.getByText('VALUE_one')).toBeInTheDocument();
    expect(loads()).toHaveLength(3);
    unmount();
    render(<HistoryViewer {...props} />);
    await screen.findByText('VALUE_one');
    await waitFor(() => expect(loads()).toHaveLength(5));
  });

  it('failed neighboring prefetch is retryable on navigation', async () => {
    let attempts = 0;
    mockInvoke.mockImplementation(async (cmd, args) => {
      if (cmd === 'snapshot_list') return [entry('one'), entry('two')];
      if (cmd === 'snapshot_get_data') {
        const id = (args as { snapshotId: string }).snapshotId;
        if (id === 'two' && ++attempts === 1) throw new Error('temporary failure');
        return { properties: { field: `VALUE_${id}` }, propertyLabels: { field: 'internal' } };
      }
      return null;
    });
    render(<HistoryViewer {...props} />);
    await screen.findByText('VALUE_one');
    await waitFor(() => expect(attempts).toBe(1));
    fireEvent.click(screen.getByTitle('Previous'));
    await screen.findByText('VALUE_two');
    expect(attempts).toBe(2);
  });

  it('returning to a revealed snapshot requires revealing again', async () => {
    mockInvoke.mockImplementation(async (cmd, args) => {
      if (cmd === 'snapshot_list') return [entry('one'), entry('two')];
      if (cmd === 'snapshot_get_data')
        return {
          properties: {
            field:
              (args as { snapshotId: string }).snapshotId === 'one'
                ? 'FIRST_SECRET'
                : 'SECOND_SECRET',
          },
          propertyLabels: { field: 'sensitive' },
        };
      return null;
    });
    render(<HistoryViewer {...props} />);
    const button = await screen.findByText('••••••••');
    await act(async () => fireEvent.click(button));
    expect(screen.getByText('FIRST_SECRET')).toBeInTheDocument();
    vi.useFakeTimers();
    fireEvent.click(screen.getByTitle('Previous'));
    expect(screen.queryByText('FIRST_SECRET')).toBeNull();
    await act(async () => vi.advanceTimersByTime(151));
    expect(screen.queryByText('SECOND_SECRET')).toBeNull();
    fireEvent.click(screen.getByTitle('Next'));
    await act(async () => vi.advanceTimersByTime(151));
    expect(screen.queryByText('FIRST_SECRET')).toBeNull();
    expect(screen.getByText('••••••••')).toBeInTheDocument();
  });

  it('rejects late snapshot data after navigation', async () => {
    let finish!: (data: unknown) => void;
    mockInvoke.mockImplementation(async (cmd, args) => {
      if (cmd === 'snapshot_list') return [entry('one'), entry('two')];
      if (cmd === 'snapshot_get_data') {
        if ((args as { snapshotId: string }).snapshotId === 'one')
          return new Promise((resolve) => {
            finish = resolve;
          });
        return {
          name: 'SECOND_SNAPSHOT',
          properties: { field: 'CURRENT_PUBLIC' },
          propertyLabels: { field: 'public' },
        };
      }
      return null;
    });
    render(<HistoryViewer {...props} />);
    await screen.findByTitle('Previous');
    await waitFor(() => expect(finish).toBeDefined());
    vi.useFakeTimers();
    fireEvent.click(screen.getByTitle('Previous'));
    await act(async () => vi.advanceTimersByTime(151));
    await act(async () =>
      finish({
        name: 'STALE_SNAPSHOT',
        properties: { field: 'STALE_PUBLIC' },
        propertyLabels: { field: 'public' },
      }),
    );
    expect(screen.getByText('SECOND_SNAPSHOT')).toBeInTheDocument();
    expect(screen.getByText('CURRENT_PUBLIC')).toBeInTheDocument();
    expect(document.body.innerHTML).not.toContain('STALE_');
  });
});
