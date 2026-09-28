import { describe, expect, it } from 'vitest';
import type { IpcCommands, JsonValue } from './generated/ipcContracts';
import { resolveFieldSensitivity } from './fieldSensitivity';
import {
  snapshotFieldRecord,
  toJsonObject,
  toObjectDataView,
  toObjectSummaryView,
  toTrashItemView,
} from './objectViewModel';

type WireObject = NonNullable<IpcCommands['object_get']['result']>;
type WireSummary = IpcCommands['object_list']['result'][number];
type WireTrash = IpcCommands['object_trash_list']['result'][number];

function object(overrides: Partial<WireObject> = {}): WireObject {
  return {
    id: 'synthetic-object',
    accountId: 'synthetic-account',
    name: '合成对象',
    typeId: 'identity',
    properties: {},
    sensitivityLevel: 'internal',
    templateId: null,
    templateType: null,
    propertyLabels: null,
    createdAt: '2026-09-28T10:00:00Z',
    updatedAt: '2026-09-28T10:00:00Z',
    deletedAt: null,
    contractTypeId: null,
    templateHash: null,
    ignoredTemplateHash: null,
    ...overrides,
  };
}

function summary(overrides: Partial<WireSummary> = {}): WireSummary {
  return {
    id: 'synthetic-object',
    name: '合成摘要',
    typeId: 'identity',
    sectionType: 'identity',
    sensitivityLevel: 'internal',
    createdAt: '2026-09-28T10:00:00Z',
    updatedAt: '2026-09-28T10:00:00Z',
    isDeleted: false,
    templateId: null,
    templateType: null,
    iconName: 'document',
    properties: {},
    tags: [],
    hasAttachments: false,
    ...overrides,
  };
}

