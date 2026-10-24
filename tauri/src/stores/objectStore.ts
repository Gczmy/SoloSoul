import { createSessionRequests, onRequestSessionChange } from '@/lib/sessionRequests';
import { create } from 'zustand';
import { normalizeObjectError, type BackendError } from '@/lib/backendErrorWire';
import type { IpcCommands } from '@/lib/generated/ipcContracts';
import {
  toJsonObject,
  toObjectDataView,
  toObjectSummaryView,
  type ObjectDataView as ObjectData,
  type ObjectSummaryView as ObjectSummary,
} from '@/lib/objectViewModel';
export type {
  ObjectDataView as ObjectData,
  ObjectSummaryView as ObjectSummary,
} from '@/lib/objectViewModel';
import { searchCache } from '@/lib/searchCache';
import { useAuthStore } from '@/stores/authStore';
// P228: 从 types/ 导入共享类型，断开 objectStore ↔ templateSync 循环依赖
// （templateSync 运行时引用 objectStore，objectStore 仅需类型层）。
import type { DeprecatedField, TemplateSyncResult } from '@/types/templateSync';

// P038: 对象写操作后按当前账户失效搜索缓存（30s TTL 内新建/编辑的对象立即可搜）。
function invalidateSearchCache() {
  const accountId = useAuthStore.getState().currentAccount?.id;
  if (accountId) {
    searchCache.invalidateAccount(accountId);
  }
}

type CreateObjectValues = Omit<IpcCommands['object_create']['args']['input'], 'properties'> & {
  properties: Record<string, unknown>;
};
type UpdateObjectValues = Omit<IpcCommands['object_update']['args']['input'], 'properties'> & {
  properties: Record<string, unknown>;
};

type ObjectReadOptions = { quiet: true; signal?: AbortSignal };

interface ObjectState {
  objects: ObjectSummary[];
  /** 对象缓存，以 objectId 为键。每个组件读取自己的缓存槽，避免全局 currentObject 竞态。 */
  currentObjectCache: Record<string, ObjectData>;
  isLoading: boolean;
  error: BackendError | null;

  loadObjects: (
    accountId: string,
    filter?: NonNullable<IpcCommands['object_list']['args']['filter']>,
  ) => Promise<void>;
  getObject: (
    accountId: string,
    objectId: string,
    options?: ObjectReadOptions,
  ) => Promise<ObjectData | null | undefined>;
  createObject: (input: CreateObjectValues) => Promise<ObjectData>;
  updateObject: (objectId: string, input: UpdateObjectValues) => Promise<ObjectData>;
  setObjectLocked: (objectId: string, locked: boolean) => Promise<ObjectData>;
  deleteObject: (objectId: string) => Promise<void>;
  /** 预览对象按当前模板同步后的变更（dryRun=true）。 */
  previewSyncTemplate: (accountId: string, objectId: string) => Promise<TemplateSyncResult>;
  /** 应用当前模板设置到对象。 */
  applySyncTemplate: (accountId: string, objectId: string) => Promise<TemplateSyncResult>;
  /** 忽略当前模板同步提示，将指纹持久化到对象。 */
  ignoreTemplateSync: (objectId: string, hash: string) => Promise<void>;
  /** 列出对象中已归档的历史字段。 */
  loadDeprecatedFields: (accountId: string, objectId: string) => Promise<DeprecatedField[]>;
  clearOnVaultLock: () => void;
}

const requests = createSessionRequests();

