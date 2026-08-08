import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

// 模拟 @tauri-apps/api/core 的 invoke
const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

describe('objectStore', () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  afterEach(() => {
    vi.resetModules();
  });

  describe('loadObjects', () => {
    it('加载对象列表成功时设置 objects', async () => {
      const objects = [
        {
          id: '1',
          name: 'Obj1',
          typeId: 'address',
          sensitivityLevel: 'public',
          createdAt: '2026-01-01',
          updatedAt: '2026-01-01',
        },
        {
          id: '2',
          name: 'Obj2',
          typeId: 'address',
          sensitivityLevel: 'private',
          createdAt: '2026-01-02',
          updatedAt: '2026-01-02',
        },
      ];
      mockInvoke.mockResolvedValue(objects);

      const { useObjectStore } = await import('./objectStore');
      await useObjectStore.getState().loadObjects('acc-1', { typeId: 'address' });

      expect(mockInvoke).toHaveBeenCalledWith('object_list', {
        accountId: 'acc-1',
        filter: { typeId: 'address' },
      });
      expect(useObjectStore.getState().objects).toEqual(objects);
      expect(useObjectStore.getState().isLoading).toBe(false);
      expect(useObjectStore.getState().error).toBeNull();
    });

    it('加载失败时设置 error', async () => {
      mockInvoke.mockRejectedValue(new Error('DB error'));

      const { useObjectStore } = await import('./objectStore');
      await useObjectStore.getState().loadObjects('acc-1');

      expect(useObjectStore.getState().error).toBe('Error: DB error');
      expect(useObjectStore.getState().isLoading).toBe(false);
    });

    it('不传 filter 时发送 null', async () => {
      mockInvoke.mockResolvedValue([]);

      const { useObjectStore } = await import('./objectStore');
      await useObjectStore.getState().loadObjects('acc-1');

      expect(mockInvoke).toHaveBeenCalledWith('object_list', { accountId: 'acc-1', filter: null });
    });
  });

  describe('getObject', () => {
    it('获取单个对象成功', async () => {
      const obj = {
        id: '1',
        accountId: 'acc-1',
        name: 'Obj1',
        typeId: 'address',
        properties: { street: 'Main St' },
        sensitivityLevel: 'public',
        createdAt: '',
        updatedAt: '',
      };
      mockInvoke.mockResolvedValue(obj);

      const { useObjectStore } = await import('./objectStore');
      await useObjectStore.getState().getObject('acc-1', '1');

      expect(mockInvoke).toHaveBeenCalledWith('object_get', { accountId: 'acc-1', objectId: '1' });
      expect(useObjectStore.getState().currentObjectCache['1']).toEqual(obj);
    });

    it('获取失败时设置 error', async () => {
      mockInvoke.mockRejectedValue(new Error('Not found'));

      const { useObjectStore } = await import('./objectStore');
      await useObjectStore.getState().getObject('acc-1', '999');

      expect(useObjectStore.getState().error).toBe('Error: Not found');
      expect(useObjectStore.getState().currentObjectCache['999']).toBeUndefined();
    });
  });

  describe('createObject', () => {
    it('创建对象成功并追加到列表', async () => {
      const created = {
        id: 'new-1',
        accountId: 'acc-1',
        name: 'New',
        typeId: 'address',
        properties: {},
        sensitivityLevel: 'public',
        createdAt: '2026-01-01',
        updatedAt: '2026-01-01',
        contractTypeId: undefined,
      };
      mockInvoke.mockResolvedValue(created);

      const { useObjectStore } = await import('./objectStore');
      const result = await useObjectStore.getState().createObject({
        accountId: 'acc-1',
        name: 'New',
        typeId: 'address',
        properties: {},
      });

      expect(mockInvoke).toHaveBeenCalledWith('object_create', {
        input: { accountId: 'acc-1', name: 'New', typeId: 'address', properties: {} },
      });
      expect(result).toEqual(created);
      expect(useObjectStore.getState().objects).toContainEqual({
        id: 'new-1',
        name: 'New',
        typeId: 'address',
        sensitivityLevel: 'public',
        createdAt: '2026-01-01',
        updatedAt: '2026-01-01',
        contractTypeId: undefined,
      });
    });

    it('创建失败时抛出异常并设置 error', async () => {
      mockInvoke.mockRejectedValue(new Error('Name required'));

      const { useObjectStore } = await import('./objectStore');
      await expect(
        useObjectStore.getState().createObject({
          accountId: 'acc-1',
          name: '',
          typeId: 'address',
          properties: {},
        }),
      ).rejects.toThrow('Name required');

      expect(useObjectStore.getState().error).toBe('Error: Name required');
    });
  });

  describe('updateObject', () => {
    it('更新对象成功并同步缓存与摘要列表', async () => {
      const updated = {
        id: '1',
        accountId: 'acc-1',
        name: 'Updated',
        typeId: 'address',
        properties: { street: 'New St' },
        sensitivityLevel: 'public',
        createdAt: '',
        updatedAt: '2026-02-01',
      };
      mockInvoke.mockResolvedValue(updated);

      const { useObjectStore } = await import('./objectStore');
      useObjectStore.setState({
        objects: [
          {
            id: '1',
            name: 'Old',
            typeId: 'address',
            sensitivityLevel: 'public',
            createdAt: '',
            updatedAt: '2026-01-01',
          },
          {
            id: '2',
            name: 'Obj2',
            typeId: 'address',
            sensitivityLevel: 'public',
            createdAt: '',
            updatedAt: '2026-01-02',
          },
        ],
      });
      await useObjectStore
        .getState()
        .updateObject('1', { name: 'Updated', properties: { street: 'New St' } });

      expect(mockInvoke).toHaveBeenCalledWith('object_update', {
        objectId: '1',
        input: { name: 'Updated', properties: { street: 'New St' } },
      });
      expect(useObjectStore.getState().currentObjectCache['1']).toEqual(updated);
      // 摘要列表同步更新，非目标对象不受影响
      expect(useObjectStore.getState().objects[0]).toMatchObject({
        id: '1',
        name: 'Updated',
        updatedAt: '2026-02-01',
      });
      expect(useObjectStore.getState().objects[1].name).toBe('Obj2');
    });
  });

  describe('deleteObject', () => {
    it('删除对象成功并从列表移除', async () => {
      mockInvoke.mockResolvedValue(undefined);

      const { useObjectStore } = await import('./objectStore');
      // 先设置对象列表
      useObjectStore.setState({
        objects: [
          {
            id: '1',
            name: 'Obj1',
            typeId: 'x',
            sensitivityLevel: 'public',
            createdAt: '',
            updatedAt: '',
          },
          {
            id: '2',
            name: 'Obj2',
            typeId: 'x',
            sensitivityLevel: 'public',
            createdAt: '',
            updatedAt: '',
          },
        ],
      });
      await useObjectStore.getState().deleteObject('1');

      expect(mockInvoke).toHaveBeenCalledWith('object_delete', { objectId: '1' });
      expect(useObjectStore.getState().objects).toHaveLength(1);
      expect(useObjectStore.getState().objects[0].id).toBe('2');
    });
  });

  describe('clearOnVaultLock', () => {
    it('清空所有敏感状态', async () => {
      const { useObjectStore } = await import('./objectStore');
      useObjectStore.setState({
        objects: [
          {
            id: '1',
            name: 'X',
            typeId: 'x',
            sensitivityLevel: 'public',
            createdAt: '',
            updatedAt: '',
          },
        ],
        currentObjectCache: {
          '3': {
            id: '3',
            accountId: 'a',
            name: 'Z',
            typeId: 'x',
            properties: {},
            sensitivityLevel: 'public',
            createdAt: '',
            updatedAt: '',
          },
        },
        error: 'some error',
      });

      useObjectStore.getState().clearOnVaultLock();

      const state = useObjectStore.getState();
      expect(state.objects).toHaveLength(0);
      expect(state.currentObjectCache).toEqual({});
      expect(state.error).toBeNull();
    });
  });
});
