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

type TranslateFn = (
  key: string,
  options?: { defaultValue?: string; count?: number },
) => string;

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

/** 属性对象中的字段定义元数据键（模板字段结构快照，非用户数据）。 */
const SCHEMA_KEY = '__fields';

/** 已知字段 → settings locale key（键为归一化 snake_case）。 */
const FIELD_LOCALE_KEYS: Record<string, string> = {
  [SCHEMA_KEY]: 'sync_conflict_field_schema',
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

/** 用户无法感知/无法修改、冲突 diff 中始终省略的字段：
 *  对象指纹（template_hash/ignored_template_hash）与簿记字段
 *  （version 每次编辑/同步应用都会 +1、updated_at 每次编辑都会变化——
 *  它们随 HLC 时间差异必然不同、与内容差异无关，详情头部已展示 HLC 时间）。
 *  后台同步引擎已对「仅簿记字段不同」的冲突做自动消解（delta.rs），
 *  此处省略保证残留冲突的 diff 只显示真实内容差异。 */
const OMIT_ALWAYS = new Set([
  'template_hash',
  'ignored_template_hash',
  'version',
  'updated_at',
]);

/** 空值判定：null / undefined / 空字符串 / 空数组。 */
function isEmptyValue(v: unknown): boolean {
  if (v === null || v === undefined || v === '') return true;
  return Array.isArray(v) && v.length === 0;
}

/**
 * 冲突 diff 中应省略的字段：用户无法感知也无法修改的系统元数据。
 * 对象指纹（template_hash/ignored_template_hash）始终省略；
 * children_ids/parent_id 仅在两侧都为空（无子对象/无父对象）时省略——
 * 存在真实关系差异（一侧有、一侧无）时保留展示。
 */
export function shouldOmitField(key: string, local: unknown, remote: unknown): boolean {
  const norm = normalizeFieldKey(key);
  if (OMIT_ALWAYS.has(norm)) return true;
  if (norm === 'children_ids' || norm === 'parent_id') {
    return isEmptyValue(local) && isEmptyValue(remote);
  }
  return false;
}

/** 时间字段（值应为 RFC3339 字符串）：显示时截断到秒，避免毫秒噪声。 */
const TIME_FIELDS = new Set([
  'created_at',
  'updated_at',
  'deleted_at',
  'expires_at',
  'last_synced_at',
  'last_trusted_at',
  'timestamp',
]);

/** 字段值是否为时间字段（归一化键命中时间字段集合）。 */
function isTimeField(key: string): boolean {
  return TIME_FIELDS.has(normalizeFieldKey(key));
}

/**
 * RFC3339 时间字符串 → 秒级精度（截断毫秒小数，`+00:00` 时区规范为 `Z`）。
 * 非 RFC3339 字符串原样返回（不改动用户数据中的日期字段值）。
 */
function formatTimeValue(value: unknown): string | null {
  if (typeof value !== 'string') return null;
  const m =
    /^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2})(?:\.\d+)?(Z|[+-]\d{2}:\d{2})$/.exec(
      value,
  );
  if (!m) return null;
  const tz = m[2] === '+00:00' ? 'Z' : m[2];
  return `${m[1]}${tz}`;
}

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

/** 同步表名 → settings locale key（冲突卡片/详情头部展示可读表名，替代原始表名如 objects）。 */
const TABLE_LOCALE_KEYS: Record<string, string> = {
  profiles: 'sync_conflict_table_profiles',
  objects: 'sync_conflict_table_objects',
  user_templates: 'sync_conflict_table_user_templates',
  trash_items: 'sync_conflict_table_trash_items',
};

/** 同步表名可读名：已知表名（objects 等）走 locale，未知表名做标题化兜底。 */
export function conflictTableLabel(table: string, t: TranslateFn): string {
  const localeKey = TABLE_LOCALE_KEYS[table];
  if (localeKey) {
    const label = t(`settings:${localeKey}`, { defaultValue: '' });
    if (label) return label;
  }
  return humanizeKey(table);
}

