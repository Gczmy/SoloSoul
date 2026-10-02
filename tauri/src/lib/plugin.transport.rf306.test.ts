import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { Channel, Resource, SERIALIZE_TO_IPC_FN } from '@tauri-apps/api/core';
import type {
  PluginInstallProgress,
  PluginInstallResult,
  PluginResult,
  PluginSession,
  PluginAuditEntry,
} from './generated/ipcContracts';
import { readBackendError, backendErrorLogDetails } from './backendErrorWire';
import { pluginCommands } from './plugin';
import { pluginEvent, pluginResult } from '@/test/pluginFixtures';
import { useAuthStore } from '@/stores/authStore';
import { usePluginStore } from '@/stores/pluginStore';
import { useUiStore } from '@/stores/uiStore';

// 保留锁定 SDK 的真实 Channel / Resource / invoke，仅替换 Webview 原生边界。
vi.unmock('@tauri-apps/api/core');

const nativeInvoke =
  vi.fn<(command: string, args?: Record<string, unknown>, options?: unknown) => Promise<unknown>>();
const callbacks = new Map<number, (message: unknown) => void>();
const indices = new Map<number, number>();
let nextCallback = 0;
const installed: PluginInstallResult = { pluginId: 'plugin', version: '1.0.0', installedAt: 1234 };
const progress: PluginInstallProgress = {
  phase: 'downloading',
  percent: 25,
  downloadedBytes: 10,
  totalBytes: null,
};

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: Error) => void;
  const promise = new Promise<T>((done, fail) => {
    resolve = done;
    reject = fail;
  });
  return { promise, resolve, reject };
}

function channelFor(command: string, argument: string, occurrence = 0): Channel<unknown> {
  const calls = nativeInvoke.mock.calls.filter(([name]) => name === command);
  const channel = calls[occurrence]?.[1]?.[argument];
  if (!(channel instanceof Channel)) throw new Error('Expected the actual SDK Channel instance');
  return channel;
}

function emit(channel: Channel<unknown>, message: unknown) {
  const callback = callbacks.get(channel.id);
  if (!callback) throw new Error('Native callback is not registered');
  const index = indices.get(channel.id) ?? 0;
  callback({ index, message });
  indices.set(channel.id, index + 1);
}

function end(channel: Channel<unknown>) {
  callbacks.get(channel.id)?.({ index: indices.get(channel.id) ?? 0, end: true });
}

function clearToasts() {
  for (const toast of useUiStore.getState().toasts) useUiStore.getState().dismissToast(toast.id);
}

beforeEach(() => {
  vi.restoreAllMocks();
  callbacks.clear();
  indices.clear();
  nextCallback = 0;
  nativeInvoke.mockReset().mockResolvedValue(null);
  vi.stubGlobal('__TAURI_INTERNALS__', {
    invoke: nativeInvoke,
    transformCallback: (callback: (message: unknown) => void) => {
      const id = ++nextCallback;
      callbacks.set(id, callback);
      return id;
    },
    unregisterCallback: (id: number) => {
      callbacks.delete(id);
    },
  });
  useAuthStore.setState({ currentAccount: null, isAuthenticated: false });
  usePluginStore.getState().clearOnVaultLock();
  useAuthStore.getState().completeUnlock({ id: 'account', name: 'Synthetic account' });
  clearToasts();
});

