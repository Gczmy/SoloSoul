import { createSessionRequests, onRequestSessionChange } from '@/lib/sessionRequests';
import { create } from 'zustand';
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
  checkFieldUsage: (
    templateId: string,
    fieldKey: string,
  ) => Promise<{ active: number; softDeleted: number }>;
  /** A-001: 锁定 Vault 时清空解密模板数据（名称/自定义字段定义不残留内存）。 */
  clearOnVaultLock: () => void;
}

const requests = createSessionRequests();

export const useTemplateStore = create<TemplateState>((set, get) => ({
  templates: [],
  isLoading: false,
  error: null,

  clearOnVaultLock: () => {
    requests.invalidate();
    return set({ templates: [], isLoading: false, error: null });
  },

  async loadTemplates() {
    const request = requests.begin('templates');
    const setCurrent = request.guardSet<TemplateState>(set);
    setCurrent({ isLoading: true, error: null });
    try {
      const templates = await request.invoke<UserTemplate[]>('template_list');
      request.assertCurrent();
      setCurrent({ templates, isLoading: false });
    } catch (err) {
      if (!request.isCurrent()) return;
      setCurrent({ error: String(err), isLoading: false });
      throw err;
    }
  },

  async createTemplate(name, iconId, category, properties, contractTypeId) {
    const request = requests.begin();
    const id = await request.invoke<string>('template_create', {
      name,
      iconId: iconId,
      category,
      properties,
      contractTypeId: contractTypeId,
    });
    request.assertCurrent();
    await get().loadTemplates();
    request.assertCurrent();
    return id;
  },

  async updateTemplate(id, updates) {
    const request = requests.begin();
    await request.invoke('template_update', {
      templateId: id,
      name: updates.name,
      iconId: updates.iconId,
      category: updates.category,
      properties: updates.properties,
      contractTypeId: updates.contractTypeId,
    });
    request.assertCurrent();
    await get().loadTemplates();
    request.assertCurrent();
  },

  async deleteTemplate(id) {
    const request = requests.begin();
    const setCurrent = request.guardSet<TemplateState>(set);
    await request.invoke('template_delete', { templateId: id });
    request.assertCurrent();
    setCurrent((state) => ({
      templates: state.templates.filter((t) => t.id !== id),
    }));
  },

  async getTemplate(id) {
    const request = requests.begin();
    try {
      return await request.invoke<UserTemplate>('template_get', { templateId: id });
    } catch (err) {
      request.assertCurrent();
      // P126: 仅「模板不存在」返回 null（合法语义）；其余为真实后端异常（如
      // 无权访问、后端故障），抛出保留错误细节，不再与「不存在」混为一谈。
      const msg = typeof err === 'string' ? err : err instanceof Error ? err.message : String(err);
      if (msg.includes('模板不存在') || /not found/i.test(msg)) {
        return null;
      }
      throw err;
    }
  },

  async checkFieldUsage(templateId, fieldKey) {
    const request = requests.begin();
    return await request.invoke<{ active: number; softDeleted: number }>(
      'template_check_field_usage',
      {
        templateId: templateId,
        fieldKey: fieldKey,
      },
    );
  },
}));

onRequestSessionChange(() => useTemplateStore.getState().clearOnVaultLock());
