/**
 * 同步冲突 diff 的字段元数据：字段名 i18n、已知值（预植入模板/分区/对象类型/布尔）
 * i18n、嵌套值可读化格式化。供 SyncConflictDialog 使用，让用户不再面对
 * `account_id` / `childrenIds` 等原始键名与紧凑 JSON。
 *
 * 字段键可能来自本地侧（camelCase，如 `childrenIds`）或远程侧（如 `accountId`），
 * 归一化到 snake_case 后统一查表，兼容两种来源。
 */

import type { SensitivityLevel } from '@/types/template';
import type { LucideIcon } from 'lucide-react';
import { resolveCustomIcon } from '@/lib/pageIcons';

type TranslateFn = (key: string, options?: { defaultValue?: string }) => string;

/** 敏感度 token 全集（与前端 `SensitivityLevel` 类型一致，供差异值渲染徽章）。 */
const SENSITIVITY_LEVELS: readonly SensitivityLevel[] = ['public', 'internal', 'sensitive', 'critical'];

/** 值是否为敏感度 token（如 `internal`/`critical`），是则可在 UI 渲染敏感度徽章。 */
export function isSensitivityLevel(value: unknown): value is SensitivityLevel {
  return (
    typeof value === 'string' &&
    (SENSITIVITY_LEVELS as readonly string[]).includes(value)
  );
}

/** 归一化字段键：camelCase / snake_case → snake_case（`accountId`/`account_id` → `account_id`）。 */
export function normalizeFieldKey(key: string): string {
  return key
    .replace(/([a-z0-9])([A-Z])/g, '$1_$2')
    .replace(/([A-Z]+)([A-Z][a-z])/g, '$1_$2')
    .toLowerCase();
}

/** 已知字段 → settings locale key（键为归一化 snake_case）。 */
const FIELD_LOCALE_KEYS: Record<string, string> = {
  id: 'sync_conflict_field_id',
  account_id: 'sync_conflict_field_account_id',
  type_id: 'sync_conflict_field_type_id',
  section_type: 'sync_conflict_field_section_type',
  sensitivity_level: 'sync_conflict_field_sensitivity_level',
  name: 'sync_conflict_field_name',
  icon_name: 'sync_conflict_field_icon_name',
  template_id: 'sync_conflict_field_template_id',
  template_type: 'sync_conflict_field_template_type',
  contract_type_id: 'sync_conflict_field_contract_type_id',
  template_hash: 'sync_conflict_field_template_hash',
  ignored_template_hash: 'sync_conflict_field_ignored_template_hash',
  parent_id: 'sync_conflict_field_parent_id',
  children_ids: 'sync_conflict_field_children_ids',
  is_deleted: 'sync_conflict_field_is_deleted',
  deleted_at: 'sync_conflict_field_deleted_at',
  created_at: 'sync_conflict_field_created_at',
  updated_at: 'sync_conflict_field_updated_at',
  version: 'sync_conflict_field_version',
  properties: 'sync_conflict_field_properties',
  tags_json: 'sync_conflict_field_tags_json',
  property_labels: 'sync_conflict_field_property_labels',
  data: 'sync_conflict_field_data',
  category: 'sync_conflict_field_category',
  item_type: 'sync_conflict_field_item_type',
  original_id: 'sync_conflict_field_original_id',
  original_parent_id: 'sync_conflict_field_original_parent_id',
  original_sort_order: 'sync_conflict_field_original_sort_order',
  expires_at: 'sync_conflict_field_expires_at',
  deleted_by: 'sync_conflict_field_deleted_by',
  name_snapshot: 'sync_conflict_field_name_snapshot',
  icon_snapshot: 'sync_conflict_field_icon_snapshot',
  icon_id: 'sync_conflict_field_icon_id',
  contract_bindings: 'sync_conflict_field_contract_bindings',
  properties_json: 'sync_conflict_field_properties_json',
};