/** 属性对象内部的系统元数据键 → 可读名（与 TrashSnapshotView/HistoryViewer 一致）。 */
const NESTED_META_LABELS: Record<string, string> = {
  __templateName: 'settings:sync_conflict_field_template_name',
  __dynamic_group__: 'editor:field_types.dynamic_group',
  __attachments: 'settings:sync_conflict_field_attachments',
};

/** 嵌套对象内部的字段可读名：`__fields` 元数据走专属标签，
 *  `__templateName` 等系统元数据键走专属 i18n，
 *  其余优先 editor:fields.<id>（动态字段名），未知做标题化兜底。 */
export function nestedFieldLabel(key: string, t: TranslateFn): string {
  if (key === SCHEMA_KEY) {
    return t('settings:sync_conflict_field_schema', { defaultValue: 'Field Definitions' });
  }
  const metaLabelKey = NESTED_META_LABELS[key];
  if (metaLabelKey) {
    const metaLabel = t(metaLabelKey, { defaultValue: '' });
    if (metaLabel) return metaLabel;
  }
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
  if (normKey === 'type') {
    // __fields 字段定义中的属性类型代码（text/date/email…）→ editor:field_types 双语
    const label = t(`editor:field_types.${value}`, { defaultValue: '' });
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

/** 按字段语义格式化标量值：时间字段截断到秒，其余走通用标量。 */
function formatFieldScalar(key: string, value: unknown, t: TranslateFn): string {
  if (isTimeField(key)) {
    const trimmed = formatTimeValue(value);
    if (trimmed !== null) return trimmed;
  }
  return formatScalar(value, t);
}

/** 嵌套值可读化：标量原样（布尔 i18n）；对象逐行 `字段: 值`；数组逐行 `- 项`。 */
export function formatConflictValue(key: string, value: unknown, t: TranslateFn): string {
  if (value === null || value === undefined) return '';
  const known = lookupKnownValue(normalizeFieldKey(key), value, t);
  if (known !== null) return known;
  return formatNested(value, t, key);
}

function formatNested(value: unknown, t: TranslateFn, parentKey = ''): string {
  if (Array.isArray(value)) {
    if (value.length === 0) return '[]';
    return value
      .map((item) => `- ${formatValueItem(item, t, parentKey)}`)
      .join('\n');
  }
  if (value !== null && typeof value === 'object') {
    const entries = Object.entries(value as Record<string, unknown>);
    if (entries.length === 0) return '{}';
    return entries
      .map(([k, v]) => `${nestedFieldLabel(k, t)}: ${formatValueItem(v, t, k)}`)
      .join('\n');
  }
  return formatFieldScalar(parentKey, value, t);
}

function formatValueItem(value: unknown, t: TranslateFn, parentKey = ''): string {
  if (value !== null && typeof value === 'object') return formatNested(value, t, parentKey);
  return formatFieldScalar(parentKey, value, t);
}

/** 顶层字段值的展示上限（字符数），超长截断避免单元格过大。 */
const CONFLICT_VALUE_MAX_LEN = 600;

/** 截断超长展示值（配合 CSS pre-wrap 换行展示）。 */
export function truncateConflictValue(text: string): string {
  if (text.length <= CONFLICT_VALUE_MAX_LEN) return text;
  return `${text.slice(0, CONFLICT_VALUE_MAX_LEN)}…`;
}

/** 深度相等：JSON 序列化比较（冲突数据均为可序列化值）。 */
function valuesEqual(a: unknown, b: unknown): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}

/** 剥离对象顶层的 `__fields` 元数据键（仅用于两侧相同时折叠展示）。 */
function withoutSchemaKeys(v: unknown): unknown {
  if (v !== null && typeof v === 'object' && !Array.isArray(v)) {
    const obj = v as Record<string, unknown>;
    const rest: Record<string, unknown> = {};
    for (const [k, val] of Object.entries(obj)) {
      if (k !== SCHEMA_KEY) rest[k] = val;
    }
    return rest;
  }
  return v;
}

/**
 * 两侧 `__fields` 相同 → 折叠为一条摘要条目（无差异、不展开）；
 * 单侧缺失或内容有差异 → 返回 null（照常展开以显示具体差异）。
 */
function buildSchemaSummaryEntry(
  local: unknown,
  remote: unknown,
  t: TranslateFn,
): DiffEntry | null {
  const toObj = (v: unknown): Record<string, unknown> | null =>
    v !== null && typeof v === 'object' && !Array.isArray(v)
      ? (v as Record<string, unknown>)
      : null;
  const lFields = toObj(local)?.[SCHEMA_KEY];
  const rFields = toObj(remote)?.[SCHEMA_KEY];
  if (lFields === undefined || rFields === undefined) return null;
  if (!valuesEqual(lFields, rFields)) return null;
  const count =
    lFields !== null && typeof lFields === 'object' && !Array.isArray(lFields)
      ? Object.keys(lFields as Record<string, unknown>).length
      : 0;
  const summary = t('settings:sync_conflict_field_schema_count', {
    defaultValue: '{{count}} fields',
    count,
  });
  return {
    path: SCHEMA_KEY,
    label: conflictFieldLabel(SCHEMA_KEY, t),
    localText: summary,
    remoteText: summary,
    localLevel: null,
    remoteLevel: null,
    localIcon: null,
    remoteIcon: null,
    changed: false,
  };
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

/** 叶子值文本化：已知代码值（模板类型/图标等）走 i18n，时间字段截秒，嵌套块走可读化多行，标量走 formatScalar。 */
function formatLeafText(key: string, value: unknown, t: TranslateFn): string {
  if (typeof value === 'string') {
    const known = lookupKnownValue(normalizeFieldKey(key), value, t);
    if (known !== null) return known;
  }
  if (value !== null && typeof value === 'object') return formatNested(value, t, key);
  return formatFieldScalar(key, value, t);
}

/** 取叶子路径的末段键（如 `Fields › Type` → `Type`），用于值 i18n / 图标解析。 */
function lastKeyOf(path: string): string {
  const seg = path.split(' › ').pop();
  return seg || '';
}

/**
 * 对象/数组字段 → 叶子级 diff 条目（本地/远程逐叶配对，供 UI 逐行高亮差异）。
 * 标量字段或双方均为空容器时返回 null（沿用整值渲染）。
 * `onlyDifferences` 为 true 时仅返回有差异的叶子（含单侧缺失），供「只看差异」模式。
 */
export function buildDiffEntries(
  key: string,
  local: unknown,
  remote: unknown,
  t: TranslateFn,
  onlyDifferences = false,
): DiffEntry[] | null {
  const isObjectLike = (v: unknown): boolean => v !== null && typeof v === 'object';
  if (!isObjectLike(local) && !isObjectLike(remote)) return null;

  // __fields 元数据：两侧相同 → 折叠为摘要条目并剥离展开；否则照常展开
  const schemaEntry = buildSchemaSummaryEntry(local, remote, t);
  const lSource = schemaEntry ? withoutSchemaKeys(local) : local;
  const rSource = schemaEntry ? withoutSchemaKeys(remote) : remote;

  const localLeaves: Leaf[] = [];
  const remoteLeaves: Leaf[] = [];
  collectLeaves('', '', lSource, t, 0, localLeaves);
  collectLeaves('', '', rSource, t, 0, remoteLeaves);

  // 双方均为空容器 / 退化标量侧：仅一条空路径，无展开价值
  const paths = Array.from(
    new Set([...localLeaves.map((l) => l.path), ...remoteLeaves.map((l) => l.path)]),
  );
  if (paths.length === 0 || (paths.length === 1 && paths[0] === '')) {
    return schemaEntry ? [schemaEntry] : null;
  }

  const localByPath = new Map(localLeaves.map((l) => [l.path, l]));
  const remoteByPath = new Map(remoteLeaves.map((l) => [l.path, l]));
  const labelByPath = new Map(
    [...localLeaves, ...remoteLeaves].map((l) => [l.path, l.label]),
  );

  const entries = paths.map((path) => {
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
  const all = schemaEntry ? [schemaEntry, ...entries] : entries;
  if (!onlyDifferences) return all;
  // 「只看差异」模式：剔除无差异叶子（摘要条目无差异、不展示）
  return all.filter((e) => e.changed);
}
