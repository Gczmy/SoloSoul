import { beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { listen, type EventCallback } from '@tauri-apps/api/event';
import { useAuthStore } from './authStore';
import { useObjectStore } from './objectStore';
import { useTemplateStore } from './templateStore';
import { useTrashStore } from './trashStore';
import { useProfileStore } from './profileStore';
import { useSettingsStore } from './settingsStore';
import { useSyncStore } from './syncStore';
import { useLlmStatsStore } from './llmStatsStore';
import { usePluginStore } from './pluginStore';
import { pluginCommands } from '@/lib/plugin';

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}
function login(id: string) {
  useAuthStore.getState().completeUnlock({ id, name: id });
}

beforeEach(() => {
  vi.restoreAllMocks();
  vi.mocked(invoke).mockReset().mockResolvedValue(undefined);
  useAuthStore.setState({ currentAccount: null, isAuthenticated: false });
  login('acc-a');
});

const cases = [
  {
    name: 'objects',
    load: () => useObjectStore.getState().loadObjects('acc-a'),
    clear: () => useObjectStore.getState().clearOnVaultLock(),
    read: () => useObjectStore.getState().objects,
    value: [{ id: 'secret-a' }],
  },
  {
    name: 'templates',
    load: () => useTemplateStore.getState().loadTemplates(),
    clear: () => useTemplateStore.getState().clearOnVaultLock(),
    read: () => useTemplateStore.getState().templates,
    value: [{ id: 'secret-a' }],
  },
  {
    name: 'trash',
    load: () => useTrashStore.getState().loadItems('acc-a'),
    clear: () => useTrashStore.getState().clearOnVaultLock(),
    read: () => useTrashStore.getState().items,
    value: [{ id: 'secret-a' }],
  },
  {
    name: 'profile',
    load: () => useProfileStore.getState().loadProfile('acc-a'),
    clear: () => useProfileStore.getState().clear(),
    read: () => useProfileStore.getState().sections,
    value: {
      accountId: 'acc-a',
      data: Array.from(
        new TextEncoder().encode(JSON.stringify({ sections: [{ type: 'secret-a', fields: [] }] })),
      ),
    },
  },
  {
    name: 'settings',
    load: () => useSettingsStore.getState().loadSettings('acc-a'),
    clear: () => useSettingsStore.getState().clearOnVaultLock(),
    read: () => useSettingsStore.getState().settings.customPages,
    value: {
      customPages: [
        { id: 'secret-a', name: 'secret-a', iconId: 'star', createdAt: '2020', sortOrder: 0 },
      ],
    },
  },
  {
    name: 'sync',
    load: () => useSyncStore.getState().loadConflicts(),
    clear: () => useSyncStore.getState().clearOnVaultLock(),
    read: () => useSyncStore.getState().conflicts,
    value: [{ id: 'secret-a' }],
  },
  {
    name: 'llm stats',
    load: () => useLlmStatsStore.getState().loadStats('acc-a'),
    clear: () => useLlmStatsStore.getState().clear(),
    read: () => useLlmStatsStore.getState().stats,
    value: { secret: 'secret-a' },
  },
];

