import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({
  Channel: vi.fn(),
  invoke: vi.fn(),
}));

describe('pluginStore persistence', () => {
  let storage: Record<string, string> = {};

  beforeEach(() => {
    storage = {};
    vi.stubGlobal(
      'localStorage',
      {
        getItem: (key: string) => storage[key] ?? null,
        setItem: (key: string, value: string) => {
          storage[key] = value;
        },
        removeItem: (key: string) => {
          delete storage[key];
        },
      },
    );
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('persists running plugins to localStorage', async () => {
    const { usePluginStore } = await import('./pluginStore');

    usePluginStore.setState({
      runningPlugins: {
        'test-plugin': {
          pluginId: 'test-plugin',
          pluginName: 'Test Plugin',
          startTime: 123456789,
          logs: [],
          results: [{ type: 'text', content: 'hello' }],
          consentRequests: [],
          dialogRequests: [],
          completed: false,
        },
      },
    });

    // 等待 zustand persist 写入 microtask
    await new Promise((resolve) => setTimeout(resolve, 0));

    const raw = storage['solosoul-plugin-store'];
    expect(raw).toBeDefined();
    const parsed = JSON.parse(raw);
    expect(parsed.state.runningPlugins['test-plugin']).toMatchObject({
      pluginId: 'test-plugin',
      pluginName: 'Test Plugin',
      completed: false,
    });
  });

  it('rehydrates running plugins from localStorage', async () => {
    storage['solosoul-plugin-store'] = JSON.stringify({
      state: {
        runningPlugins: {
          'restored-plugin': {
            pluginId: 'restored-plugin',
            pluginName: 'Restored Plugin',
            startTime: 987654321,
            logs: [{ id: '1', level: 'info', message: 'ready', timestamp: 1 }],
            results: [],
            consentRequests: [],
            dialogRequests: [],
            completed: true,
            exitCode: 0,
          },
        },
      },
      version: 0,
    });

    vi.resetModules();
    const { usePluginStore } = await import('./pluginStore');
    // 等待 rehydrate
    await new Promise((resolve) => setTimeout(resolve, 0));

    const state = usePluginStore.getState();
    expect(state.runningPlugins['restored-plugin']).toMatchObject({
      pluginId: 'restored-plugin',
      pluginName: 'Restored Plugin',
      completed: true,
      exitCode: 0,
    });
  });
});
