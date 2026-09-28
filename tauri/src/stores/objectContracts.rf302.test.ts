import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import type { IpcCommands } from '@/lib/generated/ipcContracts';
import { searchCache } from '@/lib/searchCache';
import { useAuthStore } from './authStore';
import { useObjectStore } from './objectStore';
import { useSettingsStore, type CustomPage } from './settingsStore';
import { useTrashStore } from './trashStore';

type WireObject = NonNullable<IpcCommands['object_get']['result']>;
type WireSummary = IpcCommands['object_list']['result'][number];
type WireTrash = IpcCommands['object_trash_list']['result'][number];
type WireSync = IpcCommands['object_sync_with_template']['result'];

function object(overrides: Partial<WireObject> = {}): WireObject {
  return {
    id: 'object-a',
    accountId: 'acc-a',
    name: '合成对象',
    typeId: 'identity',
    properties: {},
    sensitivityLevel: 'internal',
    templateId: null,
    templateType: null,
    propertyLabels: null,
    createdAt: '2026-09-28T10:00:00Z',
    updatedAt: '2026-09-28T10:00:00Z',
    deletedAt: null,
    contractTypeId: null,
    templateHash: null,
    ignoredTemplateHash: null,
    ...overrides,
  };
}

function summary(overrides: Partial<WireSummary> = {}): WireSummary {
  return {
    id: 'object-a',
    name: '合成对象',
    typeId: 'identity',
    sectionType: 'identity',
    sensitivityLevel: 'internal',
    createdAt: '2026-09-28T10:00:00Z',
    updatedAt: '2026-09-28T10:00:00Z',
    isDeleted: false,
    templateId: null,
    templateType: null,
    iconName: 'document',
    properties: {},
    tags: [],
    hasAttachments: false,
    ...overrides,
  };
}

const syncResult: WireSync = {
  hasChanges: true,
  templateHash: 'synthetic-hash',
  fieldsAdded: [],
  fieldsDeprecated: [],
  fieldsUpdated: [
    {
      id: 'field',
      name: 'Field',
      fieldType: 'select',
      changes: [
        { kind: 'options' },
        { kind: 'sensitivity', payload: { oldLevel: 'public', newLevel: 'critical' } },
      ],
    },
  ],
  fieldsIncompatible: [],
};

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}

function login(id: string) {
  useAuthStore.getState().completeUnlock({ id, name: id });
}

beforeEach(() => {
  vi.restoreAllMocks();
  vi.mocked(invoke).mockReset().mockResolvedValue(null);
  useAuthStore.setState({ currentAccount: null, isAuthenticated: false });
  useObjectStore.getState().clearOnVaultLock();
  useTrashStore.getState().clearOnVaultLock();
  useSettingsStore.getState().clearOnVaultLock();
  login('acc-a');
});

afterEach(() => {
  useAuthStore.setState({ currentAccount: null, isAuthenticated: false });
  vi.restoreAllMocks();
});