/** 预植入模板 key → settings locale key（模板显示名）。 */
const BUILTIN_TEMPLATE_KEYS: Record<string, string> = {
  identity: 'sync_conflict_tpl_identity',
  id_card: 'sync_conflict_tpl_id_card',
  passport: 'sync_conflict_tpl_passport',
  visa: 'sync_conflict_tpl_visa',
  bank: 'sync_conflict_tpl_bank',
  card: 'sync_conflict_tpl_card',
  education: 'sync_conflict_tpl_education',
  employment: 'sync_conflict_tpl_employment',
  address: 'sync_conflict_tpl_address',
  contact: 'sync_conflict_tpl_contact',
};

/** 对象分区/模板分类 key → settings locale key。 */
const CATEGORY_KEYS: Record<string, string> = {
  identity: 'sync_conflict_cat_identity',
  travel: 'sync_conflict_cat_travel',
  financial: 'sync_conflict_cat_financial',
  professional: 'sync_conflict_cat_professional',
};

/** 对象类型（typeId）key → settings locale key。分区值（identity 等）复用分区 key。 */
const OBJECT_TYPE_KEYS: Record<string, string> = {
  note: 'sync_conflict_type_note',
  identity: 'sync_conflict_cat_identity',
  travel: 'sync_conflict_cat_travel',
  financial: 'sync_conflict_cat_financial',
  professional: 'sync_conflict_cat_professional',
};

/** 图标字段（其字符串值即图标 ID，可渲染图标图案 + i18n 名称）。 */
const ICON_FIELDS = new Set(['icon_name', 'icon_id', 'icon_snapshot']);

/**
 * 解析图标字段的 Lucide 图标组件（未知 ID 经 resolveCustomIcon 兜底到 document）。
 * 非图标字段或非字符串值返回 null（保持文本渲染）。
 */
export function resolveConflictIcon(key: string, value: unknown): LucideIcon | null {
  if (typeof value !== 'string' || !ICON_FIELDS.has(normalizeFieldKey(key))) return null;
  return resolveCustomIcon(value);
}

/** 字段可读名：已知字段走 locale，未知字段做 camel/snake → 标题化兜底。 */
export function conflictFieldLabel(key: string, t: TranslateFn): string {
  const localeKey = FIELD_LOCALE_KEYS[normalizeFieldKey(key)];
  if (localeKey) {
    const label = t(`settings:${localeKey}`, { defaultValue: '' });
    if (label) return label;
  }
  return humanizeKey(key);
}

/** 嵌套对象内部的字段可读名：优先 editor:fields.<id>（动态字段名），未知做标题化兜底。 */
export function nestedFieldLabel(key: string, t: TranslateFn): string {
  const label = t(`editor:fields.${key}`, { defaultValue: '' });
  if (label) return label;
  return humanizeKey(key);
}

function humanizeKey(key: string): string {
  const words = normalizeFieldKey(key).split('_').filter(Boolean);
  if (words.length === 0) return key;
  return words.map((w) => w.charAt(0).toUpperCase() + w.slice(1)).join(' ');
}

/** 已知代码值（预植入模板 id / 分区 / 对象类型）→ i18n 名称；未知返回 null 保持原值。 */
function lookupKnownValue(normKey: string, value: unknown, t: TranslateFn): string | null {
  if (typeof value !== 'string') return null;
  if (normKey === 'template_id' || normKey === 'contract_type_id') {
    const localeKey = BUILTIN_TEMPLATE_KEYS[value];
    if (localeKey) return t(`settings:${localeKey}`, { defaultValue: value });
    return null;
  }
  if (normKey === 'section_type' || normKey === 'category') {
    const localeKey = CATEGORY_KEYS[value];
    if (localeKey) return t(`settings:${localeKey}`, { defaultValue: value });
    return null;
  }
  if (normKey === 'type_id') {
    const localeKey = OBJECT_TYPE_KEYS[value];
    if (localeKey) return t(`settings:${localeKey}`, { defaultValue: value });
    return null;
  }
  if (normKey === 'template_type') {
    const label = t(`settings:sync_conflict_tpltype_${value}`, { defaultValue: '' });
    return label || null;
  }
  if (ICON_FIELDS.has(normKey)) {
    const label = t(`settings:sync_conflict_icon_${value}`, { defaultValue: '' });
    return label || null;
  }
  return null;
}

