import { beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { useSettingsStore, type CustomPage } from './settingsStore';

describe('P046 稳定 ID 页面迁移', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    useSettingsStore.getState().clearOnVaultLock();
  });
  it('部分失败后重载补齐；已有新页共存，重试不重复创建且 ID 可直接用于导航', async () => {
    const legacy: CustomPage[] = ['a', 'b'].map((id, i) => ({
      id,
      name: id,
      iconId: 'star',
      description: `${id}-desc`,
      createdAt: '2020-01-01',
      sortOrder: i,
    }));
    let preferences = legacy;
    const records = new Map<string, Record<string, unknown>>([
      ['new', { id: 'new', name: 'New', createdAt: '2026-01-01' }],
    ]);
    let failB = true;
    const created: string[] = [];
    vi.mocked(invoke).mockImplementation(async (cmd, args) => {
      if (cmd === 'user_data_get_preferences') return { customPages: preferences };
      if (cmd === 'object_list') return [...records.values()];
      if (cmd === 'object_create') {
        const { input } = args as { input: Record<string, unknown> & { id: string } };
        if (input.id === 'b' && failB) throw new Error('temporary');
        if (records.has(input.id)) throw new Error('duplicate');
        records.set(input.id, { ...input, createdAt: '2026-09-17' });
        created.push(input.id);
      }
      if (cmd === 'user_data_update_preference')
        preferences = (args as { payload: { preferences: { customPages: CustomPage[] } } }).payload
          .preferences.customPages;
      return undefined;
    });
    await useSettingsStore.getState().loadSettings('acc-a');
    await useSettingsStore.getState().loadCustomPages('acc-a');
    expect(
      useSettingsStore
        .getState()
        .settings.customPages.map((p) => p.id)
        .sort(),
    ).toEqual(['a', 'new']);
    expect(preferences).toEqual(legacy);
    // 模拟重启：不依赖首轮内存残留，以 preferences 与真实已成功对象继续迁移。
    useSettingsStore.getState().clearOnVaultLock();
    failB = false;
    await useSettingsStore.getState().loadSettings('acc-a');
    await useSettingsStore.getState().loadCustomPages('acc-a');
    await useSettingsStore.getState().loadCustomPages('acc-a');
    expect(created).toEqual(['a', 'b']);
    expect(preferences).toEqual([]);
    const pages = useSettingsStore.getState().settings.customPages;
    expect(pages.map((p) => p.id).sort()).toEqual(['a', 'b', 'new']);
    for (const old of legacy) {
      expect(records.has(old.id)).toBe(true);
      expect(pages.find((p) => p.id === old.id)).toMatchObject(old);
    }
  });
  it('旧删除页不复活，已软删除的同 ID 页面不重复创建', async () => {
    const deleted = {
      id: 'deleted',
      name: 'Deleted',
      iconId: 'star',
      createdAt: '2020',
      sortOrder: 0,
      deletedAt: '2021',
    };
    useSettingsStore.setState({ legacyCustomPages: [deleted, { ...deleted, id: 'stored' }] });
    vi.mocked(invoke).mockImplementation(async (cmd) =>
      cmd === 'object_list'
        ? [{ id: 'stored', name: 'Stored', isDeleted: true, updatedAt: '2022' }]
        : undefined,
    );
    await useSettingsStore.getState().loadCustomPages('acc-a');
    expect(vi.mocked(invoke).mock.calls.some(([cmd]) => cmd === 'object_create')).toBe(false);
    expect(useSettingsStore.getState().settings.customPages.every((p) => p.deletedAt)).toBe(true);
    expect(useSettingsStore.getState().legacyCustomPages).toEqual([deleted]);
  });
});