describe('RF302 Store → typed IPC → 原生边界', () => {
  it('列表和详情保留标签原值、嵌套 JSON、tags 及未知模板类型', async () => {
    const labels = {
      publicSibling: 'public',
      invalid: 'not-a-level',
      explicitNull: null,
      nested: ['critical'],
    };
    const properties = { text: '你好 🌍', zero: 0, flag: false, nested: [{ value: 'kept' }] };
    const listed = summary({
      properties,
      propertyLabels: labels,
      tags: ['合成', 'tag'],
      templateType: 'future-kind',
    });
    const loaded = object({ properties, propertyLabels: labels, tags: ['合成', 'tag'] });
    vi.mocked(invoke).mockResolvedValueOnce([listed]).mockResolvedValueOnce(loaded);

    await useObjectStore
      .getState()
      .loadObjects('acc-a', { parentId: 'page-a', includeDeleted: false });
    await useObjectStore.getState().getObject('acc-a', 'object-a');

    expect(invoke).toHaveBeenNthCalledWith(1, 'object_list', {
      accountId: 'acc-a',
      filter: { parentId: 'page-a', includeDeleted: false },
    });
    expect(invoke).toHaveBeenNthCalledWith(2, 'object_get', {
      accountId: 'acc-a',
      objectId: 'object-a',
    });
    expect(useObjectStore.getState().objects[0]).toMatchObject({
      properties,
      propertyLabels: labels,
      tags: ['合成', 'tag'],
      templateType: 'future-kind',
    });
    const cached = useObjectStore.getState().currentObjectCache['object-a'];
    expect(cached.properties).toEqual(properties);
    expect(cached.propertyLabels).toEqual(labels);
    expect(Object.hasOwn(cached.propertyLabels!, 'explicitNull')).toBe(true);
    expect(cached.tags).toEqual(['合成', 'tag']);
    expect(cached.templateId).toBeUndefined();
    expect(cached.deletedAt).toBeUndefined();
    // View 转换不改写原 wire 的 null 或坏标签，后续共享敏感度策略仍可保守处理。
    expect(loaded.templateId).toBeNull();
    expect(loaded.propertyLabels).toEqual(labels);
  });

  it('create/update 传客户端 ID 和真实输入键，JSON 省略不修改编辑中的原值', async () => {
    const properties = {
      zero: 0,
      flag: false,
      empty: '',
      nil: null,
      omitted: undefined,
      nested: { omitted: undefined, keep: 'value' },
      entries: [undefined, null, false],
    };
    const sent = {
      zero: 0,
      flag: false,
      empty: '',
      nil: null,
      nested: { keep: 'value' },
      entries: [null, null, false],
    };
    vi.mocked(invoke)
      .mockResolvedValueOnce(object({ id: 'client-id', properties: sent }))
      .mockResolvedValueOnce(
        object({
          id: 'client-id',
          name: 'Edited',
          properties: sent,
          propertyLabels: { zero: null },
          tags: ['saved'],
        }),
      );
    const invalidate = vi.spyOn(searchCache, 'invalidateAccount');
    const created = await useObjectStore.getState().createObject({
      id: 'client-id',
      accountId: 'acc-a',
      name: 'Created',
      typeId: 'identity',
      parentId: 'page-a',
      iconName: 'star',
      templateId: null,
      templateType: null,
      properties,
    });
    await useObjectStore.getState().updateObject('client-id', {
      name: 'Edited',
      properties,
      sensitivityLevel: 'sensitive',
      iconName: 'book',
    });
    expect(invoke).toHaveBeenNthCalledWith(1, 'object_create', {
      input: {
        id: 'client-id',
        accountId: 'acc-a',
        name: 'Created',
        typeId: 'identity',
        parentId: 'page-a',
        iconName: 'star',
        templateId: null,
        templateType: null,
        properties: sent,
      },
    });
    expect(invoke).toHaveBeenNthCalledWith(2, 'object_update', {
      objectId: 'client-id',
      input: { name: 'Edited', properties: sent, sensitivityLevel: 'sensitive', iconName: 'book' },
    });
    expect(created.id).toBe('client-id');
    expect(useObjectStore.getState().currentObjectCache['client-id'].propertyLabels).toEqual({
      zero: null,
    });
    expect(useObjectStore.getState().objects[0]).toMatchObject({
      id: 'client-id',
      name: 'Edited',
      tags: ['saved'],
      propertyLabels: { zero: null },
    });
    expect(invalidate).toHaveBeenCalledTimes(2);
    expect(invalidate).toHaveBeenNthCalledWith(1, 'acc-a');
    expect(invalidate).toHaveBeenNthCalledWith(2, 'acc-a');
    expect(Object.hasOwn(properties, 'omitted')).toBe(true);
    expect(properties.entries[0]).toBeUndefined();
    expect(Object.hasOwn(properties.nested, 'omitted')).toBe(true);
  });

  it('非 JSON 编辑值在原生调用前失败，不能产生成功缓存或搜索失效', async () => {
    const invalidate = vi.spyOn(searchCache, 'invalidateAccount');
    await expect(
      useObjectStore.getState().createObject({
        accountId: 'acc-a',
        name: 'Invalid',
        typeId: 'identity',
        properties: { unsupported: 1n },
      }),
    ).rejects.toThrow();
    expect(invoke).not.toHaveBeenCalled();
    expect(useObjectStore.getState().objects).toEqual([]);
    expect(useObjectStore.getState().currentObjectCache).toEqual({});
    expect(useObjectStore.getState().isLoading).toBe(false);
    expect(useObjectStore.getState().error).not.toBeNull();
    expect(invalidate).not.toHaveBeenCalled();
  });

  it('模板 dryRun/应用/忽略/归档只发送真实参数，应用后仍读取当前对象', async () => {
    const deprecated = [
      {
        id: 'old',
        name: 'Old',
        fieldType: 'text',
        value: ['legacy', null],
        deprecatedAt: '2026',
        reason: 'removed',
      },
    ];
    vi.mocked(invoke).mockImplementation(async (cmd) => {
      if (cmd === 'object_sync_with_template') return syncResult;
      if (cmd === 'object_get') return object({ templateHash: 'synthetic-hash' });
      if (cmd === 'object_list_deprecated_fields') return deprecated;
      return null;
    });
    expect(await useObjectStore.getState().previewSyncTemplate('acc-a', 'object-a')).toEqual(
      syncResult,
    );
    expect(await useObjectStore.getState().applySyncTemplate('acc-a', 'object-a')).toEqual(
      syncResult,
    );
    expect(await useObjectStore.getState().loadDeprecatedFields('acc-a', 'object-a')).toEqual(
      deprecated,
    );
    await useObjectStore.getState().ignoreTemplateSync('object-a', 'synthetic-hash');
    expect(vi.mocked(invoke).mock.calls).toEqual([
      ['object_sync_with_template', { objectId: 'object-a', dryRun: true }],
      ['object_sync_with_template', { objectId: 'object-a', dryRun: false }],
      ['object_get', { accountId: 'acc-a', objectId: 'object-a' }],
      ['object_list_deprecated_fields', { objectId: 'object-a' }],
      ['object_ignore_template_sync', { objectId: 'object-a', hash: 'synthetic-hash' }],
    ]);
    expect(useObjectStore.getState().currentObjectCache['object-a'].templateHash).toBe(
      'synthetic-hash',
    );
  });

  it.each(['account', 'lock', 'reunlock'] as const)(
    '移除多余 accountId 后 %s 仍使旧模板应用失效且禁止后续读取',
    async (change) => {
      const completion = deferred<WireSync>();
      vi.mocked(invoke).mockReturnValueOnce(completion.promise);
      const invalidate = vi.spyOn(searchCache, 'invalidateAccount');
      const pending = useObjectStore.getState().applySyncTemplate('acc-a', 'object-a');
      const rejected = expect(pending).rejects.toThrow('expired session');
      try {
        expect(invoke).toHaveBeenCalledWith('object_sync_with_template', {
          objectId: 'object-a',
          dryRun: false,
        });
        if (change === 'account') login('acc-b');
        else {
          await useAuthStore.getState().lock();
          if (change === 'reunlock') login('acc-a');
        }
        completion.resolve(syncResult);
        await rejected;
        expect(vi.mocked(invoke).mock.calls.filter(([cmd]) => cmd === 'object_get')).toEqual([]);
        expect(useObjectStore.getState().currentObjectCache).toEqual({});
        expect(invalidate).not.toHaveBeenCalled();
      } finally {
        completion.resolve(syncResult);
        await pending.catch(() => {});
      }
    },
  );

  it('回收站列表接收真实 nullable 字段且保留绝对 since 与省略语义', async () => {
    const wire: WireTrash = {
      id: 'trash-a',
      itemType: 'object',
      originalId: 'object-a',
      name: '合成回收站',
      iconId: null,
      deletedAt: 1_790_000_000_000,
      expiresAt: null,
      originalParentId: null,
      originalSectionType: null,
      contractTypeId: null,
    };
    vi.mocked(invoke).mockResolvedValue([wire]);
    vi.spyOn(Date, 'now').mockReturnValue(1_800_000_000_000);
    await useTrashStore.getState().loadItems('acc-a');
    expect(invoke).toHaveBeenNthCalledWith(1, 'object_trash_list', { accountId: 'acc-a' });
    expect(useTrashStore.getState().items[0]).toMatchObject({
      id: 'trash-a',
      originalId: 'object-a',
      deletedAt: wire.deletedAt,
    });
    expect(useTrashStore.getState().items[0].iconId).toBeUndefined();
    useTrashStore.getState().setTimeFilter('1d');
    await useTrashStore.getState().loadItems('acc-a');
    expect(invoke).toHaveBeenNthCalledWith(2, 'object_trash_list', {
      accountId: 'acc-a',
      since: 1_800_000_000_000 - 86_400_000,
    });
    expect(wire.iconId).toBeNull();
  });

  it('旧页面迁移保持客户端 ID、已删行与元数据，未定义描述不变成 null', async () => {
    const legacy: CustomPage = {
      id: 'legacy-a',
      name: 'Legacy',
      iconId: 'star',
      createdAt: '2020',
      sortOrder: 2,
    };
    useSettingsStore.setState({ legacyCustomPages: [legacy] });
    vi.mocked(invoke).mockImplementation(async (cmd) => {
      if (cmd === 'object_list')
        return [
          summary({
            id: 'page-a',
            typeId: 'page',
            name: 'Stored',
            iconName: 'book',
            properties: { description: '说明', sortOrder: 0, legacyCreatedAt: '2019' },
          }),
          summary({
            id: 'deleted-a',
            typeId: 'page',
            name: 'Deleted',
            properties: null,
            isDeleted: true,
            updatedAt: '2024',
          }),
        ];
      if (cmd === 'object_create') return object({ id: 'legacy-a', typeId: 'page' });
      return null;
    });
    await useSettingsStore.getState().loadCustomPages('acc-a');
    expect(invoke).toHaveBeenCalledWith('object_list', {
      accountId: 'acc-a',
      filter: { typeId: 'page', includeDeleted: true },
    });
    expect(invoke).toHaveBeenCalledWith('object_create', {
      input: {
        id: 'legacy-a',
        accountId: 'acc-a',
        name: 'Legacy',
        typeId: 'page',
        iconName: 'star',
        properties: { legacyCreatedAt: '2020', sortOrder: 2 },
      },
    });
    const pages = useSettingsStore.getState().settings.customPages;
    expect(pages.find((page) => page.id === 'page-a')).toMatchObject({
      name: 'Stored',
      iconId: 'book',
      description: '说明',
      createdAt: '2019',
      sortOrder: 0,
    });
    expect(pages.find((page) => page.id === 'deleted-a')?.deletedAt).toBe('2024');
    expect(pages.find((page) => page.id === 'legacy-a')).toEqual(legacy);
    expect(useSettingsStore.getState().legacyCustomPages).toEqual([]);
  });

  it('新页面的乐观 ID 与 typed object_create payload 保持一致', async () => {
    vi.mocked(invoke).mockResolvedValue(object({ typeId: 'page' }));
    const page = await useSettingsStore.getState().addCustomPage('acc-a', 'New page', 'book');
    expect(invoke).toHaveBeenCalledTimes(1);
    expect(invoke).toHaveBeenCalledWith('object_create', {
      input: {
        id: page.id,
        accountId: 'acc-a',
        name: 'New page',
        typeId: 'page',
        iconName: 'book',
        properties: {},
      },
    });
    expect(useSettingsStore.getState().settings.customPages).toEqual([page]);
  });
});