describe('P039 会话隔离', () => {
  for (const test of cases) {
    it.each(['resolve', 'reject'] as const)(
      `${test.name}: clear 后迟到 %s 不回填`,
      async (finish) => {
        const old = deferred<unknown>();
        vi.mocked(invoke).mockReturnValueOnce(old.promise);
        const work = test.load();
        test.clear();
        const empty = test.read();
        if (finish === 'resolve') old.resolve(test.value);
        else old.reject(new Error('secret-a'));
        await work;
        expect(test.read()).toEqual(empty);
        expect(invoke).toHaveBeenCalledTimes(1);
      },
    );
  }
  it('A → 锁定 → B 自动清除缓存，A 的对象详情与失败不污染 B', async () => {
    const old = deferred<unknown>();
    vi.mocked(invoke).mockReturnValueOnce(old.promise);
    const pending = useObjectStore.getState().getObject('acc-a', 'same-id');
    await useAuthStore.getState().lock();
    login('acc-b');
    vi.mocked(invoke).mockResolvedValueOnce({ id: 'same-id', name: 'B' });
    await useObjectStore.getState().getObject('acc-b', 'same-id');
    old.resolve({ id: 'same-id', name: 'secret-a' });
    await pending;
    expect(useObjectStore.getState().currentObjectCache['same-id'].name).toBe('B');
    expect(useObjectStore.getState().error).toBeNull();
  });
  it('同一列表乱序响应只保留最新请求', async () => {
    const old = deferred<unknown>();
    vi.mocked(invoke)
      .mockReturnValueOnce(old.promise)
      .mockResolvedValueOnce([{ id: 'new-page' }]);
    const first = useObjectStore.getState().loadObjects('acc-a', { typeId: 'old' });
    await useObjectStore.getState().loadObjects('acc-a', { typeId: 'new' });
    old.resolve([{ id: 'old-page' }]);
    await first;
    expect(useObjectStore.getState().objects).toEqual([{ id: 'new-page' }]);
  });
  it('重置统计后旧读取不能恢复统计或留下 loading', async () => {
    const old = deferred<unknown>();
    vi.mocked(invoke).mockReturnValueOnce(old.promise);
    const reading = useLlmStatsStore.getState().loadStats('acc-a');
    await useLlmStatsStore.getState().resetStats('acc-a');
    old.resolve({ secret: 'old-stats' });
    await reading;
    expect(useLlmStatsStore.getState().stats).toBeNull();
    expect(useLlmStatsStore.getState().loading).toBe(false);
  });
  it('旧账户保存失败不能回滚 B 的设置', async () => {
    const old = deferred<unknown>();
    useSettingsStore.setState({
      settings: { ...useSettingsStore.getState().settings, confirmDelete: true },
    });
    vi.mocked(invoke).mockReturnValueOnce(old.promise);
    const saving = useSettingsStore.getState().updateSetting('acc-a', 'confirmDelete', false);
    login('acc-b');
    useSettingsStore.setState({
      settings: { ...useSettingsStore.getState().settings, confirmDelete: false },
    });
    old.reject(new Error('old save failed'));
    await saving;
    expect(useSettingsStore.getState().settings.confirmDelete).toBe(false);
  });
  it('旧账户的新调用被拒绝，也不能取消 B 正在读取的列表', async () => {
    login('acc-b');
    const current = deferred<unknown>();
    vi.mocked(invoke).mockReturnValueOnce(current.promise);
    const reading = useObjectStore.getState().loadObjects('acc-b');
    await useObjectStore.getState().loadObjects('acc-a');
    current.resolve([{ id: 'B' }]);
    await reading;
    expect(useObjectStore.getState().objects).toEqual([{ id: 'B' }]);
    expect(invoke).toHaveBeenCalledTimes(1);
  });
  it('旧模板写入结束后不能开始新会话的刷新', async () => {
    const old = deferred<unknown>();
    vi.mocked(invoke).mockReturnValueOnce(old.promise);
    const work = useTemplateStore.getState().createTemplate('A', undefined, undefined, []);
    const rejected = expect(work).rejects.toThrow('expired session');
    login('acc-b');
    old.resolve('template-a');
    await rejected;
    expect(invoke).toHaveBeenCalledTimes(1);
  });
  it.each(['account', 'rerun'] as const)('插件 %s 后旧事件与结果不能覆盖新运行', async (change) => {
    const old = deferred<Awaited<ReturnType<typeof pluginCommands.run>>>();
    const next = deferred<Awaited<ReturnType<typeof pluginCommands.run>>>();
    type EventCallback = NonNullable<Parameters<typeof pluginCommands.run>[2]>;
    let oldEvent!: EventCallback;
    let newEvent!: EventCallback;
    vi.spyOn(pluginCommands, 'run')
      .mockImplementationOnce((_id, _params, onEvent) => {
        oldEvent = onEvent!;
        return old.promise;
      })
      .mockImplementationOnce((_id, _params, onEvent) => {
        newEvent = onEvent!;
        return next.promise;
      });
    const first = usePluginStore.getState().runPlugin('plugin', 'First');
    oldEvent({ eventType: 'log', jsonData: 'secret-a' });
    if (change === 'account') {
      const market = usePluginStore.getState().marketPlugins;
      const installed = usePluginStore.getState().installedPlugins;
      login('acc-b');
      expect(usePluginStore.getState().runningPlugins).toEqual({});
      expect(usePluginStore.getState().marketPlugins).toBe(market);
      expect(usePluginStore.getState().installedPlugins).toBe(installed);
    }
    const second = usePluginStore.getState().runPlugin('plugin', 'Second');
    oldEvent({
      eventType: 'result',
      jsonData: JSON.stringify({ type: 'text', content: 'secret-a' }),
    });
    newEvent({ eventType: 'result', jsonData: JSON.stringify({ type: 'text', content: 'B' }) });
    old.resolve({ exitCode: 0, fuelConsumed: 0, logs: [], results: [] });
    await first;
    expect(usePluginStore.getState().runningPlugins.plugin.completed).toBe(false);
    expect(JSON.stringify(usePluginStore.getState().runningPlugins)).not.toContain('secret-a');
    next.resolve({ exitCode: 0, fuelConsumed: 0, logs: [], results: [] });
    await second;
    expect(usePluginStore.getState().runningPlugins.plugin.completed).toBe(true);
    await useAuthStore.getState().lock();
    expect(usePluginStore.getState().runningPlugins).toEqual({});
  });
  it('清理同步 Store 后旧事件与迟到监听注册不再有效', async () => {
    let emit!: EventCallback<unknown>;
    const dispose = vi.fn();
    vi.mocked(listen).mockImplementationOnce(async (_event, handler) => {
      emit = handler;
      return dispose;
    });
    await useSyncStore.getState().initSyncCompletedListener();
    useSyncStore.getState().clearOnVaultLock();
    emit({ event: 'sync-completed', id: 1, payload: { peerNodeId: 'secret-a', applied: 1 } });
    expect(useSyncStore.getState().lastResult).toBeNull();
    expect(invoke).not.toHaveBeenCalled();
    const registration = deferred<() => void>();
    vi.mocked(listen).mockReturnValueOnce(registration.promise);
    const pending = useSyncStore.getState().initConflictListener();
    useSyncStore.getState().clearOnVaultLock();
    registration.resolve(dispose);
    await pending;
    expect(dispose).toHaveBeenCalledTimes(1);
  });
  it('锁定会废弃排队的同步开关任务和后续刷新', async () => {
    const old = deferred<unknown>();
    vi.mocked(invoke).mockReturnValueOnce(old.promise);
    const first = useSyncStore.getState().enable(true);
    await Promise.resolve();
    const second = useSyncStore.getState().enable(false);
    useSyncStore.getState().clearOnVaultLock();
    old.resolve(undefined);
    await Promise.all([first, second]);
    expect(invoke).toHaveBeenCalledTimes(1);
    expect(useSyncStore.getState().isLoading).toBe(false);
  });
});
