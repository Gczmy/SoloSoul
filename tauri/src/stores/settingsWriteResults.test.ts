import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ST_UI_PREFS } from '@/lib/constants';

const { ipc, changeLanguage } = vi.hoisted(() => ({
  ipc: vi.fn(),
  changeLanguage: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({ invoke: ipc }));
vi.mock('@/lib/theme', () => ({ applyTheme: vi.fn(async () => {}) }));
vi.mock('@/lib/i18n', () => ({
  default: { changeLanguage },
  detectSystemLanguage: () => 'en-US',
}));
vi.mock('@/lib/logger', () => ({
  logger: { warn: vi.fn(), error: vi.fn(), info: vi.fn(), debug: vi.fn() },
}));

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}

type PendingWrite = ReturnType<typeof deferred<unknown>> & {
  accountId: string;
  preferences: Record<string, unknown>;
};

let settings: typeof import('./settingsStore').useSettingsStore;
let auth: typeof import('./authStore').useAuthStore;
let writes: PendingWrite[];
let readPreferences: () => Promise<unknown>;
let writePlaintext: (key: string, value: unknown) => Promise<unknown>;

async function dispatched(index: number) {
  // 后一条写可以排队；只有前一条完成后才等待其真正派发，避免测试依赖旧并发实现。
  await vi.waitFor(() => expect(writes.length).toBeGreaterThan(index));
  return writes[index];
}

function plaintextWrites() {
  return ipc.mock.calls.filter(([command]) => command === 'ui_update_preference');
}

function isCurrent(result: unknown): boolean {
  expect(result).toMatchObject({ isCurrent: expect.any(Function) });
  return (result as { isCurrent: () => boolean }).isCurrent();
}

function expectInactive(result: unknown) {
  if (result && typeof result === 'object' && 'status' in result && result.status === 'stale')
    return;
  expect(isCurrent(result)).toBe(false);
}

beforeEach(async () => {
  vi.resetModules();
  ipc.mockReset();
  changeLanguage.mockReset().mockResolvedValue(undefined);
  localStorage.clear();
  writes = [];
  writePlaintext = () => Promise.resolve(undefined);
  readPreferences = () =>
    Promise.resolve({
      theme: 'light',
      accentColor: 'ocean',
      autoLockTimeoutMinutes: 5,
      language: 'en-US',
    });
  ipc.mockImplementation((command: string, args?: Record<string, unknown>) => {
    if (command === 'user_data_get_preferences') return readPreferences();
    if (command === 'ui_update_preference') return writePlaintext(String(args?.key), args?.value);
    if (command === 'user_data_update_preference') {
      const payload = args?.payload as {
        accountId: string;
        preferences: Record<string, unknown>;
      };
      const request = { ...deferred<unknown>(), ...payload };
      writes.push(request);
      return request.promise;
    }
    return Promise.resolve(undefined);
  });
  auth = (await import('./authStore')).useAuthStore;
  settings = (await import('./settingsStore')).useSettingsStore;
  auth.getState().completeUnlock({ id: 'acc-a', name: 'Account A' });
  await settings.getState().loadSettings('acc-a');
  localStorage.setItem(ST_UI_PREFS, JSON.stringify({ theme: 'light', accentColor: 'ocean' }));
  localStorage.setItem('i18nextLng', 'en-US');
  ipc.mockClear();
  changeLanguage.mockClear();
});

afterEach(() => {
  settings.getState().clearOnVaultLock();
  auth.setState({ isAuthenticated: false, currentAccount: null });
  localStorage.clear();
});

describe('RF111 真实 Settings Store 写入结果', () => {
  it('当前保存失败明确返回 failed，回滚并保持持久缓存不变', async () => {
    const cache = localStorage.getItem(ST_UI_PREFS);
    const saving = settings.getState().updateSetting('acc-a', 'theme', 'dark');
    const write = await dispatched(0);
    expect(settings.getState().settings.theme).toBe('dark');
    write.reject(new Error('Synthetic preference write failure'));

    const result = await saving;

    expect(settings.getState().settings.theme).toBe('light');
    expect(localStorage.getItem(ST_UI_PREFS)).toBe(cache);
    expect(plaintextWrites()).toEqual([]);
    expect(result).toMatchObject({ status: 'failed', isCurrent: expect.any(Function) });
  });

  it('A→B→C 两次失败回到最后确认的 A，而不是未保存的 B', async () => {
    const first = settings.getState().updateSetting('acc-a', 'autoLockTimeoutMinutes', 10);
    const firstWrite = await dispatched(0);
    const second = settings.getState().updateSetting('acc-a', 'autoLockTimeoutMinutes', 15);
    expect(settings.getState().settings.autoLockTimeoutMinutes).toBe(15);

    firstWrite.reject(new Error('Synthetic first write failure'));
    await first;
    const secondWrite = await dispatched(1);
    expect(settings.getState().settings.autoLockTimeoutMinutes).toBe(15);
    secondWrite.reject(new Error('Synthetic second write failure'));
    const result = await second;

    expect(settings.getState().settings.autoLockTimeoutMinutes).toBe(5);
    expect(result).toMatchObject({ status: 'failed', isCurrent: expect.any(Function) });
  });
  it('同键串行派发，B 成功后 C 失败回到已保存的 B', async () => {
    const first = settings.getState().updateSetting('acc-a', 'autoLockTimeoutMinutes', 10);
    const firstWrite = await dispatched(0);
    const second = settings.getState().updateSetting('acc-a', 'autoLockTimeoutMinutes', 15);
    await Promise.resolve();
    await Promise.resolve();
    const inFlightCount = writes.length;

    firstWrite.resolve(undefined);
    const firstResult = await first;
    const secondWrite = await dispatched(1);
    expect(settings.getState().settings.autoLockTimeoutMinutes).toBe(15);
    secondWrite.reject(new Error('Synthetic latest write failure'));
    const secondResult = await second;

    expect(inFlightCount).toBe(1);
    expect(writes.map((write) => write.preferences)).toEqual([
      { autoLockTimeoutMinutes: 10 },
      { autoLockTimeoutMinutes: 15 },
    ]);
    expectInactive(firstResult);
    expect(secondResult).toMatchObject({ status: 'failed' });
    expect(isCurrent(secondResult)).toBe(true);
    expect(settings.getState().settings.autoLockTimeoutMinutes).toBe(10);
  });

  it('迟到失败不撤销新的乐观值，后续成功成为新的回滚基线', async () => {
    const first = settings.getState().updateSetting('acc-a', 'autoLockTimeoutMinutes', 10);
    const firstWrite = await dispatched(0);
    const second = settings.getState().updateSetting('acc-a', 'autoLockTimeoutMinutes', 15);
    firstWrite.reject(new Error('Synthetic superseded failure'));
    const firstResult = await first;
    const valueAfterOldFailure = settings.getState().settings.autoLockTimeoutMinutes;
    (await dispatched(1)).resolve(undefined);
    const secondResult = await second;

    expectInactive(firstResult);
    expect(valueAfterOldFailure).toBe(15);
    expect(secondResult).toMatchObject({ status: 'saved' });
    expect(isCurrent(secondResult)).toBe(true);
    expect(settings.getState().settings.autoLockTimeoutMinutes).toBe(15);

    const third = settings.getState().updateSetting('acc-a', 'autoLockTimeoutMinutes', 20);
    (await dispatched(2)).reject(new Error('Synthetic later failure'));
    await third;
    expect(settings.getState().settings.autoLockTimeoutMinutes).toBe(15);
  });

  it('跨键成功只能缓存已确认值，不能带入另一个键的乐观状态', async () => {
    const theme = settings.getState().updateSetting('acc-a', 'theme', 'dark');
    const themeWrite = await dispatched(0);
    const accent = settings.getState().updateSetting('acc-a', 'accentColor', 'forest');
    // 不同键应独立执行，不等待尚未确认的 theme 写入。
    const accentWrite = await dispatched(1);
    accentWrite.resolve(undefined);
    const result = await accent;
    const cacheAfterAccent = JSON.parse(localStorage.getItem(ST_UI_PREFS)!);
    const plaintextAfterAccent = plaintextWrites();

    themeWrite.reject(new Error('Synthetic theme failure'));
    await theme;

    expect(result).toMatchObject({ status: 'saved' });
    expect(cacheAfterAccent).toMatchObject({ theme: 'light', accentColor: 'forest' });
    expect(plaintextAfterAccent).toEqual([
      ['ui_update_preference', { key: 'accentColor', value: 'forest' }],
    ]);
    expect(settings.getState().settings.theme).toBe('light');
    expect(settings.getState().settings.accentColor).toBe('forest');
    expect(JSON.parse(localStorage.getItem(ST_UI_PREFS)!)).toMatchObject({
      theme: 'light',
      accentColor: 'forest',
    });
  });

  it.each(['switch', 'lock'] as const)(
    '%s 后旧账户成功和失败结果均 stale，不写缓存或后续副作用',
    async (transition) => {
      for (const finish of ['resolve', 'reject'] as const) {
        auth.getState().completeUnlock({ id: 'acc-a', name: 'Account A' });
        const firstIndex = writes.length;
        const saving = settings.getState().updateSetting('acc-a', 'language', 'zh-CN');
        const write = await dispatched(firstIndex);
        if (transition === 'switch') {
          auth.getState().completeUnlock({ id: 'acc-b', name: 'Account B' });
          readPreferences = () => Promise.resolve({ language: 'en-US', theme: 'dark' });
          await settings.getState().loadSettings('acc-b');
        } else {
          await auth.getState().lock();
        }
        const before = settings.getState().settings;
        localStorage.setItem(ST_UI_PREFS, '{"theme":"light","accentColor":"amber"}');
        localStorage.setItem('i18nextLng', 'en-US');
        const commandCount = ipc.mock.calls.length;
        const languageCount = changeLanguage.mock.calls.length;

        if (finish === 'resolve') write.resolve(undefined);
        else write.reject(new Error('Synthetic expired write failure'));
        const result = await saving;

        expect(result).toEqual({ status: 'stale' });
        expect(settings.getState().settings).toEqual(before);
        expect(localStorage.getItem(ST_UI_PREFS)).toBe('{"theme":"light","accentColor":"amber"}');
        expect(localStorage.getItem('i18nextLng')).toBe('en-US');
        expect(ipc.mock.calls).toHaveLength(commandCount);
        expect(changeLanguage.mock.calls).toHaveLength(languageCount);
      }
    },
  );

  it('锁定使同键排队写入失效，旧在途调用完成后也不能再派发队列', async () => {
    const first = settings.getState().updateSetting('acc-a', 'theme', 'dark');
    const firstWrite = await dispatched(0);
    const queued = settings.getState().updateSetting('acc-a', 'theme', 'system');
    await auth.getState().lock();
    const cache = localStorage.getItem(ST_UI_PREFS);
    firstWrite.resolve(undefined);
    // 旧实现已经派发第二个请求；将其完成以取得真实行为断言，不能让旧代码假超时。
    for (const pending of writes.slice(1)) pending.resolve(undefined);
    const results = await Promise.all([first, queued]);

    expect(results).toEqual([{ status: 'stale' }, { status: 'stale' }]);
    expect(writes).toHaveLength(1);
    expect(plaintextWrites()).toEqual([]);
    expect(localStorage.getItem(ST_UI_PREFS)).toBe(cache);
  });

  it('已过期账户的新写入直接 stale，不改变当前值或发送 IPC', async () => {
    auth.getState().completeUnlock({ id: 'acc-b', name: 'Account B' });
    const before = settings.getState().settings;
    const result = await settings.getState().updateSetting('acc-a', 'theme', 'dark');

    expect(result).toEqual({ status: 'stale' });
    expect(settings.getState().settings).toEqual(before);
    expect(writes).toEqual([]);
    expect(plaintextWrites()).toEqual([]);
  });

  it('旧 loadSettings 不覆盖正在写入的字段，也不能污染失败回滚基线', async () => {
    const oldRead = deferred<unknown>();
    readPreferences = () => oldRead.promise;
    const loading = settings.getState().loadSettings('acc-a');
    await vi.waitFor(() =>
      expect(ipc).toHaveBeenCalledWith('user_data_get_preferences', { accountId: 'acc-a' }),
    );
    const saving = settings.getState().updateSetting('acc-a', 'autoLockTimeoutMinutes', 10);
    const write = await dispatched(0);
    oldRead.resolve({ autoLockTimeoutMinutes: 1 });
    await loading;
    const valueAfterOldLoad = settings.getState().settings.autoLockTimeoutMinutes;
    write.reject(new Error('Synthetic write failure after old read'));
    await saving;

    expect(valueAfterOldLoad).toBe(10);
    expect(settings.getState().settings.autoLockTimeoutMinutes).toBe(5);
  });

  it('旧 loadSettings 即使在写入成功之后返回，也不能撤销已经确认的新值', async () => {
    const oldRead = deferred<unknown>();
    readPreferences = () => oldRead.promise;
    const loading = settings.getState().loadSettings('acc-a');
    await vi.waitFor(() =>
      expect(ipc).toHaveBeenCalledWith('user_data_get_preferences', { accountId: 'acc-a' }),
    );
    const saving = settings.getState().updateSetting('acc-a', 'theme', 'dark');
    (await dispatched(0)).resolve(undefined);
    await saving;
    oldRead.resolve({ theme: 'light' });
    await loading;

    expect(settings.getState().settings.theme).toBe('dark');
    expect(JSON.parse(localStorage.getItem(ST_UI_PREFS)!)).toMatchObject({ theme: 'dark' });
    expect(
      plaintextWrites().some(([, args]) => args.key === 'theme' && args.value === 'light'),
    ).toBe(false);
  });

  it('语言数据库写成功后 changeLanguage 失败仍是 saved，不回滚持久值', async () => {
    changeLanguage.mockRejectedValueOnce(new Error('Synthetic language resource failure'));
    const saving = settings.getState().updateSetting('acc-a', 'language', 'zh-CN');
    (await dispatched(0)).resolve(undefined);
    const result = await saving;

    expect(result).toMatchObject({ status: 'saved' });
    expect(isCurrent(result)).toBe(true);
    expect(settings.getState().settings.language).toBe('zh-CN');
    expect(changeLanguage).toHaveBeenCalledExactlyOnceWith('zh-CN');
    expect(plaintextWrites()).toEqual([
      ['ui_update_preference', { key: 'language', value: 'zh-CN' }],
    ]);
  });

  it('结果守卫在同键新请求和会话失效后立即变为 false', async () => {
    const first = settings.getState().updateSetting('acc-a', 'theme', 'dark');
    (await dispatched(0)).resolve(undefined);
    const saved = await first;
    expect(saved).toMatchObject({ status: 'saved' });
    expect(isCurrent(saved)).toBe(true);

    const second = settings.getState().updateSetting('acc-a', 'theme', 'system');
    const savedStillCurrent = isCurrent(saved);
    (await dispatched(1)).reject(new Error('Synthetic latest failure'));
    const failed = await second;

    expect(savedStillCurrent).toBe(false);
    expect(failed).toMatchObject({ status: 'failed' });
    expect(isCurrent(failed)).toBe(true);
    await auth.getState().lock();
    expect(isCurrent(failed)).toBe(false);
  });
  it.each([false, true])(
    '明文 A 已发出且悬停时，新确认 B 排在后面；额外旧加载=%s 不再派发 A',
    async (queueAnotherLoad) => {
      const oldMirror = deferred<unknown>();
      let mirrorTheme = 'light';
      let firstThemeMirror = true;
      writePlaintext = (key, value) => {
        if (key !== 'theme') return Promise.resolve(undefined);
        if (firstThemeMirror) {
          firstThemeMirror = false;
          return oldMirror.promise.then(() => {
            mirrorTheme = String(value);
          });
        }
        mirrorTheme = String(value);
        return Promise.resolve(undefined);
      };
      readPreferences = () => Promise.resolve({ theme: 'light' });
      const firstLoad = settings.getState().loadSettings('acc-a');
      await vi.waitFor(() =>
        expect(ipc).toHaveBeenCalledWith('ui_update_preference', {
          key: 'theme',
          value: 'light',
        }),
      );
      let secondLoad: Promise<void> | undefined;
      if (queueAnotherLoad) {
        secondLoad = settings.getState().loadSettings('acc-a');
        await vi.waitFor(() => expect(settings.getState().isLoading).toBe(false));
      }
      const saving = settings.getState().updateSetting('acc-a', 'theme', 'dark');
      (await dispatched(0)).resolve(undefined);
      // 本地缓存已经确认数据库 B 成功；此时旧 A 的明文 IPC 仍未返回。
      await vi.waitFor(() =>
        expect(JSON.parse(localStorage.getItem(ST_UI_PREFS)!)).toMatchObject({ theme: 'dark' }),
      );
      const sentBeforeRelease = plaintextWrites()
        .filter(([, args]) => args.key === 'theme')
        .map(([, args]) => args.value);

      oldMirror.resolve(undefined);
      const [, , result] = await Promise.all([firstLoad, secondLoad, saving]);

      expect(sentBeforeRelease).toEqual(['light']);
      expect(result).toMatchObject({ status: 'saved' });
      expect(mirrorTheme).toBe('dark');
      expect(
        plaintextWrites()
          .filter(([, args]) => args.key === 'theme')
          .map(([, args]) => args.value),
      ).toEqual(['light', 'dark']);
      expect(settings.getState().settings.theme).toBe('dark');
    },
  );
});
