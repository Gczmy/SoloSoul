import { describe, it, expect } from 'vitest';
import { flattenPropertyEntries } from './propertyFlatten';

describe('flattenPropertyEntries (P024 shared core)', () => {
  it('returns empty array for undefined/null props', () => {
    expect(flattenPropertyEntries(undefined)).toEqual([]);
  });

  it('skips __ prefix keys by default (keepMetaKeys=false)', () => {
    const props = {
      __internal: 'x',
      name: '张三',
      age: 0, // 数字 0 是有效值
      empty: '',
      tags: [],
      note: null,
    };
    const result = flattenPropertyEntries(props);
    expect(result).toEqual([
      { kind: 'field', key: 'name', value: '张三' },
      { kind: 'field', key: 'age', value: '0' },
    ]);
  });

  it('keepMetaKeys=true keeps __ keys defined in fieldDefs', () => {
    const props = {
      __fields: { __dynamic_group__: { type: 'dynamic_group', name: '动态组' } },
      __dynamic_group__: [{ name: 'a', value: '1' }],
    };
    const result = flattenPropertyEntries(props, undefined, undefined, {
      keepMetaKeys: true,
      flattenDynamicGroups: false,
    });
    expect(result).toEqual([
      {
        kind: 'dynamicGroup',
        key: '__dynamic_group__',
        label: '动态组',
        type: 'dynamic_group',
        children: [{ label: 'a', value: '1', type: undefined }],
      },
    ]);
  });

  it('flattenDynamicGroups=true expands children into independent entries with fieldId', () => {
    const props = {
      __fields: { experiences: { type: 'dynamic_group' } },
      experiences: [
        { id: 'exp-1', name: '第一段', value: 'A' },
        { id: 'exp-2', name: '第二段', value: ['x', 'y'] },
        { name: '无 id 子项', value: 5 },
      ],
    };
    const result = flattenPropertyEntries(props);
    expect(result).toEqual([
      { kind: 'field', key: 'experiences', label: '第一段', value: 'A', fieldId: 'experiences.exp-1' },
      { kind: 'field', key: 'experiences', label: '第二段', value: 'x, y', fieldId: 'experiences.exp-2' },
      { kind: 'field', key: 'experiences', label: '无 id 子项', value: '5', fieldId: 'experiences.无 id 子项' },
    ]);
  });

  it('flattenDynamicGroups=false returns grouped structure with children types', () => {
    const props = {
      __fields: { contacts: { type: 'dynamic_group', name: '联系方式' } },
      contacts: [
        { id: 'c1', name: '手机', type: 'phone', value: '123' },
        { id: 'c2', name: '邮箱', type: 'email', value: 'a@b.com' },
      ],
    };
    const result = flattenPropertyEntries(props, undefined, undefined, {
      flattenDynamicGroups: false,
    });
    expect(result).toEqual([
      {
        kind: 'dynamicGroup',
        key: 'contacts',
        label: '联系方式',
        type: 'dynamic_group',
        children: [
          { label: '手机', value: '123', type: 'phone' },
          { label: '邮箱', value: 'a@b.com', type: 'email' },
        ],
      },
    ]);
  });

  it('skips empty dynamic_group in grouped mode', () => {
    const props = {
      __fields: { contacts: { type: 'dynamic_group' } },
      contacts: [],
    };
    expect(flattenPropertyEntries(props, undefined, undefined, { flattenDynamicGroups: false })).toEqual([]);
  });

  it('fieldDefs parameter takes priority over props.__fields', () => {
    const props = {
      __fields: { group: { type: 'text' } },
      group: [{ name: '子项', value: 'v' }],
    };
    const explicitDefs = { group: { type: 'dynamic_group' } };
    const result = flattenPropertyEntries(props, undefined, explicitDefs);
    expect(result).toEqual([
      { kind: 'field', key: 'group', label: '子项', value: 'v', fieldId: 'group.子项' },
    ]);
  });

  it('sorts by fieldOrder with lexicographic fallback', () => {
    const props = { b: '2', a: '1', c: '3' };
    const result = flattenPropertyEntries(props, ['c', 'a']);
    expect(result.map((r) => r.key)).toEqual(['c', 'a', 'b']);
  });

  it('P024-R1: plain fields do NOT inject __fields snapshot label by default', () => {
    // 旧 objectDetailUtils/WorkspaceObjectCard 语义：普通字段不带 label，
    // 消费端 label 优先回退当前模板名；若注入过期 __fields 快照名，模板字段
    // 重命名而快照未同步时会显示旧名（核查轮次 13 发现的零行为变化破坏）。
    const props = {
      __fields: { email: { type: 'text', name: '旧邮箱名' } },
      email: 'a@b.com',
    };
    expect(flattenPropertyEntries(props)).toEqual([
      { kind: 'field', key: 'email', value: 'a@b.com', type: 'text' },
    ]);
  });

  it('P024-R1: injectFieldLabels=true keeps snapshot names (HistoryViewer semantics)', () => {
    const props = {
      __fields: { email: { type: 'text', name: '历史邮箱名' } },
      email: 'a@b.com',
    };
    expect(flattenPropertyEntries(props, undefined, undefined, { injectFieldLabels: true })).toEqual(
      [{ kind: 'field', key: 'email', value: 'a@b.com', label: '历史邮箱名', type: 'text' }],
    );
  });
});
