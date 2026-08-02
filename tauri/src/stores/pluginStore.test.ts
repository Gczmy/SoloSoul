import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({
  Channel: vi.fn(),
  invoke: vi.fn(),
}));

// 模拟 pluginCommands.run
const mockRun = vi.fn();
vi.mock('@/lib/plugin', () => ({
  pluginCommands: {
    listAll: vi.fn().mockResolvedValue([]),
    listInstalled: vi.fn().mockResolvedValue([]),
    install: vi.fn(),
    update: vi.fn(),
    uninstall: vi.fn(),
    run: mockRun,
    consentResponse: vi.fn(),
    dialogResponse: vi.fn(),
    listSessions: vi.fn(),
    auditLog: vi.fn(),
    updateRegistry: vi.fn(),
  },
}));

// 模拟 useUiStore.showToast
const toastCalls: Array<{ type: string; message: string; duration: number }> = [];
vi.mock('@/stores/uiStore', () => ({
  useUiStore: {
    getState: () => ({
      showToast: (toast: { type: string; message: string; duration: number }) => {
        toastCalls.push(toast);
      },
    }),
  },
}));

// 模拟 i18next
vi.mock('@/lib/i18n', () => ({
  default: {
    language: 'en',
    t: (_key: string, options: { defaultValue: string }) => options.defaultValue,
  },
}));

