/**
 * 对象 properties 扁平化——共享核心实现（P024 收敛三处重复）。
 *
 * 三处调用方（HistoryViewer / objectDetailUtils / WorkspaceObjectCard）此前各有一份
 * 近似实现，`__` 前缀 key 处理规则已分叉。此处统一核心，差异点参数化：
 * - `keepMetaKeys`：保留 fieldDefs 中定义的 `__` 前缀 key（如 `__dynamic_group__`）。
 *   历史快照需要（HistoryViewer 传 true）；普通对象展示一律跳过（另两处传 false）。
 * - `flattenDynamicGroups`：dynamic_group 子字段是展平为独立条目（对象详情/工作区卡片），
 *   还是保留分组结构（历史快照渲染组头 + 子行）。
 */

export interface DynamicChildItem {
  label: string;
  value: string;
  type?: string;
}

export type FlattenedPropertyEntry =
  | {
      kind: 'field';
      key: string;
      value: string;
      label?: string;
      type?: string;
      /** dynamic_group 展平子字段的完整 id（`${key}.${id}`），普通字段无。 */
      fieldId?: string;
    }
  | {
      kind: 'dynamicGroup';
      key: string;
      label?: string;
      type?: string;
      children: DynamicChildItem[];
    };

export interface FlattenPropertyOptions {
  /** 保留 fieldDefs 中定义的 `__` 前缀 key（默认 false：一律跳过）。 */
  keepMetaKeys?: boolean;
  /** dynamic_group 子字段展平为独立条目（默认 true）；false 时保留分组结构。 */
  flattenDynamicGroups?: boolean;
}

export interface FieldDefShape {
  type?: string;
  name?: string;
}

/**
 * 将对象 properties 扁平化为可展示条目列表。
 *
 * @param props 对象 properties（可能含 `__fields` 元数据）
 * @param fieldOrder 可选字段顺序（按此排序，未列出字段按 key 字典序兜底）
 * @param fieldDefs 可选字段定义（优先于 `props.__fields`；仅 type/name 两键被读取）
 * @param options 差异点参数（见 `FlattenPropertyOptions`）
 */
export function flattenPropertyEntries(
  props: Record<string, unknown> | undefined,
  fieldOrder?: string[],
  fieldDefs?: Record<string, FieldDefShape>,
  options: FlattenPropertyOptions = {},
): FlattenedPropertyEntry[] {
  if (!props) return [];
  const { keepMetaKeys = false, flattenDynamicGroups = true } = options;
  const defs =
    fieldDefs ??
    ((props.__fields as Record<string, FieldDefShape> | undefined) ?? {});
  const entries: FlattenedPropertyEntry[] = [];

  for (const [k, v] of Object.entries(props)) {
    // `__` 前缀：默认一律跳过；keepMetaKeys 时仅保留字段定义中存在的 key（如 __dynamic_group__）
    if (k.startsWith('__') && !(keepMetaKeys && defs[k])) continue;
    if (v === null || v === undefined || v === '') continue;

    if (defs[k]?.type === 'dynamic_group' && Array.isArray(v)) {
      if (flattenDynamicGroups) {
        // 展平模式：每个子字段作为独立条目返回，label 用子字段 name
        for (const item of v) {
          if (!item || typeof item !== 'object') continue;
          const { id, name, value: itemVal, type: itemType } = item as Record<string, unknown>;
          if (name === undefined || name === null || name === '') continue;
          let displayVal = '';
          if (Array.isArray(itemVal)) {
            displayVal = itemVal.join(', ');
          } else if (itemVal !== null && itemVal !== undefined) {
            displayVal = String(itemVal);
          }
          entries.push({
            kind: 'field',
            key: k,
            label: String(name),
            value: displayVal,
            type: typeof itemType === 'string' ? itemType : undefined,
            fieldId: id ? `${k}.${id}` : `${k}.${name}`,
          });
        }
      } else {
        // 分组模式：组头 + 子行结构
        if (v.length === 0) continue;
        const children: DynamicChildItem[] = [];
        for (const item of v) {
          if (!item || typeof item !== 'object') continue;
          const { name, value: itemVal, type: itemType } = item as Record<string, unknown>;
          if (name === undefined || name === null || name === '') continue;
          let displayVal = '';
          if (Array.isArray(itemVal)) {
            displayVal = itemVal.join(', ');
          } else if (itemVal !== null && itemVal !== undefined) {
            displayVal = String(itemVal);
          }
          children.push({
            label: String(name),
            value: displayVal,
            type: typeof itemType === 'string' ? itemType : undefined,
          });
        }
        entries.push({
          kind: 'dynamicGroup',
          key: k,
          label: defs[k]?.name,
          type: 'dynamic_group',
          children,
        });
      }
      continue;
    }

    if (typeof v === 'string') {
      entries.push({ kind: 'field', key: k, value: v, label: defs[k]?.name, type: defs[k]?.type });
    } else if (typeof v === 'number' || typeof v === 'boolean') {
      entries.push({
        kind: 'field',
        key: k,
        value: String(v),
        label: defs[k]?.name,
        type: defs[k]?.type,
      });
    } else if (Array.isArray(v) && v.length > 0) {
      entries.push({
        kind: 'field',
        key: k,
        value: v.join(', '),
        label: defs[k]?.name,
        type: defs[k]?.type,
      });
    }
  }

  if (fieldOrder && fieldOrder.length > 0) {
    const orderMap = new Map(fieldOrder.map((id, i) => [id, i]));
    entries.sort((a, b) => {
      const ia = orderMap.get(a.key);
      const ib = orderMap.get(b.key);
      if (ia !== undefined && ib !== undefined) return ia - ib;
      if (ia !== undefined) return -1;
      if (ib !== undefined) return 1;
      return a.key.localeCompare(b.key);
    });
  }
  return entries;
}
