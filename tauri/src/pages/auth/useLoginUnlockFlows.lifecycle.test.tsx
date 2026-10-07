import { act, renderHook } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { LAST_ACCOUNT_KEY, useAuthStore } from '@/stores/authStore';
import { useLoginUnlockFlows } from './useLoginUnlockFlows';

const { navigate } = vi.hoisted(() => ({ navigate: vi.fn() }));
vi.mock('react-router-dom', async (original) => ({
  ...(await original<typeof import('react-router-dom')>()),
  useNavigate: () => navigate,
}));
function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}
const accountA = { id: 'acc-a', name: 'Account A' };
const accountB = { id: 'acc-b', name: 'Account B' };
const clock = window as typeof window & { __SOLOSOUL_UNLOCK_TIME?: number };
function renderFlow(fromExisting = false) {
  const callbacks = { setLoginMethod: vi.fn(), setBioLockout: vi.fn(), setPinAvailable: vi.fn() };
  const hook = renderHook(
    ({ accountId }) =>
      useLoginUnlockFlows({
        selectedAccountId: accountId,
        fromExisting,
        bioLockout: false,
        biometryTypeRaw: 'touchId',
        ...callbacks,
      }),
    { initialProps: { accountId: accountA.id }, wrapper: MemoryRouter },
  );
  return { ...hook, callbacks };
}
type Method = 'pin' | 'biometric';
function delayedUnlock(response: ReturnType<typeof deferred<unknown>>) {
  vi.mocked(invoke).mockImplementation(
    (command) =>
      (command === 'pin_unlock' || command === 'biometric_unlock'
        ? response.promise
        : Promise.resolve(
            command === 'vault_list_accounts' ? [accountA, accountB] : undefined,
          )) as ReturnType<typeof invoke>,
  );
}
function start(flow: ReturnType<typeof renderFlow>, method: Method) {
  let pending!: Promise<void>;
  act(() => {
    pending =
      method === 'pin'
        ? flow.result.current.handlePinComplete('123456')
        : flow.result.current.handleBiometricUnlock();
  });
  return pending;
}
async function finish(
  response: ReturnType<typeof deferred<unknown>>,
  method: Method,
  outcome: 'resolve' | 'reject',
  pending: Promise<void>,
) {
  await act(async () => {
    if (outcome === 'resolve') response.resolve(method === 'pin' ? accountA : undefined);
    else response.reject(Error(method === 'pin' ? '__PIN_ERR__:locked' : '__BIO_ERR__:lockout'));
    await pending;
  });
}

