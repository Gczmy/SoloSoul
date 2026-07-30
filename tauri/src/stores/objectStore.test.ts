import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

// 模拟 @tauri-apps/api/core 的 invoke
const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

// 模拟 i18next
vi.mock('@/lib/i18n', () => ({
  default: { language: 'en' },
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
          collectionType: 'address',
          sensitivityLevel: 'public',
          createdAt: '2026-01-01',
          updatedAt: '2026-01-01',
        },
        {
          id: '2',
          name: 'Obj2',
          collectionType: 'address',
          sensitivityLevel: 'private',
          createdAt: '2026-01-02',
          updatedAt: '2026-01-02',
        },
      ];
      mockInvoke.mockResolvedValue(objects);

      const { useObjectStore } = await import('./objectStore');
      await useObjectStore.getState().loadObjects('acc-1', { collectionType: 'address' });

      expect(mockInvoke).toHaveBeenCalledWith('object_list', {
        account_id: 'acc-1',
        filter: { collectionType: 'address' },
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

      expect(mockInvoke).toHaveBeenCalledWith('object_list', { account_id: 'acc-1', filter: null });
    });
  });

  describe('getObject', () => {
    it('获取单个对象成功', async () => {
      const obj = {
        id: '1',
        accountId: 'acc-1',
        name: 'Obj1',
        collectionType: 'address',
        properties: { street: 'Main St' },
        sensitivityLevel: 'public',
        createdAt: '',
        updatedAt: '',
      };
      mockInvoke.mockResolvedValue(obj);

      const { useObjectStore } = await import('./objectStore');
      await useObjectStore.getState().getObject('acc-1', '1');

      expect(mockInvoke).toHaveBeenCalledWith('object_get', { account_id: 'acc-1', object_id: '1' });
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
        collectionType: 'address',
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
        collectionType: 'address',
        properties: {},
      });

      expect(mockInvoke).toHaveBeenCalledWith('object_create', {
        input: { accountId: 'acc-1', name: 'New', collectionType: 'address', properties: {} },
      });
      expect(result).toEqual(created);
      expect(useObjectStore.getState().objects).toContainEqual({
        id: 'new-1',
        name: 'New',
        collectionType: 'address',
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
          collectionType: 'address',
          properties: {},
        }),
      ).rejects.toThrow('Name required');

      expect(useObjectStore.getState().error).toBe('Error: Name required');
    });
  });

  describe('updateObject', () => {
    it('更新对象成功', async () => {
      const updated = {
        id: '1',
        accountId: 'acc-1',
        name: 'Updated',
        collectionType: 'address',
        properties: { street: 'New St' },
        sensitivityLevel: 'public',
        createdAt: '',
        updatedAt: '2026-02-01',
      };
      mockInvoke.mockResolvedValue(updated);

      const { useObjectStore } = await import('./objectStore');
      await useObjectStore
        .getState()
        .updateObject('1', { name: 'Updated', properties: { street: 'New St' } });

      expect(mockInvoke).toHaveBeenCalledWith('object_update', {
        object_id: '1',
        input: { name: 'Updated', properties: { street: 'New St' } },
      });
      expect(useObjectStore.getState().currentObjectCache['1']).toEqual(updated);
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
            collectionType: 'x',
            sensitivityLevel: 'public',
            createdAt: '',
            updatedAt: '',
          },
          {
            id: '2',
            name: 'Obj2',
            collectionType: 'x',
            sensitivityLevel: 'public',
            createdAt: '',
            updatedAt: '',
          },
        ],
      });
      await useObjectStore.getState().deleteObject('1');

      expect(mockInvoke).toHaveBeenCalledWith('object_delete', { object_id: '1' });
      expect(useObjectStore.getState().objects).toHaveLength(1);
      expect(useObjectStore.getState().objects[0].id).toBe('2');
    });
  });

  describe('trash lifecycle', () => {
    it('loadTrashObjects 加载回收站列表', async () => {
      const trash = [
        {
          id: 'del-1',
          name: 'Deleted',
          collectionType: 'x',
          sensitivityLevel: 'public',
          createdAt: '',
          updatedAt: '',
          isDeleted: true,
        },
      ];
      mockInvoke.mockResolvedValue(trash);

      const { useObjectStore } = await import('./objectStore');
      await useObjectStore.getState().loadTrashObjects('acc-1');

      expect(mockInvoke).toHaveBeenCalledWith('object_trash_list', { account_id: 'acc-1' });
      expect(useObjectStore.getState().trashObjects).toEqual(trash);
    });

    it('restoreObject 从回收站恢复并从 trashObjects 移除', async () => {
      mockInvoke.mockResolvedValue(undefined);

      const { useObjectStore } = await import('./objectStore');
      useObjectStore.setState({
        trashObjects: [
          {
            id: 'del-1',
            name: 'Del1',
            collectionType: 'x',
            sensitivityLevel: 'public',
            createdAt: '',
            updatedAt: '',
          },
        ],
      });
      await useObjectStore.getState().restoreObject('del-1');

      expect(mockInvoke).toHaveBeenCalledWith('object_restore', { objectId: 'del-1', lang: 'en' });
      expect(useObjectStore.getState().trashObjects).toHaveLength(0);
    });

    it('purgeObject 永久删除并从 trashObjects 移除', async () => {
      mockInvoke.mockResolvedValue(undefined);

      const { useObjectStore } = await import('./objectStore');
      useObjectStore.setState({
        trashObjects: [
          {
            id: 'del-1',
            name: 'Del1',
            collectionType: 'x',
            sensitivityLevel: 'public',
            createdAt: '',
            updatedAt: '',
          },
        ],
      });
      await useObjectStore.getState().purgeObject('del-1');

      expect(mockInvoke).toHaveBeenCalledWith('object_purge', { object_id: 'del-1' });
      expect(useObjectStore.getState().trashObjects).toHaveLength(0);
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
            collectionType: 'x',
            sensitivityLevel: 'public',
            createdAt: '',
            updatedAt: '',
          },
        ],
        trashObjects: [
          {
            id: '2',
            name: 'Y',
            collectionType: 'x',
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
            collectionType: 'x',
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
      expect(state.trashObjects).toHaveLength(0);
      expect(state.currentObjectCache).toEqual({});
      expect(state.error).toBeNull();
    });
  });
});
