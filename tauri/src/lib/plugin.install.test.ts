import { beforeEach, expect, it, vi } from 'vitest';
import { pluginCommands, type PluginInstallProgress } from './plugin';
import { invokeCommand } from './ipcClient';
const mocks = vi.hoisted(() => ({ close: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({
  Channel: vi.fn(),
  Resource: class {
    constructor(public rid: number) {}
    close = mocks.close;
  },
}));
vi.mock('./ipcClient', () => ({ invokeCommand: vi.fn() }));
beforeEach(() => {
  vi.resetAllMocks();
  mocks.close.mockResolvedValue(undefined);
});
it('cancels the native resource while keeping the task pending until native cleanup finishes', async () => {
  let reject!: (error: Error) => void;
  vi.mocked(invokeCommand).mockImplementation(async (cmd) => {
    if (cmd === 'create_plugin_install') return 42 as never;
    return new Promise((_, rej) => {
      reject = rej;
    }) as never;
  });
  const controller = new AbortController();
  const task = pluginCommands.install('plugin', '1.0.0', controller.signal);
  await vi.waitFor(() =>
    expect(invokeCommand).toHaveBeenCalledWith('plugin_install', {
      pluginId: 'plugin',
      version: '1.0.0',
      operationId: 42,
      onProgress: expect.anything(),
    }),
  );
  controller.abort();
  expect(mocks.close).toHaveBeenCalledOnce();
  reject(new Error('PLUGIN_INSTALL_CANCELLED'));
  await expect(task).rejects.toThrow('PLUGIN_INSTALL_CANCELLED');
  expect(mocks.close).toHaveBeenCalledOnce();
});

it('forwards native progress and ignores events after cancellation or completion', async () => {
  for (const cancel of [false, true]) {
    let finish!: () => void;
    let channel!: { onmessage: (event: PluginInstallProgress) => void };
    vi.mocked(invokeCommand).mockImplementation(async (cmd, args) => {
      if (cmd === 'create_plugin_install') return 42 as never;
      channel = args!.onProgress as typeof channel;
      return new Promise<void>((resolve) => {
        finish = resolve;
      }) as never;
    });
    const onProgress = vi.fn();
    const controller = new AbortController();
    const task = pluginCommands.update('plugin', controller.signal, onProgress);
    await vi.waitFor(() => expect(channel).toBeDefined());
    const event: PluginInstallProgress = {
      percent: 50,
      phase: 'downloading',
      downloadedBytes: 500,
      totalBytes: 1000,
    };
    channel.onmessage(event);
    expect(onProgress).toHaveBeenCalledWith(event);
    if (cancel) {
      controller.abort();
      channel.onmessage({ ...event, percent: 90 });
      expect(onProgress).toHaveBeenCalledTimes(1);
    }
    finish();
    await task;
    channel.onmessage({ ...event, percent: 100, phase: 'completed' });
    expect(onProgress).toHaveBeenCalledTimes(1);
  }
});
it('a pre-cancelled request never starts installation or creates a resource', async () => {
  const controller = new AbortController();
  controller.abort();
  await expect(pluginCommands.install('plugin', '1.0.0', controller.signal)).rejects.toMatchObject({
    name: 'AbortError',
  });
  expect(invokeCommand).not.toHaveBeenCalled();
});
