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

import { useObjectStore } from '@/stores/objectStore';
import { logger } from '@/lib/logger';

// 语义复核结果缓存，避免同一对象在短时间内触发多次 preview/apply IPC。
const semanticCheckCache = new Map<string, Promise<boolean>>();
const healedObjects = new Set<string>();

/** 仅用于测试：清空语义复核缓存。 */
export function __resetSemanticSyncCache() {
  semanticCheckCache.clear();
  healedObjects.clear();
}

/**
 * 模板同步语义复核：在 hash 初判命中后，先 dry-run 确认是否真的有字段变更。
 * - 有真实变更 → 返回 true（应显示提示条）。
 * - 无变更（仅哈希漂移）→ 调用非 dry-run 让后端刷新 templateHash 自愈，返回 false。
 * - 复核失败 → 保守返回 true，维持原有提示行为。
 */
export async function resolveSemanticNeedsSync(
  accountId: string,
  objectId: string,
): Promise<boolean> {
  const cacheKey = `${accountId}:${objectId}`;
  if (semanticCheckCache.has(cacheKey)) {
    return semanticCheckCache.get(cacheKey)!;
  }

  const promise = (async () => {
    try {
      const store = useObjectStore.getState();
      const preview = await store.previewSyncTemplate(accountId, objectId);
      if (preview.hasChanges) return true;
      // 无变更：apply 仅刷新 templateHash（后端无变更时不写快照/审计），
      // 并触发 getObject 缓存刷新，使列表中 obj.templateHash 更新、hash 初判归零。
      // 已经自愈过的对象在本次运行期内跳过重复 apply，降低写压力。
      if (!healedObjects.has(cacheKey)) {
        await store.applySyncTemplate(accountId, objectId);
        healedObjects.add(cacheKey);
      }
      return false;
    } catch (err) {
      logger.warn('[templateSync] semantic sync check failed:', err);
      return true;
    }
  })();

  semanticCheckCache.set(cacheKey, promise);
  return promise;
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
