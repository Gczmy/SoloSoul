import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

describe('templateStore', () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  afterEach(() => {
    vi.resetModules();
  });

  describe('loadTemplates', () => {
    it('成功加载模板列表', async () => {
      const templates = [
        {
          id: 't1',
          name: 'Address',
          iconId: 'map-pin',
          category: 'contact',
          properties: [],
          createdAt: '',
          updatedAt: '',
          accountId: 'acc-1',
        },
      ];
      mockInvoke.mockResolvedValue(templates);

      const { useTemplateStore } = await import('./templateStore');
      await useTemplateStore.getState().loadTemplates();

      expect(mockInvoke).toHaveBeenCalledWith('template_list');
      expect(useTemplateStore.getState().templates).toEqual(templates);
      expect(useTemplateStore.getState().isLoading).toBe(false);
    });

    it('加载失败时设置 error 并抛出', async () => {
      mockInvoke.mockRejectedValue(new Error('DB error'));

      const { useTemplateStore } = await import('./templateStore');
      await expect(useTemplateStore.getState().loadTemplates()).rejects.toThrow('DB error');
      expect(useTemplateStore.getState().error).toBe('Error: DB error');
      expect(useTemplateStore.getState().isLoading).toBe(false);
    });
  });

  describe('createTemplate', () => {
    it('创建成功后刷新列表并返回 id', async () => {
      mockInvoke.mockResolvedValueOnce('new-tpl-1'); // template_create
      mockInvoke.mockResolvedValueOnce([]); // template_list (刷新)

      const { useTemplateStore } = await import('./templateStore');
      const id = await useTemplateStore
        .getState()
        .createTemplate('My Template', 'star', 'general', [
          { id: 'field1', name: 'Field 1', type: 'text' },
        ]);

      expect(mockInvoke).toHaveBeenCalledWith('template_create', {
        name: 'My Template',
        iconId: 'star',
        category: 'general',
        properties: [{ id: 'field1', name: 'Field 1', type: 'text' }],
        contractTypeId: undefined,
      });
      expect(id).toBe('new-tpl-1');
    });

    it('创建时透传 contractTypeId', async () => {
      mockInvoke.mockResolvedValueOnce('tpl-ct');
      mockInvoke.mockResolvedValueOnce([]);

      const { useTemplateStore } = await import('./templateStore');
      await useTemplateStore.getState().createTemplate('C', 'icon', 'cat', [], 'ct-123');
      expect(mockInvoke).toHaveBeenCalledWith(
        'template_create',
        expect.objectContaining({
          contractTypeId: 'ct-123',
        }),
      );
    });
  });

  describe('updateTemplate', () => {
    it('更新成功后刷新列表', async () => {
      mockInvoke.mockResolvedValueOnce(undefined); // template_update
      mockInvoke.mockResolvedValueOnce([]); // template_list

      const { useTemplateStore } = await import('./templateStore');
      await useTemplateStore.getState().updateTemplate('t1', { name: 'Renamed' });

      expect(mockInvoke).toHaveBeenCalledWith('template_update', {
        templateId: 't1',
        name: 'Renamed',
        iconId: undefined,
        category: undefined,
        properties: undefined,
      });
    });
  });

  describe('deleteTemplate', () => {
    it('删除后从列表中移除', async () => {
      mockInvoke.mockResolvedValue(undefined);

      const { useTemplateStore } = await import('./templateStore');
      useTemplateStore.setState({
        templates: [
          {
            id: 't1',
            name: 'A',
            iconId: undefined,
            category: undefined,
            properties: [],
            createdAt: '',
            updatedAt: '',
            accountId: 'acc-1',
          },
          {
            id: 't2',
            name: 'B',
            iconId: undefined,
            category: undefined,
            properties: [],
            createdAt: '',
            updatedAt: '',
            accountId: 'acc-1',
          },
        ],
      });
      await useTemplateStore.getState().deleteTemplate('t1');

      expect(mockInvoke).toHaveBeenCalledWith('template_delete', { templateId: 't1' });
      expect(useTemplateStore.getState().templates).toHaveLength(1);
      expect(useTemplateStore.getState().templates[0].id).toBe('t2');
    });
  });

  describe('getTemplate', () => {
    it('成功时返回模板', async () => {
      const tpl = {
        id: 't1',
        name: 'T',
        iconId: undefined,
        category: undefined,
        properties: [],
        createdAt: '',
        updatedAt: '',
        accountId: 'acc-1',
      };
      mockInvoke.mockResolvedValue(tpl);

      const { useTemplateStore } = await import('./templateStore');
      const result = await useTemplateStore.getState().getTemplate('t1');
      expect(result).toEqual(tpl);
    });

    it('失败时返回 null', async () => {
      mockInvoke.mockRejectedValue(new Error('Not found'));
      const { useTemplateStore } = await import('./templateStore');
      const result = await useTemplateStore.getState().getTemplate('missing');
      expect(result).toBeNull();
    });

    it('P126: 真实后端异常抛出而非返回 null（与「模板不存在」区分）', async () => {
      mockInvoke.mockRejectedValue(new Error('database locked'));
      const { useTemplateStore } = await import('./templateStore');
      await expect(useTemplateStore.getState().getTemplate('t1')).rejects.toThrow('database locked');
    });
  });

  describe('checkFieldUsage', () => {
    it('返回字段使用统计', async () => {
      mockInvoke.mockResolvedValue({ active: 3, softDeleted: 1 });

      const { useTemplateStore } = await import('./templateStore');
      const result = await useTemplateStore.getState().checkFieldUsage('t1', 'field1');
      expect(mockInvoke).toHaveBeenCalledWith('template_check_field_usage', {
        templateId: 't1',
        fieldKey: 'field1',
      });
      expect(result).toEqual({ active: 3, softDeleted: 1 });
    });
  });

  describe('state management', () => {
    it('初始状态正确', async () => {
      const { useTemplateStore } = await import('./templateStore');
      const s = useTemplateStore.getState();
      expect(s.templates).toEqual([]);
      expect(s.isLoading).toBe(false);
      expect(s.error).toBeNull();
    });
  });
});
