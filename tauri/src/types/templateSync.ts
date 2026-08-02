/**
 * P228: 模板同步共享类型（§29 模板更新后对象手动同步）。
 *
 * 从 `lib/templateSync.ts` 抽出——`stores/objectStore.ts` 与 `lib/templateSync.ts`
 * 之间存在循环依赖（objectStore 类型引用 templateSync，templateSync 运行时引用
 * objectStore）。类型抽到 `types/` 后，objectStore 只依赖无运行时副作用的类型层，
 * 循环彻底断开。
 */

export interface SyncFieldInfo {
  id: string;
  name: string;
  fieldType: string;
}

export interface SyncFieldChangeItem {
  kind: 'type' | 'name' | 'sensitivity' | 'options' | 'metadata';
  payload?:
    | { oldType: string; newType: string }
    | { oldName: string; newName: string }
    | { oldLevel: string; newLevel: string }
    | { metadataKeys: string[] };
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
