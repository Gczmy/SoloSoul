import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';

export interface SystemTemplateSummary {
  key: string;
  category: string;
  icon: string;
  name: string;
  fieldCount: number;
  sensitiveFieldCount: number;
}

export interface SystemTemplateProperty {
  id: string;
  nameI18nKey: string;
  nameFallback: string;
  type: string;
  sensitive?: boolean;
  required?: boolean;
  options?: string[];
}

export interface SystemTemplate {
  key: string;
  category: string;
  icon: string;
  nameI18nKey: string;
  nameFallback: string;
  properties: SystemTemplateProperty[];
}

interface SystemTemplateState {
  templates: SystemTemplate[];
  loaded: boolean;
  isLoading: boolean;
  error: string | null;

  load: () => Promise<void>;
  getByKey: (key: string) => SystemTemplate | undefined;
  getByCategory: (category: string) => SystemTemplate[];
}

export const useSystemTemplateStore = create<SystemTemplateState>((set, get) => ({
  templates: [],
  loaded: false,
  isLoading: false,
  error: null,

  async load() {
    if (get().loaded) return;
    set({ isLoading: true, error: null });
    try {
      const templates = await invoke<SystemTemplate[]>('system_template_list');
      set({ templates, loaded: true, isLoading: false });
    } catch (err) {
      set({ error: String(err), isLoading: false });
    }
  },

  getByKey(key) {
    return get().templates.find((t) => t.key === key);
  },

  getByCategory(category) {
    return get().templates.filter((t) => t.category === category);
  },
}));