describe('RF302 object ViewModel 边界', () => {
  it('写入省略对象 undefined，数组空位和非有限数转 null，保留其他 JSON 值', () => {
    const entries: unknown[] = new Array<unknown>(3);
    entries[1] = undefined;
    entries[2] = false;
    Object.freeze(entries);
    const nested = Object.freeze({ omitted: undefined, keep: '你好 🌍' });
    const source = Object.freeze({
      omitted: undefined,
      entries,
      nested,
      numbers: Object.freeze([NaN, Infinity, -Infinity, 0, 1.5]),
      zero: 0,
      flag: false,
      empty: '',
      nil: null,
    });

    const result = toJsonObject(source);

    expect(result).toStrictEqual({
      entries: [null, null, false],
      nested: { keep: '你好 🌍' },
      numbers: [null, null, null, 0, 1.5],
      zero: 0,
      flag: false,
      empty: '',
      nil: null,
    });
    expect(Object.hasOwn(source, 'omitted')).toBe(true);
    expect(Object.hasOwn(nested, 'omitted')).toBe(true);
    expect(Object.hasOwn(entries, 0)).toBe(false);
    expect(Object.hasOwn(entries, 1)).toBe(true);
    expect(entries[1]).toBeUndefined();
    expect(source.numbers).toEqual([NaN, Infinity, -Infinity, 0, 1.5]);
    expect(result.entries).not.toBe(entries);
    expect(result.nested).not.toBe(nested);
  });

  it('允许无循环的共享引用，并为每处写入复制数据而不修改编辑对象', () => {
    const child = Object.freeze({ value: 'original' });
    const shared = Object.freeze({ child, values: Object.freeze([child]) });
    const source = Object.freeze({ first: shared, second: shared });

    const result = toJsonObject(source);

    expect(result).toStrictEqual({
      first: { child: { value: 'original' }, values: [{ value: 'original' }] },
      second: { child: { value: 'original' }, values: [{ value: 'original' }] },
    });
    expect(result.first).not.toBe(shared);
    expect(result.second).not.toBe(shared);
    expect(result.first).not.toBe(result.second);
    expect(source.first).toBe(source.second);
    expect(source.first.child).toBe(child);
    expect(source.first.values[0]).toBe(child);
  });

  it('保留 null 原型对象的 __proto__ 自有键，不将其应用为输出原型', () => {
    const source: Record<string, unknown> = {
      ['__proto__']: { marker: 'own-value' },
      constructor: 'ordinary-field',
      omitted: undefined,
    };
    Object.setPrototypeOf(source, null);
    Object.freeze(source);

    const result = toJsonObject(source);

    expect(Object.getPrototypeOf(source)).toBeNull();
    expect(Object.getPrototypeOf(result)).toBe(Object.prototype);
    expect(Object.hasOwn(result, '__proto__')).toBe(true);
    expect(result['__proto__']).toStrictEqual({ marker: 'own-value' });
    expect(result['__proto__']).not.toBe(source['__proto__']);
    expect(result.constructor).toBe('ordinary-field');
    expect(Object.hasOwn(result, 'omitted')).toBe(false);
    expect(Object.hasOwn(source, 'omitted')).toBe(true);
  });

  it('拒绝对象与数组循环，失败后仍能转换正常对象', () => {
    const cyclicObject: Record<string, unknown> = {};
    cyclicObject.self = cyclicObject;
    const cyclicArray: unknown[] = [];
    cyclicArray.push(cyclicArray);

    expect(() => toJsonObject(cyclicObject)).toThrow(TypeError);
    expect(() => toJsonObject({ nested: cyclicArray })).toThrow(TypeError);
    expect(cyclicObject.self).toBe(cyclicObject);
    expect(cyclicArray[0]).toBe(cyclicArray);
    expect(toJsonObject({ valid: true })).toStrictEqual({ valid: true });
  });

  it('拒绝 BigInt、函数、Symbol 和非 JSON 对象，而非静默伪装成普通记录', () => {
    class NonJsonRecord {
      value = 'kept';
    }
    const unsupported: unknown[] = [
      1n,
      () => 'value',
      Symbol('synthetic'),
      new Date('2026-09-28T10:00:00Z'),
      new Map([['field', 'value']]),
      new NonJsonRecord(),
    ];

    for (const value of unsupported) {
      expect(() => toJsonObject({ nested: { value } })).toThrow(TypeError);
      expect(() => toJsonObject({ entries: [value] })).toThrow(TypeError);
    }
  });

  it('任意 JSON 快照仅对 record 返回字段，不把数组或标量当作字段表', () => {
    const nonRecords: JsonValue[] = [null, false, 0, 'snapshot', [], [{ field: 'value' }]];
    for (const value of nonRecords) {
      expect(snapshotFieldRecord(value)).toBeNull();
    }
    const record: JsonValue = {
      text: '合成快照',
      nil: null,
      nested: [{ enabled: false, count: 0 }],
    };
    expect(snapshotFieldRecord(record)).toStrictEqual(record);
    expect(snapshotFieldRecord({})).toStrictEqual({});
  });

  it('详情只把 nullable 字段转可选值，保留未知模板类型、空串及 tags 的省略语义', () => {
    const wire = object({ properties: null });
    const view = toObjectDataView(wire);

    expect(view.properties).toStrictEqual({});
    expect(view.propertyLabels).toBeUndefined();
    expect(view.templateId).toBeUndefined();
    expect(view.templateType).toBeUndefined();
    expect(view.templateHash).toBeUndefined();
    expect(view.ignoredTemplateHash).toBeUndefined();
    expect(view.deletedAt).toBeUndefined();
    expect(view.contractTypeId).toBeUndefined();
    expect(Object.hasOwn(view, 'tags')).toBe(false);
    expect(wire.templateId).toBeNull();
    expect(wire.propertyLabels).toBeNull();
    expect(wire.properties).toBeNull();
    expect(wire.deletedAt).toBeNull();

    const populated = object({
      templateId: '',
      templateType: 'future-template-kind',
      templateHash: '',
      ignoredTemplateHash: '',
      deletedAt: '',
      contractTypeId: '',
      tags: ['tag', '合成'],
      propertyLabels: {},
      properties: { enabled: false, count: 0, nested: [null, 'value'] },
    });
    expect(toObjectDataView(populated)).toStrictEqual(populated);
  });

  it('摘要保留可省略字段与空 tags，非 record properties 不伪造成字段值', () => {
    const wire = summary({ properties: ['legacy-value'] });
    const view = toObjectSummaryView(wire);

    expect(view.properties).toBeUndefined();
    expect(view.propertyLabels).toBeUndefined();
    expect(view.templateId).toBeUndefined();
    expect(view.templateType).toBeUndefined();
    expect(view.tags).toStrictEqual([]);
    expect(Object.hasOwn(view, 'contractTypeId')).toBe(false);
    expect(Object.hasOwn(view, 'templateHash')).toBe(false);
    expect(Object.hasOwn(view, 'sensitivityLevels')).toBe(false);
    expect(wire.properties).toStrictEqual(['legacy-value']);
    expect(wire.templateType).toBeNull();

    const populated = summary({
      templateId: '',
      templateType: 'future-template-kind',
      contractTypeId: '',
      templateHash: '',
      ignoredTemplateHash: '',
      parentId: '',
      propertyLabels: { field: 'public' },
      sensitivityLevels: ['public', 'future-level'],
      properties: { field: 'value' },
    });
    expect(toObjectSummaryView(populated)).toStrictEqual(populated);
  });

  it('详情和摘要保留显式非法标签，不能回退到模板 public', () => {
    const labels = {
      typo: 'publci',
      explicitNull: null,
      numeric: 0,
      nested: { sensitivityLevel: 'public' },
      array: ['public'],
      publicSibling: 'public',
      critical: 'critical',
    };
    const views = [
      toObjectDataView(object({ propertyLabels: labels })),
      toObjectSummaryView(summary({ propertyLabels: labels })),
    ];

    for (const view of views) {
      expect(view.propertyLabels).toStrictEqual(labels);
      for (const fieldId of ['typo', 'explicitNull', 'numeric', 'nested', 'array']) {
        expect(Object.hasOwn(view.propertyLabels ?? {}, fieldId)).toBe(true);
        expect(
          resolveFieldSensitivity({
            fieldId,
            propertyLabels: view.propertyLabels,
            definition: { sensitivityLevel: 'public' },
            template: { sensitivityLevel: 'public' },
          }),
        ).toBe('internal');
      }
      expect(
        resolveFieldSensitivity({
          fieldId: 'publicSibling',
          propertyLabels: view.propertyLabels,
          template: { sensitivityLevel: 'critical' },
        }),
      ).toBe('public');
      expect(
        resolveFieldSensitivity({
          fieldId: 'critical',
          propertyLabels: view.propertyLabels,
          template: { sensitivityLevel: 'public' },
        }),
      ).toBe('critical');
    }
    expect(labels.explicitNull).toBeNull();
    expect(labels.typo).toBe('publci');
  });

  it('回收站 nullable 字段不改写原 DTO，零时间和空字符串不当作缺失', () => {
    const wire: WireTrash = {
      id: 'synthetic-trash',
      itemType: 'object',
      originalId: 'synthetic-object',
      name: '合成回收项',
      deletedAt: 0,
      iconId: null,
      expiresAt: null,
      originalParentId: null,
      originalSectionType: null,
      contractTypeId: null,
    };
    const view = toTrashItemView(wire);

    expect(view).toStrictEqual({
      ...wire,
      iconId: undefined,
      expiresAt: undefined,
      originalParentId: undefined,
      originalSectionType: undefined,
      contractTypeId: undefined,
    });
    expect(wire.expiresAt).toBeNull();
    expect(wire.iconId).toBeNull();
    const populated: WireTrash = {
      ...wire,
      iconId: '',
      expiresAt: 0,
      originalParentId: '',
      originalSectionType: '',
      contractTypeId: '',
    };
    expect(toTrashItemView(populated)).toStrictEqual(populated);
  });
});
