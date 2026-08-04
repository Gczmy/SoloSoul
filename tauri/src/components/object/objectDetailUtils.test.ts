import { describe, it, expect } from 'vitest';
import type { useTranslation } from 'react-i18next';
import { flattenProperties, buildDetailGuidePages } from './objectDetailUtils';

type T = ReturnType<typeof useTranslation>['t'];
const mockT = ((key: string) => key) as unknown as T;

describe('flattenProperties', () => {
  it('返回空数组当 props 为 undefined/null', () => {
    expect(flattenProperties(undefined)).toEqual([]);
  });

  it('过滤内部 __ 前缀字段与 null/undefined/空串/空数组', () => {
    const props = {
      __internal: 'x',
      name: '张三',
      age: 0, // 数字 0 是有效值，不应被过滤
      empty: '',
      tags: [],
      note: null,
    };
    expect(flattenProperties(props)).toEqual([
      { key: 'name', value: '张三' },
      { key: 'age', value: '0' },
    ]);
  });

  it('支持字符串/数字/布尔/数组值', () => {
    const props = { name: '张三', age: 30, active: true, tags: ['a', 'b'] };
    expect(flattenProperties(props)).toEqual([
      { key: 'name', value: '张三' },
      { key: 'age', value: '30' },
      { key: 'active', value: 'true' },
      { key: 'tags', value: 'a, b' },
    ]);
  });

  it('dynamic_group 子字段展开为独立条目（含 label + fieldId）', () => {
    const props = {
      __fields: { experiences: { type: 'dynamic_group' } },
      experiences: [
        { id: 'exp-1', name: '第一段', value: 'A' },
        { id: 'exp-2', name: '第二段', value: ['x', 'y'] },
        { name: '无 id 子项', value: 5 },
      ],
    };
    expect(flattenProperties(props)).toEqual([
      { key: 'experiences', label: '第一段', value: 'A', fieldId: 'experiences.exp-1' },
      { key: 'experiences', label: '第二段', value: 'x, y', fieldId: 'experiences.exp-2' },
      { key: 'experiences', label: '无 id 子项', value: '5', fieldId: 'experiences.无 id 子项' },
    ]);
  });

  it('fieldDefs 参数优先于 props.__fields', () => {
    const props = {
      __fields: { group: { type: 'text' } },
      group: [{ name: '子项', value: 'v' }],
    };
    const explicitDefs = { group: { type: 'dynamic_group' } };
    const result = flattenProperties(props, undefined, explicitDefs);
    expect(result).toHaveLength(1);
    expect(result[0]).toMatchObject({ key: 'group', label: '子项', value: 'v' });
  });

  it('按 fieldOrder 排序，未列出字段按 key 字典序兜底', () => {
    const props = { b: '2', a: '1', c: '3' };
    const result = flattenProperties(props, ['c', 'a']);
    expect(result.map((r) => r.key)).toEqual(['c', 'a', 'b']);
  });
});

describe('buildDetailGuidePages', () => {
  it('移动端返回 3 步 + 2 帮助链接的指南', () => {
    const pages = buildDetailGuidePages(mockT, true);
    expect(pages).toHaveLength(1);
    expect(pages[0].steps).toHaveLength(3);
    expect(pages[0].helpLinks).toHaveLength(2);
  });

  it('桌面端返回 2 步 + 1 帮助链接的拖拽上传指南', () => {
    const pages = buildDetailGuidePages(mockT, false);
    expect(pages).toHaveLength(1);
    expect(pages[0].steps).toHaveLength(2);
    expect(pages[0].helpLinks).toHaveLength(1);
  });
});
