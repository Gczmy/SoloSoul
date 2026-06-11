import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import i18next from '@/lib/i18n';

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
  createdAt: string;
  updatedAt: string;
  deletedAt?: string;
  tags?: string[];
}

interface ObjectState {
  objects: ObjectSummary[];
  currentObject: ObjectData | null;
  isLoading: boolean;
  error: string | null;

  loadObjects: (accountId: string, filter?: { collectionType?: string; parentId?: string }) => Promise<void>;
  getObject: (accountId: string, objectId: string) => Promise<void>;
  createObject: (input: { accountId: string; name: string; collectionType: string; properties: Record<string, unknown>; parentId?: string; iconName?: string; templateId?: string; templateType?: 'system' | 'user' }) => Promise<ObjectData>;
  updateObject: (objectId: string, input: { name: string; properties: Record<string, unknown> }) => Promise<void>;
  deleteObject: (objectId: string) => Promise<void>;
  loadTrashObjects: (accountId: string) => Promise<void>;
  restoreObject: (objectId: string) => Promise<void>;
  purgeObject: (objectId: string) => Promise<void>;
  trashObjects: ObjectSummary[];
  clearOnVaultLock: () => void;
}

export const useObjectStore = create<ObjectState>((set) => ({
  objects: [],
  currentObject: null,
  isLoading: false,
  error: null,

  loadObjects: async (accountId, filter) => {
    set({ isLoading: true, error: null });
    try {
      const objects = await invoke<ObjectSummary[]>('object_list', {
        accountId,
        filter: filter || null,
      });
      set({ objects, isLoading: false });
    } catch (err) {
      set({ error: String(err), isLoading: false });
    }
  },

  getObject: async (accountId, objectId) => {
    set({ currentObject: null, isLoading: true, error: null });
    try {
      const obj = await invoke<ObjectData | null>('object_get', { accountId, objectId });
      set({ currentObject: obj, isLoading: false });
    } catch (err) {
      set({ error: String(err), isLoading: false });
    }
  },

  createObject: async (input) => {
    set({ isLoading: true, error: null });
    try {
      const obj = await invoke<ObjectData>('object_create', { input });
      set((s) => ({ objects: [...s.objects, {
        id: obj.id, name: obj.name, collectionType: obj.collectionType,
        sensitivityLevel: obj.sensitivityLevel,
        createdAt: obj.createdAt, updatedAt: obj.updatedAt,
      }], isLoading: false }));
      return obj;
    } catch (err) {
      set({ error: String(err), isLoading: false });
      throw err;
    }
  },

  updateObject: async (objectId, input) => {
    set({ isLoading: true, error: null });
    try {
      const obj = await invoke<ObjectData>('object_update', { objectId, input });
      set({ currentObject: obj, isLoading: false });
    } catch (err) {
      set({ error: String(err), isLoading: false });
    }
  },

  deleteObject: async (objectId) => {
    set({ isLoading: true, error: null });
    try {
      await invoke('object_delete', { objectId });
      set((s) => ({ objects: s.objects.filter((o) => o.id !== objectId), isLoading: false }));
    } catch (err) {
      set({ error: String(err), isLoading: false });
    }
  },

  trashObjects: [],

  loadTrashObjects: async (accountId) => {
    set({ isLoading: true, error: null });
    try {
      const items = await invoke<ObjectSummary[]>('object_trash_list', { accountId });
      set({ trashObjects: items, isLoading: false });
    } catch (err) {
      set({ error: String(err), isLoading: false });
    }
  },

  restoreObject: async (objectId) => {
    set({ isLoading: true, error: null });
    try {
      await invoke('object_restore', { objectId, lang: i18next.language });
      set((s) => ({
        trashObjects: s.trashObjects.filter((o) => o.id !== objectId),
        isLoading: false,
      }));
    } catch (err) {
      set({ error: String(err), isLoading: false });
    }
  },

  purgeObject: async (objectId) => {
    set({ isLoading: true, error: null });
    try {
      await invoke('object_purge', { objectId });
      set((s) => ({
        trashObjects: s.trashObjects.filter((o) => o.id !== objectId),
        isLoading: false,
      }));
    } catch (err) {
      set({ error: String(err), isLoading: false });
    }
  },

  clearOnVaultLock: () => set({ objects: [], trashObjects: [], currentObject: null, error: null }),
}));