describe('pluginStore Toast behavior', () => {
  let storage: Record<string, string> = {};

  beforeEach(() => {
    storage = {};
    toastCalls.length = 0;
    mockRun.mockReset();
    vi.stubGlobal('localStorage', {
      getItem: (key: string) => storage[key] ?? null,
      setItem: (key: string, value: string) => {
        storage[key] = value;
      },
      removeItem: (key: string) => {
        delete storage[key];
      },
    });
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.resetModules();
  });

  it('场景1 — exitCode=0 时显示 "run_complete" Toast 并设置 toastShown', async () => {
    mockRun.mockResolvedValue({ exitCode: 0, logs: [], results: [], fuelConsumed: 100 });

    const { usePluginStore } = await import('./pluginStore');
    await usePluginStore.getState().runPlugin('addr-fmt', 'Address Formatter');

    // 验证 Toast：成功类型
    expect(toastCalls).toHaveLength(1);
    expect(toastCalls[0].type).toBe('success');
    expect(toastCalls[0].message).toContain('run completed');

    // 验证 toastShown 标记
    const plugin = usePluginStore.getState().runningPlugins['addr-fmt'];
    expect(plugin?.toastShown).toBe(true);
    expect(plugin?.completed).toBe(true);
    expect(plugin?.exitCode).toBe(0);
  });

  it('场景2 — exitCode=-1 时显示 "run_failed" Toast 并设置 toastShown', async () => {
    mockRun.mockResolvedValue({ exitCode: -1, logs: [], results: [], fuelConsumed: 50 });

    const { usePluginStore } = await import('./pluginStore');
    await usePluginStore.getState().runPlugin('addr-fmt', 'Address Formatter');

    // 验证 Toast：错误类型
    expect(toastCalls).toHaveLength(1);
    expect(toastCalls[0].type).toBe('error');
    expect(toastCalls[0].message).toContain('run failed');

    // 验证 toastShown + exitCode
    const plugin = usePluginStore.getState().runningPlugins['addr-fmt'];
    expect(plugin?.toastShown).toBe(true);
    expect(plugin?.completed).toBe(true);
    expect(plugin?.exitCode).toBe(-1);
  });

  it('场景3 — runPlugin 抛出异常时显示 "run_error" Toast 并设置 toastShown', async () => {
    mockRun.mockRejectedValue(new Error('Plugin crashed'));

    const { usePluginStore } = await import('./pluginStore');
    await usePluginStore.getState().runPlugin('addr-fmt', 'Address Formatter');

    // 验证 Toast：错误类型
    expect(toastCalls).toHaveLength(1);
    expect(toastCalls[0].type).toBe('error');
    expect(toastCalls[0].message).toContain('run error');

    // 验证 toastShown + error
    const plugin = usePluginStore.getState().runningPlugins['addr-fmt'];
    expect(plugin?.toastShown).toBe(true);
    expect(plugin?.completed).toBe(true);
    expect(plugin?.error).toContain('Plugin crashed');
  });

  it('场景4 — stopPlugin 设置 toastShown 但不触发 Toast', async () => {
    const { usePluginStore } = await import('./pluginStore');

    // 先设置一个正在运行的插件
    usePluginStore.setState({
      runningPlugins: {
        'addr-fmt': {
          pluginId: 'addr-fmt',
          pluginName: 'Address Formatter',
          startTime: Date.now(),
          logs: [],
          results: [],
          consentRequests: [],
          dialogRequests: [],
          completed: false,
        },
      },
    });

    // 停止插件
    usePluginStore.getState().stopPlugin('addr-fmt');

    // 验证：没有 Toast
    expect(toastCalls).toHaveLength(0);

    // 验证 toastShown 已设置
    const plugin = usePluginStore.getState().runningPlugins['addr-fmt'];
    expect(plugin?.toastShown).toBe(true);
    expect(plugin?.completed).toBe(true);
  });

  it('场景5 — 事件通道发送 "completed" 后最终状态仍包含 toastShown', async () => {
    // 模拟插件运行时通过事件通道发送 'completed' 事件
    mockRun.mockImplementation(
      async (
        _id: string,
        _params: Record<string, string>,
        onEvent: (event: { eventType: string; jsonData: string }) => void,
      ) => {
        // 模拟 WASM 完成后发送 completed 事件（不设置 toastShown）
        onEvent({ eventType: 'completed', jsonData: JSON.stringify({ exitCode: 0 }) });
        return { exitCode: 0, logs: [], results: [], fuelConsumed: 100 };
      },
    );

    const { usePluginStore } = await import('./pluginStore');
    await usePluginStore.getState().runPlugin('addr-fmt', 'Address Formatter');

    // 验证最终状态：toastShown 存在（由 runPlugin 的 post-await set() 确保）
    const plugin = usePluginStore.getState().runningPlugins['addr-fmt'];
    expect(plugin?.toastShown).toBe(true);
    expect(plugin?.completed).toBe(true);
    expect(plugin?.exitCode).toBe(0);
  });

  it('场景6 — exitCode=0 时无 error 不触发 "failed" Toast', async () => {
    mockRun.mockResolvedValue({ exitCode: 0, logs: [], results: [], fuelConsumed: 100 });

    const { usePluginStore } = await import('./pluginStore');
    await usePluginStore.getState().runPlugin('addr-fmt', 'Address Formatter');

    // 验证成功 Toast（非失败）
    expect(toastCalls).toHaveLength(1);
    expect(toastCalls[0].type).toBe('success');
  });

  it('场景7 — 只有一个 Toast（runPlugin 不触发重复 Toast）', async () => {
    mockRun.mockResolvedValue({ exitCode: 0, logs: [], results: [], fuelConsumed: 100 });

    const { usePluginStore } = await import('./pluginStore');
    await usePluginStore.getState().runPlugin('addr-fmt', 'Address Formatter');

    // 验证严格只有 1 个 Toast
    expect(toastCalls).toHaveLength(1);
  });

  it('P215 — 事件通道日志/结果环形截断到上限，避免 O(n²) 不可变累积', async () => {
    const { usePluginStore, MAX_PLUGIN_LOGS, MAX_PLUGIN_RESULTS } = await import('./pluginStore');
    mockRun.mockImplementation(
      async (
        _id: string,
        _params: Record<string, string>,
        onEvent: (event: { eventType: string; jsonData: string }) => void,
      ) => {
        // 一次性塞入远超上限的日志与结果事件
        for (let i = 0; i < MAX_PLUGIN_LOGS + 50; i++) {
          onEvent({
            eventType: 'log',
            jsonData: JSON.stringify({
              id: `log-${i}`,
              level: 'info',
              message: `line ${i}`,
              timestamp: Date.now(),
            }),
          });
        }
        for (let i = 0; i < MAX_PLUGIN_RESULTS + 20; i++) {
          onEvent({
            eventType: 'result',
            jsonData: JSON.stringify({ type: 'text', content: `result ${i}` }),
          });
        }
        onEvent({ eventType: 'completed', jsonData: JSON.stringify({ exitCode: 0 }) });
        return { exitCode: 0, logs: [], results: [], fuelConsumed: 100 };
      },
    );

    await usePluginStore.getState().runPlugin('addr-fmt', 'Address Formatter');

    const plugin = usePluginStore.getState().runningPlugins['addr-fmt'];
    expect(plugin?.logs.length).toBe(MAX_PLUGIN_LOGS);
    expect(plugin?.logs[0]?.id).toBe('log-50'); // 保留最新 MAX 条，丢弃最旧 50 条
    expect(plugin?.results.length).toBe(MAX_PLUGIN_RESULTS);
    expect(plugin?.results[0]).toEqual({ type: 'text', content: 'result 20' });
    expect(plugin?.completed).toBe(true);
  });
});
