import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import type { UserTemplate, TemplateProperty } from '@/types/template';

interface TemplateState {
  templates: UserTemplate[];
  isLoading: boolean;
  error: string | null;

  loadTemplates: () => Promise<void>;
  createTemplate: (
    name: string,
    iconId: string | undefined,
    category: string | undefined,
    properties: TemplateProperty[],
    contractTypeId?: string,
  ) => Promise<string>;
  updateTemplate: (
    id: string,
    updates: Partial<
      Pick<UserTemplate, 'name' | 'iconId' | 'category' | 'properties' | 'contractTypeId'>
    >,
  ) => Promise<void>;
  deleteTemplate: (id: string) => Promise<void>;
  getTemplate: (id: string) => Promise<UserTemplate | null>;
  saveFromObject: (objectId: string, name: string) => Promise<string>;
  checkFieldUsage: (
    templateId: string,
    fieldKey: string,
  ) => Promise<{ active: number; softDeleted: number }>;
}

export const useTemplateStore = create<TemplateState>((set, get) => ({
  templates: [],
  isLoading: false,
  error: null,

  async loadTemplates() {
    set({ isLoading: true, error: null });
    try {
      const templates = await invoke<UserTemplate[]>('template_list');
      set({ templates, isLoading: false });
    } catch (err) {
      set({ error: String(err), isLoading: false });
      throw err;
    }
  },

  async createTemplate(name, iconId, category, properties, contractTypeId) {
    const id = await invoke<string>('template_create', {
      name,
      iconId: iconId,
      category,
      properties,
      contractTypeId: contractTypeId,
    });
    await get().loadTemplates();
    return id;
  },

  async updateTemplate(id, updates) {
    await invoke('template_update', {
      templateId: id,
      name: updates.name,
      iconId: updates.iconId,
      category: updates.category,
      properties: updates.properties,
      contractTypeId: updates.contractTypeId,
    });
    await get().loadTemplates();
  },

  async deleteTemplate(id) {
    await invoke('template_delete', { templateId: id });
    set((state) => ({
      templates: state.templates.filter((t) => t.id !== id),
    }));
  },

  async getTemplate(id) {
    try {
      return await invoke<UserTemplate>('template_get', { templateId: id });
    } catch {
      return null;
    }
  },

  async saveFromObject(objectId, name) {
    const id = await invoke<string>('template_save_from_object', {
      objectId: objectId,
      templateName: name,
      iconId: undefined,
    });
    await get().loadTemplates();
    return id;
  },

  async checkFieldUsage(templateId, fieldKey) {
    return await invoke<{ active: number; softDeleted: number }>('template_check_field_usage', {
      templateId: templateId,
      fieldKey: fieldKey,
    });
  },
}));
