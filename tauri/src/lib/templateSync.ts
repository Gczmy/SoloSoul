/**
 * Template sync utilities (§29 模板更新后对象手动同步).
 *
 * Mirrors the Rust fingerprint algorithm in `commands/object/mod.rs` so the
 * frontend can cheaply detect whether an object is still based on the latest
 * version of its template without an IPC round-trip per card.
 */

import type { UserTemplate, TemplateProperty } from '@/types/template';

export interface TemplateSyncStatus {
  needsSync: boolean;
  currentHash?: string;
  latestHash?: string;
  templateExists: boolean;
}

export interface SyncFieldInfo {
  id: string;
  name: string;
  fieldType: string;
}

export interface SyncFieldChangeItem {
  kind: 'type' | 'name' | 'sensitivity' | 'options';
  payload?:
    | { oldType: string; newType: string }
    | { oldName: string; newName: string }
    | { oldLevel: string; newLevel: string };
}

export interface SyncFieldChange {
  id: string;
  name: string;
  fieldType: string;
  changes: SyncFieldChangeItem[];
}

export interface SyncFieldIncompatible {
  id: string;
  name: string;
  oldType: string;
  newType: string;
  oldValuePreview: string;
}

export interface TemplateSyncResult {
  hasChanges: boolean;
  templateHash: string;
  fieldsAdded: SyncFieldInfo[];
  fieldsDeprecated: SyncFieldInfo[];
  fieldsUpdated: SyncFieldChange[];
  fieldsIncompatible: SyncFieldIncompatible[];
}

export interface DeprecatedField {
  id: string;
  name: string;
  fieldType: string;
  value: unknown;
  deprecatedAt: string;
  reason: string;
}

/** Minimal object info required by sync checks. */
export interface SyncableObject {
  id: string;
  templateId?: string;
  templateHash?: string;
  ignoredTemplateHash?: string;
}

async function sha256Hex(input: string): Promise<string> {
  const data = new TextEncoder().encode(input);
  const digest = await crypto.subtle.digest('SHA-256', data);
  const bytes = Array.from(new Uint8Array(digest));
  return bytes.map((b) => b.toString(16).padStart(2, '0')).join('');
}

/**
 * 计算模板指纹，与后端 `template_fingerprint` 使用相同的规范：
 * - 忽略 id、accountId、createdAt、updatedAt
 * - 字段按 id 稳定排序
 * - Option 字段缺失时以 null 参与序列化
 * - SHA-256 后取前 8 字节（16 个 hex 字符）
 */
export async function computeTemplateFingerprint(tpl: UserTemplate): Promise<string> {
  const sortedProps = [...tpl.properties].sort((a, b) => a.id.localeCompare(b.id));
  const canonical = {
    properties: sortedProps.map((p) => propertyToCanonical(p)),
  };
  const fullHash = await sha256Hex(JSON.stringify(canonical));
  return fullHash.slice(0, 16);
}

function propertyToCanonical(p: TemplateProperty): Record<string, unknown> {
  const def: Record<string, unknown> = {
    id: p.id,
    name: p.name,
    type: p.type,
  };
  // 与后端 TemplateProperty 的 serde 序列化名保持一致：
  // 后端 struct 使用 #[serde(rename_all = "camelCase")]，因此 sensitivity_level / deprecated_at
  // 实际序列化为 sensitivityLevel / deprecatedAt；其余字段与前端 camelCase 一致。
  if (p.sensitivityLevel != null) def.sensitivityLevel = p.sensitivityLevel;
  if (p.options != null) def.options = p.options;
  if (p.deprecatedAt != null) def.deprecatedAt = p.deprecatedAt;
  if (p.contractField != null) def.contractField = p.contractField;
  if (p.contractBindings != null) def.contractBindings = p.contractBindings;
  if (p.allowedTypes != null) def.allowedTypes = p.allowedTypes;
  if (p.maxItems != null) def.maxItems = p.maxItems;
  return def;
}

/**
 * 为模板列表预计算指纹映射，用于批量判断对象是否需要同步。
 */
export async function buildTemplateHashMap(
  templates: UserTemplate[],
): Promise<Map<string, string>> {
  const map = new Map<string, string>();
  for (const tpl of templates) {
    const hash = await computeTemplateFingerprint(tpl);
    map.set(tpl.id, hash);
  }
  return map;
}

/**
 * 判断对象是否需要同步模板更新。
 * - 无 templateId：无需同步
 * - 模板已不存在：无需同步（保持对象现有字段）
 * - 对象缺少 templateHash：需要同步（旧对象首次纳入同步检测）
 * - templateHash 与模板当前指纹不一致：需要同步
 */
export function objectNeedsSync(
  obj: SyncableObject,
  templateHashMap: Map<string, string>,
): boolean {
  if (!obj.templateId) return false;
  const latestHash = templateHashMap.get(obj.templateId);
  if (!latestHash) return false;
  if (obj.templateHash === latestHash || obj.ignoredTemplateHash === latestHash) return false;
  return true;
}
