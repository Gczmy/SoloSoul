import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { createSessionRequests, setRequestSession } from './sessionRequests';

const auth = vi.hoisted(() => ({ getState: vi.fn(() => ({ isAuthenticated: true })) }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
vi.mock('@/stores/authStore', () => ({ useAuthStore: { getState: auth.getState } }));

beforeEach(() => {
  vi.mocked(invoke).mockReset().mockResolvedValue(null);
  auth.getState.mockReset().mockReturnValue({ isAuthenticated: true });
  setRequestSession('session-a');
  vi.stubEnv('MODE', 'development');
  vi.spyOn(console, 'warn').mockImplementation(() => {});
});
afterEach(() => {
  setRequestSession(null);
  vi.unstubAllEnvs();
  vi.restoreAllMocks();
});

describe('RF302 typed session calls preserve the existing transport guards', () => {
  it('keeps the generated null response and native command arguments', async () => {
    const request = createSessionRequests().begin(undefined, 'session-a');
    await expect(
      request.invokeTyped('snapshot_rollback', {
        snapshotId: 'snapshot',
        objectId: 'object',
      }),
    ).resolves.toBeNull();
    expect(invoke).toHaveBeenCalledExactlyOnceWith('snapshot_rollback', {
      snapshotId: 'snapshot',
      objectId: 'object',
    });
    expect(auth.getState).toHaveBeenCalledOnce();
  });

  it('rejects locked calls before native IPC', async () => {
    auth.getState.mockReturnValue({ isAuthenticated: false });
    const request = createSessionRequests().begin();
    await expect(request.invokeTyped('object_delete', { objectId: 'object' })).rejects.toThrow(
      'VAULT_LOCKED',
    );
    expect(invoke).not.toHaveBeenCalled();
  });

  it('rejects an old account even when the caller opts out of the unlock guard', async () => {
    const request = createSessionRequests().begin(undefined, 'session-a');
    setRequestSession('session-b');
    await expect(
      request.invokeTyped(
        'object_get',
        {
          accountId: 'session-a',
          objectId: 'object',
        },
        { requireUnlocked: false },
      ),
    ).rejects.toThrow('Request belongs to an expired session');
    expect(invoke).not.toHaveBeenCalled();
  });

  it('rechecks the session after asynchronous authentication before dispatch', async () => {
    const request = createSessionRequests().begin(undefined, 'session-a');
    auth.getState.mockImplementation(() => {
      setRequestSession('session-b');
      return { isAuthenticated: true };
    });
    await expect(
      request.invokeTyped('object_get', {
        accountId: 'session-a',
        objectId: 'object',
      }),
    ).rejects.toThrow('Request belongs to an expired session');
    expect(invoke).not.toHaveBeenCalled();
  });

  it.each([false, true])(
    'rejects a late native result or failure after lock: %s',
    async (fails) => {
      let finish!: () => void;
      const started = new Promise<void>((resolveStarted) => {
        vi.mocked(invoke).mockImplementationOnce(
          () =>
            new Promise((resolve, reject) => {
              finish = () => (fails ? reject(new Error('native failure')) : resolve(null));
              resolveStarted();
            }),
        );
      });
      const request = createSessionRequests().begin();
      const pending = request.invokeTyped('object_get', {
        accountId: 'session-a',
        objectId: 'object',
      });
      const assertion = expect(pending).rejects.toThrow('Request belongs to an expired session');
      await started;
      setRequestSession(null);
      finish();
      await assertion;
    },
  );

  it.each([new Error('original'), 'native rejection'])(
    'preserves current-session failure with safe machine code: %s',
    async (failure) => {
      vi.mocked(invoke).mockRejectedValueOnce(failure);
      const request = createSessionRequests().begin();
      await expect(
        request.invokeTyped('object_list', { accountId: 'session-a' }),
      ).rejects.toMatchObject({
        backend: { code: 'INTERNAL_ERROR', safeDetails: null, retryable: false },
      });
    },
  );
});