function formatScalar(value: unknown, t: TranslateFn): string {
  if (typeof value === 'boolean') {
    return value
      ? t('settings:sync_conflict_value_true', { defaultValue: 'Yes' })
      : t('settings:sync_conflict_value_false', { defaultValue: 'No' });
  }
  return String(value);
}

/** 嵌套值可读化：标量原样（布尔 i18n）；对象逐行 `字段: 值`；数组逐行 `- 项`。 */
export function formatConflictValue(key: string, value: unknown, t: TranslateFn): string {
  if (value === null || value === undefined) return '';
  const known = lookupKnownValue(normalizeFieldKey(key), value, t);
  if (known !== null) return known;
  return formatNested(value, t);
}

function formatNested(value: unknown, t: TranslateFn): string {
  if (Array.isArray(value)) {
    if (value.length === 0) return '[]';
    return value.map((item) => `- ${formatValueItem(item, t)}`).join('\n');
  }
  if (value !== null && typeof value === 'object') {
    const entries = Object.entries(value as Record<string, unknown>);
    if (entries.length === 0) return '{}';
    return entries
      .map(([k, v]) => `${nestedFieldLabel(k, t)}: ${formatValueItem(v, t)}`)
      .join('\n');
  }
  return formatScalar(value, t);
}

function formatValueItem(value: unknown, t: TranslateFn): string {
  if (value !== null && typeof value === 'object') return formatNested(value, t);
  return formatScalar(value, t);
}

/** 顶层字段值的展示上限（字符数），超长截断避免单元格过大。 */
export const CONFLICT_VALUE_MAX_LEN = 600;

/** 截断超长展示值（配合 CSS pre-wrap 换行展示）。 */
export function truncateConflictValue(text: string): string {
  if (text.length <= CONFLICT_VALUE_MAX_LEN) return text;
  return `${text.slice(0, CONFLICT_VALUE_MAX_LEN)}…`;
}

/** 深度相等：JSON 序列化比较（冲突数据均为可序列化值）。 */
function valuesEqual(a: unknown, b: unknown): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}

/** 叶子级 diff 条目：对象/数组字段按叶子展开，本地/远程逐叶配对。 */
export interface DiffEntry {
  /** 原始键路径（用于 React key 与本地/远程配对，不随语言变化）。 */
  path: string;
  /** 可读标签（i18n 名称链，如 `姓名`、`Fields › 出生日期`）。 */
  label: string;
  /** 本地侧文本；null 表示本地缺失。 */
  localText: string | null;
  /** 远程侧文本；null 表示远程缺失。 */
  remoteText: string | null;
  /** 本地侧是否为敏感度 token（渲染敏感度徽章）；否则 null。 */
  localLevel: SensitivityLevel | null;
  /** 远程侧是否为敏感度 token（渲染敏感度徽章）；否则 null。 */
  remoteLevel: SensitivityLevel | null;
  /** 本地侧图标字段的 Lucide 组件（渲染图标图案）；否则 null。 */
  localIcon: LucideIcon | null;
  /** 远程侧图标字段的 Lucide 组件（渲染图标图案）；否则 null。 */
  remoteIcon: LucideIcon | null;
  /** 该叶子是否存在差异（含单侧缺失）。 */
  changed: boolean;
}

interface Leaf {
  path: string;
  label: string;
  value: unknown;
}

/** 嵌套展开深度上限：超过后按整块值呈现（避免无限递归与超长标签链）。 */
const DIFF_MAX_DEPTH = 3;