describe('RF-1099 unlock request lifecycle', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.mocked(invoke).mockReset().mockResolvedValue(undefined);
    navigate.mockClear();
    localStorage.clear();
    clock.__SOLOSOUL_UNLOCK_TIME = 987;
    useAuthStore.setState({
      isAuthenticated: false,
      isLoading: false,
      currentAccount: null,
      accounts: [accountA, accountB],
      hasAccount: true,
      error: null,
    });
  });
  afterEach(() => {
    vi.clearAllTimers();
    vi.useRealTimers();
    delete clock.__SOLOSOUL_UNLOCK_TIME;
  });

  for (const method of ['pin', 'biometric'] as const) {
    for (const expiry of [
      'lock',
      'logout',
      'account-switch',
      'unmount',
      'new-unlock',
      'password-login',
    ] as const) {
      it.each(['resolve', 'reject'] as const)(
        `${method}: late %s after ${expiry} has no effects`,
        async (outcome) => {
          const response = deferred<unknown>();
          delayedUnlock(response);
          const flow = renderFlow();
          const pending = start(flow, method);
          if (expiry === 'lock' || expiry === 'logout')
            await act(async () => useAuthStore.getState()[expiry]());
          else if (expiry === 'account-switch') flow.rerender({ accountId: accountB.id });
          else if (expiry === 'unmount') flow.unmount();
          else if (expiry === 'new-unlock')
            act(() => useAuthStore.getState().completeUnlock(accountB, [accountB]));
          else await act(async () => useAuthStore.getState().login(accountB.id, 'test-password'));
          const current = useAuthStore.getState();
          const saved = localStorage.getItem(LAST_ACCOUNT_KEY);
          const unlockClock = clock.__SOLOSOUL_UNLOCK_TIME;
          const timers = vi.getTimerCount();
          if (expiry !== 'unmount') {
            expect(flow.result.current.pinUnlocking).toBe(false);
            expect(flow.result.current.bioLoading).toBe(false);
          }
          await finish(response, method, outcome, pending);
          expect(useAuthStore.getState()).toEqual(current);
          expect(localStorage.getItem(LAST_ACCOUNT_KEY)).toBe(saved);
          expect(clock.__SOLOSOUL_UNLOCK_TIME).toBe(unlockClock);
          expect(navigate).not.toHaveBeenCalled();
          expect(vi.getTimerCount()).toBe(timers);
          for (const callback of Object.values(flow.callbacks))
            expect(callback).not.toHaveBeenCalled();
          if (expiry !== 'unmount') {
            expect(flow.result.current.pinError).toBeNull();
            expect(flow.result.current.bioError).toBeNull();
          }
        },
      );
    }
  }

  for (const expiry of ['lock', 'account-switch', 'unmount', 'new-unlock'] as const) {
    it.each(['resolve', 'reject'] as const)(
      `biometric list refresh: %s after ${expiry} is ignored`,
      async (outcome) => {
        const accounts = deferred<unknown>();
        vi.mocked(invoke).mockImplementation(
          (command) =>
            (command === 'vault_list_accounts'
              ? accounts.promise
              : Promise.resolve(undefined)) as ReturnType<typeof invoke>,
        );
        const flow = renderFlow();
        const pending = start(flow, 'biometric');
        await act(async () => {
          await vi.waitFor(() => expect(invoke).toHaveBeenCalledWith('vault_list_accounts'));
        });
        if (expiry === 'lock') await act(async () => useAuthStore.getState().lock());
        else if (expiry === 'account-switch') flow.rerender({ accountId: accountB.id });
        else if (expiry === 'unmount') flow.unmount();
        else act(() => useAuthStore.getState().completeUnlock(accountB, [accountB]));
        const current = useAuthStore.getState();
        await act(async () => {
          if (outcome === 'resolve') accounts.resolve([accountA]);
          else accounts.reject(Error('old account list failure'));
          await pending;
        });
        expect(useAuthStore.getState()).toEqual(current);
        expect(navigate).not.toHaveBeenCalled();
        expect(localStorage.getItem(LAST_ACCOUNT_KEY)).toBeNull();
        expect(clock.__SOLOSOUL_UNLOCK_TIME).toBe(987);
      },
    );
  }

  it.each(['pin', 'biometric'] as const)(
    '%s suppresses synchronous duplicate submissions and still succeeds',
    async (method) => {
      const response = deferred<unknown>();
      delayedUnlock(response);
      const flow = renderFlow();
      const first = start(flow, method);
      const second = start(flow, method);
      expect(
        vi.mocked(invoke).mock.calls.filter(([command]) => command === `${method}_unlock`),
      ).toHaveLength(1);
      await finish(response, method, 'resolve', first);
      await second;
      expect(useAuthStore.getState()).toMatchObject({
        isAuthenticated: true,
        currentAccount: accountA,
      });
      expect(localStorage.getItem(LAST_ACCOUNT_KEY)).toBe(accountA.id);
      expect(navigate).toHaveBeenCalledTimes(1);
      expect(navigate).toHaveBeenCalledWith('/');
    },
  );

  it('an obsolete PIN failure cannot reset a newer biometric attempt', async () => {
    const pin = deferred<unknown>();
    const biometric = deferred<unknown>();
    vi.mocked(invoke).mockImplementation(
      (command) =>
        (command === 'pin_unlock'
          ? pin.promise
          : command === 'biometric_unlock'
            ? biometric.promise
            : Promise.resolve(
                command === 'vault_list_accounts' ? [accountA] : undefined,
              )) as ReturnType<typeof invoke>,
    );
    const flow = renderFlow();
    const first = start(flow, 'pin');
    const second = start(flow, 'biometric');
    await finish(pin, 'pin', 'reject', first);
    expect(flow.result.current.bioLoading).toBe(true);
    expect(flow.result.current.pinError).toBeNull();
    expect(flow.callbacks.setLoginMethod).not.toHaveBeenCalled();
    await finish(biometric, 'biometric', 'resolve', second);
    expect(useAuthStore.getState().isAuthenticated).toBe(true);
    expect(navigate).toHaveBeenCalledTimes(1);
  });

  it('account selection cancels a pending password login and clears its password', async () => {
    const response = deferred<unknown>();
    vi.mocked(invoke).mockImplementation(
      (command) =>
        (command === 'login'
          ? response.promise
          : Promise.resolve([accountA, accountB])) as ReturnType<typeof invoke>,
    );
    const flow = renderFlow();
    act(() => flow.result.current.setPassword('test-password'));
    let pending!: Promise<void>;
    act(() => {
      pending = flow.result.current.handleSubmit();
    });
    flow.rerender({ accountId: accountB.id });
    expect(flow.result.current.password).toBe('');
    expect(useAuthStore.getState().isLoading).toBe(false);
    await act(async () => {
      response.resolve(undefined);
      await pending;
    });
    expect(useAuthStore.getState().isAuthenticated).toBe(false);
    expect(invoke).not.toHaveBeenCalledWith('vault_list_accounts');
    expect(localStorage.getItem(LAST_ACCOUNT_KEY)).toBeNull();
  });

  it('unmounting an old PIN page cannot cancel a newer password login', async () => {
    const pin = deferred<unknown>();
    const password = deferred<unknown>();
    vi.mocked(invoke).mockImplementation(
      (command) =>
        (command === 'pin_unlock'
          ? pin.promise
          : command === 'login'
            ? password.promise
            : Promise.resolve([accountB])) as ReturnType<typeof invoke>,
    );
    const flow = renderFlow();
    const old = start(flow, 'pin');
    let fresh!: Promise<void>;
    act(() => {
      fresh = useAuthStore.getState().login(accountB.id, 'test-password');
    });
    flow.unmount();
    expect(useAuthStore.getState().isLoading).toBe(true);
    await finish(pin, 'pin', 'reject', old);
    expect(useAuthStore.getState().isLoading).toBe(true);
    await act(async () => {
      password.resolve(undefined);
      await fresh;
    });
    expect(useAuthStore.getState()).toMatchObject({
      isAuthenticated: true,
      currentAccount: accountB,
      isLoading: false,
    });
  });

  it('handlers retained by an unmounted page do not send new native requests', async () => {
    const flow = renderFlow();
    const old = flow.result.current;
    flow.unmount();
    await old.handlePinComplete('123456');
    await old.handleBiometricUnlock();
    expect(invoke).not.toHaveBeenCalled();
  });
  it.each([
    ['__PIN_ERR__:locked', 'auth:pin_locked', true],
    ['__PIN_ERR__:incorrect', 'auth:pin_incorrect', false],
    ['unavailable', 'auth:pin_error', false],
  ] as const)('current PIN error %s preserves its fallback', async (message, error, fallback) => {
    vi.mocked(invoke).mockRejectedValueOnce(Error(message));
    const flow = renderFlow();
    await act(async () => flow.result.current.handlePinComplete('123456'));
    expect(flow.result.current.pinError).toBe(error);
    expect(flow.result.current.pinUnlocking).toBe(false);
    expect(flow.result.current.pinInputKey).toBe(1);
    expect(useAuthStore.getState().isAuthenticated).toBe(false);
    if (fallback) {
      expect(flow.callbacks.setPinAvailable).toHaveBeenCalledWith(false);
      expect(flow.callbacks.setLoginMethod).toHaveBeenCalledWith('password');
    } else expect(flow.callbacks.setLoginMethod).not.toHaveBeenCalled();
  });

  it.each(['__BIO_ERR__:cancelled', '__BIO_ERR__:lockout', '__BIO_ERR__:invalid_password'])(
    'current biometric error %s preserves its fallback',
    async (message) => {
      vi.mocked(invoke).mockRejectedValueOnce(Error(message));
      const flow = renderFlow();
      await act(async () => flow.result.current.handleBiometricUnlock());
      expect(flow.result.current.bioLoading).toBe(false);
      expect(flow.callbacks.setLoginMethod).toHaveBeenCalledWith('password');
      expect(useAuthStore.getState().isAuthenticated).toBe(false);
      if (message.includes('cancelled')) expect(flow.result.current.bioError).toBeNull();
      else expect(flow.result.current.bioError).not.toBeNull();
      if (message.includes('lockout'))
        expect(flow.callbacks.setBioLockout).toHaveBeenCalledWith(true);
    },
  );

  it('successful navigation does not cancel the existing-directory safety refresh', async () => {
    const reset = deferred<unknown>();
    let accountReads = 0;
    vi.mocked(invoke).mockImplementation((command) => {
      if (command === 'reset_security_flags') return reset.promise as ReturnType<typeof invoke>;
      if (command === 'vault_list_accounts') {
        accountReads += 1;
        return Promise.resolve([{ ...accountA, hasPinHistory: accountReads === 1 }]) as ReturnType<
          typeof invoke
        >;
      }
      return Promise.resolve(undefined);
    });
    const flow = renderFlow(true);
    act(() => flow.result.current.setPassword('test-password'));
    let pending!: Promise<void>;
    act(() => {
      pending = flow.result.current.handleSubmit();
    });
    await act(async () => {
      await vi.waitFor(() =>
        expect(invoke).toHaveBeenCalledWith('reset_security_flags', { accountId: accountA.id }),
      );
    });
    expect(useAuthStore.getState().isAuthenticated).toBe(true);
    flow.unmount();
    await act(async () => {
      reset.resolve(undefined);
      await pending;
    });
    expect(accountReads).toBe(2);
    expect(useAuthStore.getState().currentAccount).toMatchObject({
      id: accountA.id,
      hasPinHistory: false,
    });
    expect(useAuthStore.getState().isAuthenticated).toBe(true);
  });

  it('locking during the safety reset prevents its account refresh', async () => {
    const reset = deferred<unknown>();
    vi.mocked(invoke).mockImplementation(
      (command) =>
        (command === 'reset_security_flags'
          ? reset.promise
          : Promise.resolve(
              command === 'vault_list_accounts' ? [accountA] : undefined,
            )) as ReturnType<typeof invoke>,
    );
    const flow = renderFlow(true);
    act(() => flow.result.current.setPassword('test-password'));
    let pending!: Promise<void>;
    act(() => {
      pending = flow.result.current.handleSubmit();
    });
    await act(async () => {
      await vi.waitFor(() =>
        expect(invoke).toHaveBeenCalledWith('reset_security_flags', { accountId: accountA.id }),
      );
      await useAuthStore.getState().lock();
      reset.resolve(undefined);
      await pending;
    });
    expect(
      vi.mocked(invoke).mock.calls.filter(([command]) => command === 'vault_list_accounts'),
    ).toHaveLength(1);
    expect(useAuthStore.getState().isAuthenticated).toBe(false);
  });

  it.each(['resolve', 'reject'] as const)(
    'late safety account refresh %s after lock has no effects',
    async (outcome) => {
      const refresh = deferred<unknown>();
      let accountReads = 0;
      vi.mocked(invoke).mockImplementation((command) => {
        if (command === 'vault_list_accounts') {
          accountReads += 1;
          return (accountReads === 1 ? Promise.resolve([accountA]) : refresh.promise) as ReturnType<
            typeof invoke
          >;
        }
        return Promise.resolve(undefined);
      });
      const flow = renderFlow(true);
      act(() => flow.result.current.setPassword('test-password'));
      let pending!: Promise<void>;
      act(() => {
        pending = flow.result.current.handleSubmit();
      });
      await act(async () => {
        await vi.waitFor(() => expect(accountReads).toBe(2));
        await useAuthStore.getState().lock();
      });
      const locked = useAuthStore.getState();
      await act(async () => {
        if (outcome === 'resolve') refresh.resolve([accountB]);
        else refresh.reject(Error('old safety refresh failure'));
        await pending;
      });
      expect(useAuthStore.getState()).toEqual(locked);
    },
  );

  it('password submission retained after unmount cannot clear the new page error', async () => {
    const flow = renderFlow();
    act(() => flow.result.current.setPassword('test-password'));
    const old = flow.result.current;
    flow.unmount();
    useAuthStore.setState({ error: 'new page error' });
    await old.handleSubmit();
    expect(useAuthStore.getState().error).toBe('new page error');
    expect(invoke).not.toHaveBeenCalled();
  });

  it('handlers retained after account selection cannot start an old-account unlock', async () => {
    const flow = renderFlow();
    const old = flow.result.current;
    flow.rerender({ accountId: accountB.id });
    await old.handlePinComplete('123456');
    await old.handleBiometricUnlock();
    expect(invoke).not.toHaveBeenCalled();
  });
});
