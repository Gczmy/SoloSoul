import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import type { AppInfo } from './generated/ipcContracts';
import { invokeTypedCommand } from './typedIpc';

const auth = vi.hoisted(() => ({ getState: vi.fn(() => ({ isAuthenticated: false })) }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
vi.mock('@/stores/authStore', () => ({ useAuthStore: { getState: auth.getState } }));

const appInfo = {
  appName: 'SoloSoul',
  version: '2.13.2',
  os: 'windows',
  arch: 'x86_64',
} satisfies AppInfo;

describe('invokeTypedCommand delegates to the existing IPC transport', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    vi.mocked(invoke).mockResolvedValue(appInfo);
    auth.getState.mockReset();
    auth.getState.mockReturnValue({ isAuthenticated: false });
    // 避免 MODE=test 的现有守卫豁免掩盖真实鉴权行为。
    vi.stubEnv('MODE', 'development');
    vi.spyOn(console, 'warn').mockImplementation(() => {});
  });

  afterEach(() => {
    vi.unstubAllEnvs();
    vi.restoreAllMocks();
  });

  it('keeps get_app_info available before unlock and invokes native IPC with one argument', async () => {
    await expect(invokeTypedCommand('get_app_info')).resolves.toBe(appInfo);
    expect(invoke).toHaveBeenCalledExactlyOnceWith('get_app_info');
    expect(auth.getState).not.toHaveBeenCalled();
  });

  it('keeps the options in the third position without sending undefined native arguments', async () => {
    const requestIsCurrent = vi.fn(() => true);
    await expect(
      invokeTypedCommand('get_app_info', undefined, {
        requireUnlocked: false,
        requestIsCurrent,
      }),
    ).resolves.toBe(appInfo);
    expect(requestIsCurrent).toHaveBeenCalledOnce();
    expect(invoke).toHaveBeenCalledExactlyOnceWith('get_app_info');
    expect(auth.getState).not.toHaveBeenCalled();
  });

  it('preserves requireUnlocked=true for an otherwise exempt command', async () => {
    await expect(
      invokeTypedCommand('get_app_info', undefined, {
        requireUnlocked: true,
      }),
    ).rejects.toThrow('No account is currently unlocked');
    expect(auth.getState).toHaveBeenCalledOnce();
    expect(invoke).not.toHaveBeenCalled();
  });

  it('allows the forced guard after authentication and retains the full response object', async () => {
    auth.getState.mockReturnValue({ isAuthenticated: true });
    const result = await invokeTypedCommand('get_app_info', undefined, { requireUnlocked: true });
    expect(result).toBe(appInfo);
    expect(result).toEqual({
      appName: 'SoloSoul',
      version: '2.13.2',
      os: 'windows',
      arch: 'x86_64',
    });
    expect(invoke).toHaveBeenCalledExactlyOnceWith('get_app_info');
  });

  it('rejects an expired exempt request before native dispatch', async () => {
    await expect(
      invokeTypedCommand('get_app_info', undefined, {
        requestIsCurrent: () => false,
      }),
    ).rejects.toThrow('Request belongs to an expired session');
    expect(invoke).not.toHaveBeenCalled();
  });

  it('rechecks the request after asynchronous authentication', async () => {
    let current = true;
    auth.getState.mockImplementation(() => {
      current = false;
      return { isAuthenticated: true };
    });
    await expect(
      invokeTypedCommand('get_app_info', undefined, {
        requireUnlocked: true,
        requestIsCurrent: () => current,
      }),
    ).rejects.toThrow('Request belongs to an expired session');
    expect(auth.getState).toHaveBeenCalledOnce();
    expect(invoke).not.toHaveBeenCalled();
  });

  it.each([new Error('synthetic IPC failure'), 'synthetic native rejection'])(
    'preserves unmigrated native rejection identity with redacted command log: %s',
    async (failure) => {
      vi.mocked(invoke).mockRejectedValue(failure);
      await expect(invokeTypedCommand('get_app_info')).rejects.toBe(failure);
      expect(invoke).toHaveBeenCalledExactlyOnceWith('get_app_info');
      expect(console.warn).toHaveBeenCalledExactlyOnceWith("[ipc] command 'get_app_info' failed:", {
        code: 'LEGACY_ERROR',
      });
    },
  );
});
