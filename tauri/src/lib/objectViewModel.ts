/** Wire DTO 由 Rust 生成；这里只描述展示层和编辑器允许的本地形状。 */
import type {
  JsonObject,
  JsonValue,
  ObjectData,
  ObjectSummary,
  TrashItemSummary,
} from './generated/ipcContracts';
import { asFieldRecord } from './fieldSensitivity';

type OptionalNonNull<T, K extends keyof T> = { [P in K]?: Exclude<T[P], null> };
type DataNullable =
  | 'templateId'
  | 'templateType'
  | 'templateHash'
  | 'ignoredTemplateHash'
  | 'deletedAt'
  | 'contractTypeId';
export type ObjectDataView = Omit<ObjectData, DataNullable | 'properties' | 'propertyLabels'> &
  OptionalNonNull<ObjectData, DataNullable> & {
    properties: Record<string, unknown>;
    propertyLabels?: Record<string, unknown>;
  };

type SummaryIdentity = 'id' | 'name' | 'typeId' | 'sensitivityLevel' | 'createdAt' | 'updatedAt';
type SummaryNullable = 'templateId' | 'templateType';
/** 新建对象先写入局部摘要；列表接口本身仍要求完整的生成 DTO。 */
export type ObjectSummaryView = Pick<ObjectSummary, SummaryIdentity> &
  Partial<
    Omit<ObjectSummary, SummaryIdentity | SummaryNullable | 'properties' | 'propertyLabels'>
  > &
  OptionalNonNull<ObjectSummary, SummaryNullable> & {
    properties?: Record<string, unknown>;
    propertyLabels?: Record<string, unknown>;
  };

type TrashNullable =
  | 'iconId'
  | 'expiresAt'
  | 'originalParentId'
  | 'originalSectionType'
  | 'contractTypeId';
export type TrashItemView = Omit<TrashItemSummary, TrashNullable> &
  OptionalNonNull<TrashItemSummary, TrashNullable>;

export function toObjectDataView(value: ObjectData): ObjectDataView {
  return {
    ...value,
    templateId: value.templateId ?? undefined,
    templateType: value.templateType ?? undefined,
    templateHash: value.templateHash ?? undefined,
    ignoredTemplateHash: value.ignoredTemplateHash ?? undefined,
    deletedAt: value.deletedAt ?? undefined,
    contractTypeId: value.contractTypeId ?? undefined,
    properties: asFieldRecord(value.properties) ?? {},
    // 保留标签中的非法/显式 null 值，让共享敏感度策略决定回退，不能误降级为 public。
    propertyLabels: asFieldRecord(value.propertyLabels),
  };
}

export function toObjectSummaryView(value: ObjectSummary): ObjectSummaryView {
  return {
    ...value,
    templateId: value.templateId ?? undefined,
    templateType: value.templateType ?? undefined,
    properties: asFieldRecord(value.properties),
    propertyLabels: asFieldRecord(value.propertyLabels),
  };
}

export function toTrashItemView(value: TrashItemSummary): TrashItemView {
  return {
    ...value,
    iconId: value.iconId ?? undefined,
    expiresAt: value.expiresAt ?? undefined,
    originalParentId: value.originalParentId ?? undefined,
    originalSectionType: value.originalSectionType ?? undefined,
    contractTypeId: value.contractTypeId ?? undefined,
  };
}

/** 历史快照允许任意 JSON；仅对象形状可以交给字段展示器。 */
export function snapshotFieldRecord(value: JsonValue): Record<string, unknown> | null {
  return asFieldRecord(value) ?? null;
}

/** 编辑器 unknown 值必须在写入边界验证，不能用类型断言冒充 JSON。 */
export function toJsonObject(value: Record<string, unknown>): JsonObject {
  const ancestors = new Set<object>();
  const convert = (item: unknown): JsonValue => {
    if (item === null || typeof item === 'string' || typeof item === 'boolean') return item;
    // 与原 IPC JSON 序列化规则一致：非有限数值以及数组中的 undefined 写为 null。
    if (typeof item === 'number') return Number.isFinite(item) ? item : null;
    if (item === undefined) return null;
    if (typeof item !== 'object') throw new TypeError('Object properties must contain JSON values');
    if (ancestors.has(item)) throw new TypeError('Object properties cannot contain cycles');
    ancestors.add(item);
    try {
      if (Array.isArray(item)) return Array.from(item, convert);
      return convertObject(item);
    } finally {
      ancestors.delete(item);
    }
  };
  const convertObject = (item: object): JsonObject => {
    const prototype: unknown = Object.getPrototypeOf(item);
    if (prototype !== Object.prototype && prototype !== null) {
      throw new TypeError('Object properties must contain plain JSON objects');
    }
    // fromEntries 保留 __proto__ 自有键；不修改正在编辑的原对象。
    return Object.fromEntries(
      Object.entries(item)
        .filter(([, entry]) => entry !== undefined)
        .map(([key, entry]) => [key, convert(entry)]),
    );
  };
  if (value === null || typeof value !== 'object' || Array.isArray(value)) {
    throw new TypeError('Object properties must be a JSON object');
  }
  ancestors.add(value);
  return convertObject(value);
}
