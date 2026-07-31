import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import type { DeprecatedField, TemplateSyncResult } from '@/lib/templateSync';

export interface ObjectSummary {
  id: string;
  name: string;
  collectionType: string;
  sectionType?: string; // §25.1.3 — page affiliation
  sensitivityLevel: string;
  createdAt: string;
  updatedAt: string;
  isDeleted?: boolean;
  properties?: Record<string, unknown>;
  tags?: string[];
  templateId?: string;
  templateType?: 'system' | 'user';
  /** 创建对象时模板的指纹；用于检测模板后续是否发生变更。 */
  templateHash?: string;
  /** 用户选择忽略同步时记录的模板指纹；持久化到后端。 */
  ignoredTemplateHash?: string;
  /** 插件合约类型 ID — 继承自模板的插件绑定标识。 */
  contractTypeId?: string;
  /** 字段级敏感度覆盖：fieldName -> sensitivityLevel。即使模板被删除，对象仍保留自己的敏感度副本。 */
  propertyLabels?: Record<string, string>;
}

export interface ObjectData {
  id: string;
  accountId: string;
  name: string;
  collectionType: string;
  properties: Record<string, unknown>;
  sensitivityLevel: string;
  templateId?: string;
  templateType?: 'system' | 'user';
  /** 创建对象时模板的指纹；用于检测模板后续是否发生变更。 */
  templateHash?: string;
  /** 用户选择忽略同步时记录的模板指纹；持久化到后端。 */
  ignoredTemplateHash?: string;
  createdAt: string;
  updatedAt: string;
  deletedAt?: string;
  tags?: string[];
  /** 插件合约类型 ID — 继承自模板的插件绑定标识。 */
  contractTypeId?: string;
  /** 字段级敏感度覆盖：fieldName -> sensitivityLevel。即使模板被删除，对象仍保留自己的敏感度副本。 */
  propertyLabels?: Record<string, string>;
}

interface ObjectState {
  objects: ObjectSummary[];
  /** 对象缓存，以 objectId 为键。每个组件读取自己的缓存槽，避免全局 currentObject 竞态。 */
  currentObjectCache: Record<string, ObjectData>;
  isLoading: boolean;
  error: string | null;

  loadObjects: (
    accountId: string,
    filter?: { collectionType?: string; parentId?: string },
  ) => Promise<void>;
  getObject: (accountId: string, objectId: string) => Promise<void>;
  createObject: (input: {
    accountId: string;
    name: string;
    collectionType: string;
    properties: Record<string, unknown>;
    parentId?: string;
    iconName?: string;
    templateId?: string;
    templateType?: 'system' | 'user';
  }) => Promise<ObjectData>;
  updateObject: (
    objectId: string,
    input: { name: string; properties: Record<string, unknown> },
  ) => Promise<void>;
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

export const useObjectStore = create<ObjectState>((set) => ({
  objects: [],
  currentObjectCache: {},
  isLoading: false,
  error: null,

  loadObjects: async (accountId, filter) => {
    // 立即清空之前页面的陈旧对象，避免页面切换时闪烁旧卡片
    set({ objects: [], isLoading: true, error: null });
    try {
      const objects = await invoke<ObjectSummary[]>('object_list', {
        accountId: accountId,
        filter: filter || null,
      });
      set({ objects, isLoading: false });
    } catch (err) {
      set({ error: String(err), isLoading: false });
    }
  },

  getObject: async (accountId, objectId) => {
    set({ isLoading: true, error: null });
    try {
      const obj = await invoke<ObjectData | null>('object_get', { accountId: accountId, objectId: objectId });
      set((s) => ({
        currentObjectCache: obj
          ? { ...s.currentObjectCache, [objectId]: obj }
          : s.currentObjectCache,
        isLoading: false,
      }));
    } catch (err) {
      set({ error: String(err), isLoading: false });
    }
  },

  createObject: async (input) => {
    set({ isLoading: true, error: null });
    try {
      const obj = await invoke<ObjectData>('object_create', { input });
      set((s) => ({
        objects: [
          ...s.objects,
          {
            id: obj.id,
            name: obj.name,
            collectionType: obj.collectionType,
            sensitivityLevel: obj.sensitivityLevel,
            createdAt: obj.createdAt,
            updatedAt: obj.updatedAt,
            templateId: obj.templateId,
            templateType: obj.templateType,
            templateHash: obj.templateHash,
            contractTypeId: obj.contractTypeId,
          },
        ],
        isLoading: false,
      }));
      return obj;
    } catch (err) {
      set({ error: String(err), isLoading: false });
      throw err;
    }
  },

  updateObject: async (objectId, input) => {
    set({ isLoading: true, error: null });
    try {
      const obj = await invoke<ObjectData>('object_update', { objectId: objectId, input });
      set((s) => ({
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
        isLoading: false,
      }));
    } catch (err) {
      set({ error: String(err), isLoading: false });
    }
  },

  deleteObject: async (objectId) => {
    set({ isLoading: true, error: null });
    try {
      await invoke('object_delete', { objectId: objectId });
      set((s) => ({
        objects: s.objects.filter((o) => o.id !== objectId),
        isLoading: false,
      }));
    } catch (err) {
      set({ error: String(err), isLoading: false });
    }
  },

  previewSyncTemplate: async (accountId, objectId) => {
    return invoke<TemplateSyncResult>('object_sync_with_template', {
      accountId,
      objectId: objectId,
      dryRun: true,
    });
  },

  applySyncTemplate: async (accountId, objectId) => {
    const result = await invoke<TemplateSyncResult>('object_sync_with_template', {
      accountId,
      objectId: objectId,
      dryRun: false,
    });
    // 同步成功后刷新该对象缓存，使 UI 立即反映最新字段与敏感度。
    await useObjectStore.getState().getObject(accountId, objectId);
    return result;
  },

  ignoreTemplateSync: async (objectId: string, hash: string) => {
    await invoke('object_ignore_template_sync', { objectId: objectId, hash });
  },

  loadDeprecatedFields: async (accountId, objectId) => {
    return invoke<DeprecatedField[]>('object_list_deprecated_fields', {
      accountId,
      objectId: objectId,
    });
  },

  clearOnVaultLock: () => set({ objects: [], currentObjectCache: {}, error: null }),
}));
