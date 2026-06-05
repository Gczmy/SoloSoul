import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';

export interface ObjectSummary {
  id: string;
  name: string;
  collectionType: string;
  sensitivityLevel: string;
  createdAt: string;
  updatedAt: string;
}

export interface ObjectData {
  id: string;
  accountId: string;
  name: string;
  collectionType: string;
  properties: Record<string, unknown>;
  sensitivityLevel: string;
  createdAt: string;
  updatedAt: string;
  deletedAt?: string;
}

interface ObjectState {
  objects: ObjectSummary[];
  currentObject: ObjectData | null;
  isLoading: boolean;
  error: string | null;

  loadObjects: (accountId: string, filter?: { collectionType?: string }) => Promise<void>;
  getObject: (accountId: string, objectId: string) => Promise<void>;
  createObject: (input: { accountId: string; name: string; collectionType: string; properties: Record<string, unknown> }) => Promise<ObjectData>;
  updateObject: (objectId: string, input: { name: string; properties: Record<string, unknown> }) => Promise<void>;
  deleteObject: (objectId: string) => Promise<void>;
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
    set({ isLoading: true, error: null });
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
      await invoke('object_update', { objectId, input });
      set({ isLoading: false });
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

  clearOnVaultLock: () => set({ objects: [], currentObject: null, error: null }),
}));