afterEach(() => {
  useAuthStore.setState({ currentAccount: null, isAuthenticated: false });
  clearToasts();
  callbacks.clear();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

describe('RF306 typed transport with the real Tauri SDK', () => {
  it('passes the same serializable Channel capability and numeric rid, then releases once', async () => {
    const pending = deferred<PluginInstallResult>();
    nativeInvoke.mockImplementation(async (command) => {
      if (command === 'create_plugin_install') return 42;
      if (command === 'plugin_install') return pending.promise;
      return null;
    });
    const close = vi.spyOn(Resource.prototype, 'close');
    const report = vi.fn();
    const task = pluginCommands.install('plugin', '1.0.0', undefined, report);
    try {
      await vi.waitFor(() =>
        expect(nativeInvoke).toHaveBeenCalledWith(
          'plugin_install',
          expect.objectContaining({ operationId: 42 }),
          undefined,
        ),
      );
      const channel = channelFor('plugin_install', 'onProgress');
      expect(channel[SERIALIZE_TO_IPC_FN]()).toBe(`__CHANNEL__:${channel.id}`);
      expect(nativeInvoke).toHaveBeenCalledWith(
        'plugin_install',
        {
          pluginId: 'plugin',
          version: '1.0.0',
          operationId: 42,
          onProgress: channel,
        },
        undefined,
      );
      emit(channel, progress);
      expect(report).toHaveBeenCalledTimes(1);
      expect(report).toHaveBeenCalledWith(progress);
      pending.resolve(installed);
      await expect(task).resolves.toEqual(installed);
      expect(close).toHaveBeenCalledOnce();
      expect(nativeInvoke).toHaveBeenCalledWith('plugin:resources|close', { rid: 42 }, undefined);
      emit(channel, { ...progress, percent: 100, phase: 'completed' });
      expect(report).toHaveBeenCalledTimes(1);
      end(channel);
      expect(callbacks.has(channel.id)).toBe(false);
    } finally {
      pending.resolve(installed);
      await task.catch(() => {});
    }
  });

  it('abort closes once but waits for native termination and close cleanup before settling', async () => {
    const pending = deferred<PluginInstallResult>();
    const closed = deferred<null>();
    nativeInvoke.mockImplementation(async (command) => {
      if (command === 'create_plugin_install') return 43;
      if (command === 'plugin_update') return pending.promise;
      if (command === 'plugin:resources|close') return closed.promise;
      return null;
    });
    const close = vi.spyOn(Resource.prototype, 'close');
    const controller = new AbortController();
    const report = vi.fn();
    const task = pluginCommands.update('plugin', controller.signal, report);
    let settled = false;
    const observed = task.then(
      () => {
        settled = true;
      },
      () => {
        settled = true;
      },
    );
    try {
      await vi.waitFor(() =>
        expect(nativeInvoke).toHaveBeenCalledWith(
          'plugin_update',
          expect.objectContaining({ operationId: 43 }),
          undefined,
        ),
      );
      const channel = channelFor('plugin_update', 'onProgress');
      emit(channel, progress);
      controller.abort();
      await Promise.resolve();
      expect(close).toHaveBeenCalledOnce();
      expect(settled).toBe(false);
      emit(channel, { ...progress, percent: 80 });
      expect(report).toHaveBeenCalledTimes(1);
      const cancelled = new Error('PLUGIN_INSTALL_CANCELLED');
      pending.reject(cancelled);
      await pending.promise.catch(() => {});
      expect(settled).toBe(false);
      closed.resolve(null);
      await expect(task).rejects.toMatchObject({ backend: { code: 'PLUGIN_INSTALL_CANCELLED' } });
      expect(close).toHaveBeenCalledOnce();
      end(channel);
    } finally {
      pending.resolve(installed);
      closed.resolve(null);
      await observed;
    }
  });

  it('abort while the resource is being created releases its eventual rid without installing', async () => {
    const created = deferred<number>();
    nativeInvoke.mockImplementation(async (command) =>
      command === 'create_plugin_install' ? created.promise : null,
    );
    const close = vi.spyOn(Resource.prototype, 'close');
    const controller = new AbortController();
    const task = pluginCommands.install('plugin', '1.0.0', controller.signal);
    const outcome = task.catch((error: unknown) => error);
    try {
      expect(nativeInvoke).toHaveBeenCalledWith('create_plugin_install', {}, undefined);
      controller.abort();
      created.resolve(44);
      expect(await outcome).toMatchObject({ name: 'AbortError' });
      expect(close).toHaveBeenCalledOnce();
      expect(nativeInvoke.mock.calls.map(([command]) => command)).toEqual([
        'create_plugin_install',
        'plugin:resources|close',
      ]);
      expect(nativeInvoke).toHaveBeenCalledWith('plugin:resources|close', { rid: 44 }, undefined);
    } finally {
      created.resolve(44);
      await outcome;
    }
  });

  it('update preserves the native error when best-effort Resource.close also fails', async () => {
    const failure = new Error('synthetic install failure');
    nativeInvoke.mockImplementation(async (command) => {
      if (command === 'create_plugin_install') return 45;
      if (command === 'plugin_update') throw failure;
      if (command === 'plugin:resources|close') throw new Error('synthetic close failure');
      return null;
    });
    const close = vi.spyOn(Resource.prototype, 'close');
    await expect(pluginCommands.update('plugin')).rejects.toMatchObject({
      backend: { code: 'PLUGIN_INSTALL_FAILED' },
    });
    expect(close).toHaveBeenCalledOnce();
    expect(nativeInvoke).toHaveBeenCalledWith(
      'plugin_update',
      {
        pluginId: 'plugin',
        operationId: 45,
        onProgress: expect.any(Channel),
      },
      undefined,
    );
  });

  it('keeps nullable session/audit fields and sends denial without an approval value', async () => {
    const session: PluginSession = {
      sessionId: 'session',
      pluginId: 'plugin',
      createdAt: 1,
      expiresAt: 2,
    };
    const audit: PluginAuditEntry = {
      timestamp: '2026-09-28',
      pluginId: 'plugin',
      sessionId: null,
      action: { action: 'plugin_run_completed', exit_code: 0 },
    };
    nativeInvoke.mockImplementation(async (command) => {
      if (command === 'plugin_list_sessions') return [session];
      if (command === 'plugin_audit_log') return [audit];
      return null;
    });
    expect(await pluginCommands.listSessions()).toEqual([session]);
    expect(await pluginCommands.auditLog()).toEqual([audit]);
    await pluginCommands.consentResponse('request', false);
    await pluginCommands.dialogResponse('request');
    expect(nativeInvoke).toHaveBeenCalledWith(
      'plugin_consent_response',
      {
        requestId: 'request',
        approved: false,
        value: undefined,
      },
      undefined,
    );
    expect(nativeInvoke).toHaveBeenCalledWith(
      'plugin_dialog_response',
      {
        requestId: 'request',
        value: undefined,
      },
      undefined,
    );
  });
});

describe('RF306 real PluginStore → typed transport → Channel', () => {
  it('filters arbitrary results and null request IDs, retains valid events, and does not duplicate final results', async () => {
    const pending = deferred<PluginResult>();
    nativeInvoke.mockImplementation(async (command) =>
      command === 'plugin_run' ? pending.promise : null,
    );
    const task = usePluginStore.getState().runPlugin('plugin', 'Synthetic Plugin');
    try {
      await vi.waitFor(() =>
        expect(nativeInvoke).toHaveBeenCalledWith('plugin_run', expect.anything(), undefined),
      );
      const channel = channelFor('plugin_run', 'channel');
      const valid = { type: 'text', content: 'visible' };
      for (const result of [
        null,
        true,
        ['array'],
        { type: 'future' },
        { type: 'table', headers: ['h'], rows: [null] },
        valid,
      ]) {
        emit(channel, pluginEvent({ eventType: 'result', jsonData: JSON.stringify(result) }));
      }
      const consent = pluginEvent({
        eventType: 'consent_request',
        requestId: 'consent',
        pluginId: 'plugin',
        pluginName: 'Synthetic Plugin',
        fieldId: 'field',
        fieldLabel: 'Field',
        sensitivityLevel: 'critical',
      });
      const dialog = pluginEvent({
        eventType: 'dialog_request',
        requestId: 'dialog',
        pluginId: 'plugin',
        pluginName: 'Synthetic Plugin',
      });
      emit(channel, { ...consent, requestId: null });
      emit(channel, { ...dialog, requestId: null });
      emit(channel, pluginEvent({ eventType: 'future', jsonData: 'ignored' }));
      emit(channel, consent);
      emit(channel, dialog);
      emit(channel, pluginEvent({ eventType: 'completed', jsonData: '{"exitCode":0}' }));
      const running = usePluginStore.getState().runningPlugins.plugin;
      expect(running.results).toEqual([valid]);
      expect(running.consentRequests).toEqual([consent]);
      expect(running.dialogRequests).toEqual([dialog]);
      expect(useUiStore.getState().toasts).toHaveLength(0);
      pending.resolve(pluginResult({ results: [valid, null, 123] }));
      await task;
      expect(usePluginStore.getState().runningPlugins.plugin.results).toEqual([valid]);
      expect(useUiStore.getState().toasts.map((toast) => toast.type)).toEqual(['success']);
      end(channel);
    } finally {
      pending.resolve(pluginResult());
      await task;
    }
  });

  it.each(['lock', 'reunlock', 'account-switch'])(
    'ignores late run events/results after %s without output, consent, dialog or toast',
    async (transition) => {
      const pending = deferred<PluginResult>();
      nativeInvoke.mockImplementation(async (command) =>
        command === 'plugin_run' ? pending.promise : null,
      );
      const task = usePluginStore.getState().runPlugin('plugin', 'Synthetic Plugin');
      try {
        await vi.waitFor(() =>
          expect(nativeInvoke).toHaveBeenCalledWith('plugin_run', expect.anything(), undefined),
        );
        const channel = channelFor('plugin_run', 'channel');
        useAuthStore.setState({ isAuthenticated: false });
        if (transition !== 'lock')
          useAuthStore.getState().completeUnlock({
            id: transition === 'account-switch' ? 'other-account' : 'account',
            name: 'Synthetic account',
          });
        emit(
          channel,
          pluginEvent({ eventType: 'result', jsonData: '{"type":"text","content":"late secret"}' }),
        );
        emit(
          channel,
          pluginEvent({
            eventType: 'consent_request',
            requestId: 'late',
            pluginId: 'plugin',
            pluginName: 'Plugin',
            fieldId: 'field',
            fieldLabel: 'Field',
            sensitivityLevel: 'critical',
          }),
        );
        emit(
          channel,
          pluginEvent({
            eventType: 'dialog_request',
            requestId: 'late',
            pluginId: 'plugin',
            pluginName: 'Plugin',
          }),
        );
        pending.resolve(pluginResult({ results: [{ type: 'text', content: 'late secret' }] }));
        await task;
        expect(usePluginStore.getState().runningPlugins).toEqual({});
        expect(useUiStore.getState().toasts).toEqual([]);
        end(channel);
      } finally {
        pending.resolve(pluginResult());
        await task;
      }
    },
  );

  it('a stopped old run cannot overwrite the next run, and stop does not claim native completion', async () => {
    const oldResult = deferred<PluginResult>();
    const newResult = deferred<PluginResult>();
    let runCount = 0;
    nativeInvoke.mockImplementation(async (command) => {
      if (command === 'plugin_run') return ++runCount === 1 ? oldResult.promise : newResult.promise;
      return null;
    });
    const oldTask = usePluginStore.getState().runPlugin('plugin', 'Old');
    let newTask: Promise<void> | undefined;
    let oldSettled = false;
    void oldTask.then(() => {
      oldSettled = true;
    });
    try {
      await vi.waitFor(() => expect(runCount).toBe(1));
      const oldChannel = channelFor('plugin_run', 'channel');
      usePluginStore.getState().stopPlugin('plugin');
      expect(oldSettled).toBe(false);
      newTask = usePluginStore.getState().runPlugin('plugin', 'New');
      await vi.waitFor(() => expect(runCount).toBe(2));
      const newChannel = channelFor('plugin_run', 'channel', 1);
      emit(
        oldChannel,
        pluginEvent({ eventType: 'result', jsonData: '{"type":"text","content":"old"}' }),
      );
      oldResult.resolve(pluginResult());
      await oldTask;
      expect(usePluginStore.getState().runningPlugins.plugin.completed).toBe(false);
      emit(
        newChannel,
        pluginEvent({ eventType: 'result', jsonData: '{"type":"text","content":"new"}' }),
      );
      newResult.resolve(pluginResult());
      await newTask;
      expect(usePluginStore.getState().runningPlugins.plugin.results).toEqual([
        { type: 'text', content: 'new' },
      ]);
      expect(useUiStore.getState().toasts).toHaveLength(1);
      expect(nativeInvoke.mock.calls.some(([command]) => command === 'plugin_cancel')).toBe(false);
      end(oldChannel);
      end(newChannel);
    } finally {
      oldResult.resolve(pluginResult());
      newResult.resolve(pluginResult());
      await oldTask;
      await newTask;
    }
  });
});

describe('RF320 actual SDK error transport and legacy channel projection', () => {
  it('keeps a read-only plugin callable while locked, but sanitizes rejection and channel errors', async () => {
    useAuthStore.setState({ currentAccount: null, isAuthenticated: false });
    const pending = deferred<PluginResult>();
    nativeInvoke.mockImplementation(async (cmd) => (cmd === 'plugin_run' ? pending.promise : null));
    const onEvent = vi.fn();
    const task = pluginCommands.run('plugin', {}, onEvent);
    await vi.waitFor(() =>
      expect(nativeInvoke).toHaveBeenCalledWith('plugin_run', expect.anything(), undefined),
    );
    const channel = channelFor('plugin_run', 'channel');
    emit(
      channel,
      pluginEvent({
        eventType: 'error',
        jsonData: JSON.stringify({
          code: 'PLUGIN_CONSENT_DENIED',
          message: 'RF320_PRIVATE_FIELD_KEY_PATH',
          key: 'RF320_PRIVATE_FIELD_KEY_PATH',
        }),
        fieldId: 'RF320_PRIVATE_FIELD_KEY_PATH',
        pluginName: 'RF320_PRIVATE_FIELD_KEY_PATH',
      }),
    );
    expect(onEvent).toHaveBeenCalledOnce();
    const projected = onEvent.mock.calls[0][0];
    expect(JSON.stringify(projected)).not.toContain('RF320_PRIVATE');
    expect(JSON.parse(projected.jsonData)).toEqual({
      code: 'PLUGIN_CONSENT_DENIED',
      message: 'PLUGIN_CONSENT_DENIED',
    });
    pending.reject(new Error('RF320_PRIVATE_FIELD_KEY_PATH'));
    const error = await task.catch((e: unknown) => e);
    expect(readBackendError(error)?.code).toBe('PLUGIN_EXECUTION_FAILED');
    expect(JSON.stringify(backendErrorLogDetails(error))).not.toContain('RF320_PRIVATE');
    end(channel);
  });
  it('drops stale error callbacks without projecting or storing them', async () => {
    let current = true;
    const events = vi.fn();
    const pending = deferred<PluginResult>();
    nativeInvoke.mockImplementation(async (cmd) => (cmd === 'plugin_run' ? pending.promise : null));
    const task = pluginCommands.run('plugin', {}, events, () => current);
    await vi.waitFor(() =>
      expect(nativeInvoke).toHaveBeenCalledWith('plugin_run', expect.anything(), undefined),
    );
    const channel = channelFor('plugin_run', 'channel');
    current = false;
    emit(channel, pluginEvent({ eventType: 'error', jsonData: 'private expired response' }));
    expect(events).not.toHaveBeenCalled();
    pending.resolve(pluginResult());
    await expect(task).resolves.toEqual(pluginResult());
    await expect(pluginCommands.run('plugin', {}, events, () => current)).rejects.toMatchObject({
      backend: { code: 'SESSION_EXPIRED' },
    });
    expect(nativeInvoke.mock.calls.filter(([cmd]) => cmd === 'plugin_run')).toHaveLength(1);
    end(channel);
  });
  it('stores classified channel errors and retains successful result schema', async () => {
    const pending = deferred<PluginResult>();
    nativeInvoke.mockImplementation(async (cmd) => (cmd === 'plugin_run' ? pending.promise : null));
    const task = usePluginStore.getState().runPlugin('plugin', 'Synthetic Plugin');
    await vi.waitFor(() =>
      expect(nativeInvoke).toHaveBeenCalledWith('plugin_run', expect.anything(), undefined),
    );
    const channel = channelFor('plugin_run', 'channel');
    emit(
      channel,
      pluginEvent({
        eventType: 'error',
        jsonData: JSON.stringify({
          code: 'PLUGIN_SESSION_EXPIRED',
          message: 'RF320_PRIVATE_FIELD_KEY_PATH',
        }),
      }),
    );
    expect(usePluginStore.getState().runningPlugins.plugin.error).toBe('PLUGIN_SESSION_EXPIRED');
    pending.resolve(pluginResult());
    await task;
    expect(usePluginStore.getState().runningPlugins.plugin.error).toBe('PLUGIN_SESSION_EXPIRED');
    expect(JSON.stringify(usePluginStore.getState().runningPlugins)).not.toContain('RF320_PRIVATE');
    end(channel);
  });
});

it('RF320 actual native audit results project old failure reason but retain normal audit shape', async () => {
  nativeInvoke.mockResolvedValue([
    {
      timestamp: '2001-02-03',
      pluginId: 'plugin',
      sessionId: null,
      action: { action: 'plugin_run_failed', reason: 'RF320_PRIVATE_FIELD_KEY_PATH' },
    },
    {
      timestamp: '2001-02-03',
      pluginId: 'plugin',
      sessionId: 'session',
      action: { action: 'plugin_run_failed', reason: 'PLUGIN_CONSENT_DENIED' },
    },
    {
      timestamp: '2001-02-03',
      pluginId: 'plugin',
      sessionId: null,
      action: { action: 'plugin_run_completed', exit_code: 0 },
    },
  ]);
  const entries = await pluginCommands.auditLog();
  expect(entries[0].action).toEqual({
    action: 'plugin_run_failed',
    reason: 'PLUGIN_EXECUTION_FAILED',
  });
  expect(entries[1].action).toEqual({
    action: 'plugin_run_failed',
    reason: 'PLUGIN_CONSENT_DENIED',
  });
  expect(entries[2].action).toEqual({ action: 'plugin_run_completed', exit_code: 0 });
  expect(entries[0].sessionId).toBeNull();
  expect(JSON.stringify(entries)).not.toContain('RF320_PRIVATE');
});
