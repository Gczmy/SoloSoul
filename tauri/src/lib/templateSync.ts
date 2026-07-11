/**
 * Template sync utilities (§29 模板更新后对象手动同步).
 *
 * 模板指纹由后端 `template_hash_map` 命令统一计算，前端仅消费结果，
 * 避免前后端序列化细节不一致导致误判。
 */

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