export const useObjectStore = create<ObjectState>((set) => {
  type Request = ReturnType<typeof requests.begin>;
  const loadingOwners = new Set<symbol>();
  const mutationTails = new Map<string, Promise<void>>();
  // loading 归属于会话中的操作，不能由读取的 latest-key 序号决定是否清理。
  const beginLoading = (request: Request) => {
    const token = Symbol();
    if (request.isCurrent()) {
      loadingOwners.add(token);
      set({ isLoading: true, error: null });
    }
    return () => {
      if (loadingOwners.delete(token) && request.isCurrent()) {
        set({ isLoading: loadingOwners.size > 0 });
      }
    };
  };
  const invalidateObjectReads = (objectId: string) => {
    requests.invalidate(`object:${objectId}`);
    requests.invalidate(`object-detail:${objectId}`);
  };
  // 回执只受会话保护；同一对象的写入按调用顺序发送和发布。
  const serializeMutation = async <T>(
    objectId: string,
    request: Request,
    operation: () => Promise<T>,
  ): Promise<T> => {
    const previous = mutationTails.get(objectId);
    let release!: () => void;
    const tail = new Promise<void>((resolve) => {
      release = resolve;
    });
    mutationTails.set(objectId, tail);
    try {
      if (previous) await previous;
      request.assertCurrent();
      return await operation();
    } finally {
      release();
      if (mutationTails.get(objectId) === tail) mutationTails.delete(objectId);
    }
  };
  return {
    objects: [],
    currentObjectCache: {},
    isLoading: false,
    error: null,

    loadObjects: async (accountId, filter) => {
      const request = requests.begin('objects', accountId);
      const setCurrent = request.guardSet<ObjectState>(set);
      const finishLoading = beginLoading(requests.begin(undefined, accountId));
      // 立即清空之前页面的陈旧对象，避免页面切换时闪烁旧卡片
      setCurrent({ objects: [] });
      try {
        const objects = await request.invokeTyped('object_list', {
          accountId: accountId,
          filter: filter || null,
        });
        request.assertCurrent();
        setCurrent({ objects: objects.map(toObjectSummaryView) });
      } catch (err) {
        if (!request.isCurrent()) return;
        setCurrent({ error: normalizeObjectError(err) });
      } finally {
        finishLoading();
      }
    },

    getObject: async (accountId, objectId, options) => {
      const request = requests.begin(
        options?.quiet ? `object-detail:${objectId}` : `object:${objectId}`,
        accountId,
      );
      const setCurrent = request.guardSet<ObjectState>(set);
      const finishLoading = options?.quiet
        ? () => {}
        : beginLoading(requests.begin(undefined, accountId));
      try {
        const wire = await request.invokeTyped('object_get', {
          accountId: accountId,
          objectId: objectId,
        });
        request.assertCurrent();
        if (options?.signal?.aborted) return;
        const obj = wire === null ? null : toObjectDataView(wire);
        setCurrent((s) => ({
          currentObjectCache: obj
            ? { ...s.currentObjectCache, [objectId]: obj }
            : s.currentObjectCache,
        }));
        // 调用方直接使用本次已校验的响应，不能从旧缓存推断此次读取成功。
        return obj;
      } catch (err) {
        if (!request.isCurrent() || options?.signal?.aborted) return;
        if (options?.quiet) throw err;
        setCurrent({ error: normalizeObjectError(err) });
      } finally {
        finishLoading();
      }
    },

    createObject: async (input) => {
      const request = requests.begin(undefined, input.accountId);
      const setCurrent = request.guardSet<ObjectState>(set);
      const finishLoading = beginLoading(request);
      try {
        const obj = toObjectDataView(
          await request.invokeTyped('object_create', {
            input: { ...input, properties: toJsonObject(input.properties) },
          }),
        );
        request.assertCurrent();
        setCurrent((s) => ({
          objects: [
            ...s.objects,
            {
              id: obj.id,
              name: obj.name,
              typeId: obj.typeId,
              sensitivityLevel: obj.sensitivityLevel,
              createdAt: obj.createdAt,
              updatedAt: obj.updatedAt,
              templateId: obj.templateId,
              templateType: obj.templateType,
              templateHash: obj.templateHash,
              contractTypeId: obj.contractTypeId,
            },
          ],
        }));
        // P038: 新建对象后立即失效搜索缓存
        invalidateSearchCache();
        return obj;
      } catch (err) {
        request.assertCurrent();
        setCurrent({ error: normalizeObjectError(err) });
        throw err;
      } finally {
        finishLoading();
      }
    },

    updateObject: async (objectId, input) => {
      const request = requests.begin();
      const setCurrent = request.guardSet<ObjectState>(set);
      const finishLoading = beginLoading(request);
      try {
        const payload = { ...input, properties: toJsonObject(input.properties) };
        return await serializeMutation(objectId, request, async () => {
          const obj = toObjectDataView(
            await request.invokeTyped('object_update', {
              objectId: objectId,
              input: payload,
            }),
          );
          request.assertCurrent();
          invalidateObjectReads(objectId);
          setCurrent((s) => ({
            currentObjectCache: { ...s.currentObjectCache, [objectId]: obj },
            // 同步更新摘要列表对应项，避免列表与详情缓存不一致（P057）。
            objects: s.objects.map((o) =>
              o.id === objectId
                ? {
                    ...o,
                    name: obj.name,
                    sensitivityLevel: obj.sensitivityLevel,
                    updatedAt: obj.updatedAt,
                    templateId: obj.templateId,
                    templateType: obj.templateType,
                    templateHash: obj.templateHash,
                    contractTypeId: obj.contractTypeId,
                    tags: obj.tags,
                    propertyLabels: obj.propertyLabels,
                  }
                : o,
            ),
          }));
          // P038: 编辑对象后立即失效搜索缓存
          invalidateSearchCache();
          return obj;
        });
      } catch (err) {
        request.assertCurrent();
        setCurrent({ error: normalizeObjectError(err) });
        // P002: 与 createObject 对齐——失败必须向上抛，调用方（useObjectEditorPage
        // handleSave）依赖异常进入 onError 分支；吞错会导致「保存失败被误报成功并
        // 退出页面」，编辑内容静默丢失。
        throw err;
      } finally {
        finishLoading();
      }
    },

    setObjectLocked: async (objectId, locked) => {
      const request = requests.begin();
      return serializeMutation(objectId, request, async () => {
        const obj = toObjectDataView(
          await request.invokeTyped('object_set_locked', { objectId, locked }),
        );
        request.assertCurrent();
        if (obj.id !== objectId || obj.accountId !== useAuthStore.getState().currentAccount?.id)
          throw new Error('Object response ownership mismatch');
        invalidateObjectReads(objectId);
        set((s) => ({
          currentObjectCache: { ...s.currentObjectCache, [objectId]: obj },
          objects: s.objects.map((item) =>
            item.id === objectId
              ? {
                  ...item,
                  updatedAt: obj.updatedAt,
                  properties: { ...item.properties, __editLocked: obj.properties.__editLocked },
                }
              : item,
          ),
        }));
        invalidateSearchCache();
        return obj;
      });
    },

    deleteObject: async (objectId) => {
      const request = requests.begin();
      const setCurrent = request.guardSet<ObjectState>(set);
      const finishLoading = beginLoading(request);
      try {
        return await serializeMutation(objectId, request, async () => {
          await request.invokeTyped('object_delete', { objectId: objectId });
          request.assertCurrent();
          invalidateObjectReads(objectId);
          setCurrent((s) => {
            // P043: 删除对象时同步清理详情缓存，避免残留旧数据被误读
            const nextCache = { ...s.currentObjectCache };
            delete nextCache[objectId];
            return {
              objects: s.objects.filter((o) => o.id !== objectId),
              currentObjectCache: nextCache,
            };
          });
          // P038: 删除对象后立即失效搜索缓存
          invalidateSearchCache();
        });
      } catch (err) {
        if (!request.isCurrent()) return;
        setCurrent({ error: normalizeObjectError(err) });
        throw err;
      } finally {
        finishLoading();
      }
    },

    previewSyncTemplate: async (accountId, objectId) => {
      const request = requests.begin(undefined, accountId);
      return request.invokeTyped('object_sync_with_template', {
        objectId: objectId,
        dryRun: true,
      });
    },

    applySyncTemplate: async (accountId, objectId) => {
      const request = requests.begin(undefined, accountId);
      return serializeMutation(objectId, request, async () => {
        const result = await request.invokeTyped('object_sync_with_template', {
          objectId: objectId,
          dryRun: false,
        });
        request.assertCurrent();
        invalidateObjectReads(objectId);
        // 同步成功后刷新该对象缓存，使 UI 立即反映最新字段与敏感度。
        request.assertCurrent();
        await useObjectStore.getState().getObject(accountId, objectId);
        request.assertCurrent();
        // P038: 模板同步改写对象内容后立即失效搜索缓存
        searchCache.invalidateAccount(accountId);
        return result;
      });
    },

    ignoreTemplateSync: async (objectId: string, hash: string) => {
      const request = requests.begin();
      return serializeMutation(objectId, request, async () => {
        await request.invokeTyped('object_ignore_template_sync', { objectId: objectId, hash });
        request.assertCurrent();
        invalidateObjectReads(objectId);
      });
    },

    loadDeprecatedFields: async (accountId, objectId) => {
      const request = requests.begin(undefined, accountId);
      return request.invokeTyped('object_list_deprecated_fields', {
        objectId: objectId,
      });
    },

    clearOnVaultLock: () => {
      requests.invalidate();
      loadingOwners.clear();
      mutationTails.clear();
      return set({ objects: [], currentObjectCache: {}, isLoading: false, error: null });
    },
  };
});

onRequestSessionChange(() => useObjectStore.getState().clearOnVaultLock());