/** 将对象/数组值按叶子展开（路径基于原始键、标签基于 i18n 名称链）。 */
function collectLeaves(
  prefix: string,
  label: string,
  value: unknown,
  t: TranslateFn,
  depth: number,
  out: Leaf[],
): void {
  if (value === null || value === undefined) return;
  if (depth >= DIFF_MAX_DEPTH || typeof value !== 'object') {
    out.push({ path: prefix, label, value });
    return;
  }
  if (Array.isArray(value)) {
    if (value.length === 0) {
      out.push({ path: prefix, label, value });
      return;
    }
    value.forEach((item, i) => {
      collectLeaves(`${prefix}[${i}]`, `${label}[${i}]`, item, t, depth + 1, out);
    });
    return;
  }
  const entries = Object.entries(value as Record<string, unknown>);
  if (entries.length === 0) {
    out.push({ path: prefix, label, value });
    return;
  }
  for (const [k, v] of entries) {
    const subPath = prefix ? `${prefix} › ${k}` : k;
    const subLabel = prefix ? `${label} › ${nestedFieldLabel(k, t)}` : nestedFieldLabel(k, t);
    collectLeaves(subPath, subLabel, v, t, depth + 1, out);
  }
}

/** 叶子值文本化：已知代码值（模板类型/图标等）走 i18n，嵌套块走可读化多行，标量走 formatScalar。 */
function formatLeafText(key: string, value: unknown, t: TranslateFn): string {
  if (typeof value === 'string') {
    const known = lookupKnownValue(normalizeFieldKey(key), value, t);
    if (known !== null) return known;
  }
  if (value !== null && typeof value === 'object') return formatNested(value, t);
  return formatScalar(value, t);
}

/** 取叶子路径的末段键（如 `Fields › Type` → `Type`），用于值 i18n / 图标解析。 */
function lastKeyOf(path: string): string {
  const seg = path.split(' › ').pop();
  return seg || '';
}

/**
 * 对象/数组字段 → 叶子级 diff 条目（本地/远程逐叶配对，供 UI 逐行高亮差异）。
 * 标量字段或双方均为空容器时返回 null（沿用整值渲染）。
 */
export function buildDiffEntries(
  key: string,
  local: unknown,
  remote: unknown,
  t: TranslateFn,
): DiffEntry[] | null {
  const isObjectLike = (v: unknown): boolean => v !== null && typeof v === 'object';
  if (!isObjectLike(local) && !isObjectLike(remote)) return null;

  const localLeaves: Leaf[] = [];
  const remoteLeaves: Leaf[] = [];
  collectLeaves('', '', local, t, 0, localLeaves);
  collectLeaves('', '', remote, t, 0, remoteLeaves);

  // 双方均为空容器 / 退化标量侧：仅一条空路径，无展开价值
  const paths = Array.from(
    new Set([...localLeaves.map((l) => l.path), ...remoteLeaves.map((l) => l.path)]),
  );
  if (paths.length === 0 || (paths.length === 1 && paths[0] === '')) return null;

  const localByPath = new Map(localLeaves.map((l) => [l.path, l]));
  const remoteByPath = new Map(remoteLeaves.map((l) => [l.path, l]));
  const labelByPath = new Map(
    [...localLeaves, ...remoteLeaves].map((l) => [l.path, l.label]),
  );

  return paths.map((path) => {
    const leafKey = lastKeyOf(path);
    const lValue = localByPath.get(path)?.value;
    const rValue = remoteByPath.get(path)?.value;
    return {
      path,
      label: labelByPath.get(path) || humanizeKey(key),
      localText: lValue === undefined ? null : formatLeafText(leafKey, lValue, t),
      remoteText: rValue === undefined ? null : formatLeafText(leafKey, rValue, t),
      localLevel: lValue === undefined ? null : isSensitivityLevel(lValue) ? lValue : null,
      remoteLevel: rValue === undefined ? null : isSensitivityLevel(rValue) ? rValue : null,
      localIcon: lValue === undefined ? null : resolveConflictIcon(leafKey, lValue),
      remoteIcon: rValue === undefined ? null : resolveConflictIcon(leafKey, rValue),
      changed: !valuesEqual(lValue, rValue),
    };
  });
}
