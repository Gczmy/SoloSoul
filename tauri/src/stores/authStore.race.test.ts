import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { LAST_ACCOUNT_KEY, useAuthStore } from './authStore';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}
const oldAccount = { id: 'old', name: 'Old account' };
const newAccount = { id: 'new', name: 'New account' };

describe('RF-1098 authentication settlement', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.mocked(invoke).mockReset().mockResolvedValue(undefined);
    useAuthStore.setState({
      isAuthenticated: false,
      isLoading: false,
      currentAccount: null,
      accounts: [],
      error: null,
      hasAccount: true,
      backendError: false,
    });
    localStorage.clear();
  });
  afterEach(() => {
    vi.clearAllTimers();
    vi.useRealTimers();
  });

  it.each(['lock', 'logout'] as const)(
    '%s keeps its state when the account refresh of an earlier login completes',
    async (action) => {
      const accounts = deferred<Array<typeof oldAccount>>();
      vi.mocked(invoke).mockImplementation(
        (command) =>
          (command === 'vault_list_accounts'
            ? accounts.promise
            : Promise.resolve(undefined)) as ReturnType<typeof invoke>,
      );
      const pending = useAuthStore.getState().login('old', 'test-password');
      await vi.waitFor(() => expect(invoke).toHaveBeenCalledWith('vault_list_accounts'));
      await useAuthStore.getState()[action]();
      const expiredState = useAuthStore.getState();
      expect(expiredState.isLoading).toBe(false);
      accounts.resolve([oldAccount]);
      await pending;
      expect(useAuthStore.getState()).toEqual(expiredState);
      expect(expiredState.isAuthenticated).toBe(false);
      expect(localStorage.getItem(LAST_ACCOUNT_KEY)).toBeNull();
      expect(vi.getTimerCount()).toBe(0);
    },
  );

  it.each(['resolve', 'reject'] as const)(
    'a login %s after lock cannot publish state or begin account refresh',
    async (finish) => {
      const login = deferred<void>();
      vi.mocked(invoke).mockImplementation(
        (command) =>
          (command === 'login' ? login.promise : Promise.resolve(undefined)) as ReturnType<
            typeof invoke
          >,
      );
      const pending = useAuthStore.getState().login('old', 'test-password');
      await useAuthStore.getState().lock();
      const lockedState = useAuthStore.getState();
      if (finish === 'resolve') login.resolve();
      else login.reject(Error('expired failure'));
      await pending;
      expect(useAuthStore.getState()).toEqual(lockedState);
      expect(invoke).not.toHaveBeenCalledWith('vault_list_accounts');
      expect(localStorage.getItem(LAST_ACCOUNT_KEY)).toBeNull();
    },
  );

  it.each(['resolve', 'reject'] as const)(
    'an account refresh %s cannot overwrite a newer completeUnlock',
    async (finish) => {
      const accounts = deferred<Array<typeof oldAccount>>();
      vi.mocked(invoke).mockImplementation(
        (command) =>
          (command === 'vault_list_accounts'
            ? accounts.promise
            : Promise.resolve(undefined)) as ReturnType<typeof invoke>,
      );
      const pending = useAuthStore.getState().login('old', 'test-password');
      await vi.waitFor(() => expect(invoke).toHaveBeenCalledWith('vault_list_accounts'));
      useAuthStore.getState().completeUnlock(newAccount, [newAccount]);
      const unlockedState = useAuthStore.getState();
      if (finish === 'resolve') accounts.resolve([oldAccount]);
      else accounts.reject(Error('expired list failure'));
      await pending;
      expect(useAuthStore.getState()).toEqual(unlockedState);
      expect(vi.getTimerCount()).toBe(0);
    },
  );

  it.each(['resolve', 'reject'] as const)(
    'an older login %s keeps a newer pending login loading and error intact',
    async (finish) => {
      const oldLogin = deferred<void>();
      const newLogin = deferred<void>();
      vi.mocked(invoke).mockImplementation(
        (command, args) =>
          (command === 'login'
            ? args && 'accountId' in args && args.accountId === 'old'
              ? oldLogin.promise
              : newLogin.promise
            : Promise.resolve([newAccount])) as ReturnType<typeof invoke>,
      );
      const first = useAuthStore.getState().login('old', 'test-password');
      const second = useAuthStore.getState().login('new', 'test-password');
      if (finish === 'resolve') oldLogin.resolve();
      else oldLogin.reject(Error('old login failure'));
      await first;
      expect(useAuthStore.getState()).toMatchObject({
        isLoading: true,
        error: null,
        isAuthenticated: false,
      });
      newLogin.resolve();
      await second;
      expect(useAuthStore.getState()).toMatchObject({
        isLoading: false,
        error: null,
        currentAccount: newAccount,
        isAuthenticated: true,
      });
    },
  );

  it('out-of-order account refreshes retain only the latest successful password login', async () => {
    const oldList = deferred<Array<typeof oldAccount>>();
    let lists = 0;
    vi.mocked(invoke).mockImplementation(
      (command) =>
        (command === 'vault_list_accounts'
          ? ++lists === 1
            ? oldList.promise
            : Promise.resolve([newAccount])
          : Promise.resolve(undefined)) as ReturnType<typeof invoke>,
    );
    const first = useAuthStore.getState().login('old', 'test-password');
    await vi.waitFor(() => expect(lists).toBe(1));
    await useAuthStore.getState().login('new', 'test-password');
    const current = useAuthStore.getState();
    const timers = vi.getTimerCount();
    oldList.resolve([oldAccount]);
    await first;
    expect(useAuthStore.getState()).toEqual(current);
    expect(localStorage.getItem(LAST_ACCOUNT_KEY)).toBe('new');
    expect(vi.getTimerCount()).toBe(timers);
  });

  for (const action of ['lock', 'logout', 'completeUnlock'] as const) {
    it.each(['resolve', 'reject'] as const)(
      `bootstrap %s after ${action} cannot publish an expired account or error`,
      async (finish) => {
        const bootstrap = deferred<typeof oldAccount>();
        vi.mocked(invoke).mockImplementation(
          (command) =>
            (command === 'bootstrap'
              ? bootstrap.promise
              : Promise.resolve(undefined)) as ReturnType<typeof invoke>,
        );
        const pending = useAuthStore.getState().bootstrap('Old account', 'test-password', 'en-US');
        if (action === 'completeUnlock')
          useAuthStore.getState().completeUnlock(newAccount, [newAccount]);
        else await useAuthStore.getState()[action]();
        const current = useAuthStore.getState();
        if (finish === 'resolve') bootstrap.resolve(oldAccount);
        else bootstrap.reject(Error('expired bootstrap failure'));
        await pending;
        expect(useAuthStore.getState()).toEqual(current);
        expect(current.isLoading).toBe(false);
        expect(localStorage.getItem(LAST_ACCOUNT_KEY)).toBeNull();
      },
    );
  }

  it('starting bootstrap invalidates an older password login', async () => {
    const login = deferred<void>();
    vi.mocked(invoke).mockImplementation(
      (command) =>
        (command === 'login' ? login.promise : Promise.resolve(newAccount)) as ReturnType<
          typeof invoke
        >,
    );
    const pending = useAuthStore.getState().login('old', 'test-password');
    await useAuthStore.getState().bootstrap('New account', 'test-password', 'en-US');
    const current = useAuthStore.getState();
    login.resolve();
    await pending;
    expect(useAuthStore.getState()).toEqual(current);
    expect(localStorage.getItem(LAST_ACCOUNT_KEY)).toBe('new');
    expect(invoke).not.toHaveBeenCalledWith('vault_list_accounts');
  });

  it('starting password login invalidates an older bootstrap', async () => {
    const bootstrap = deferred<typeof oldAccount>();
    vi.mocked(invoke).mockImplementation(
      (command) =>
        (command === 'bootstrap'
          ? bootstrap.promise
          : Promise.resolve(
              command === 'vault_list_accounts' ? [newAccount] : undefined,
            )) as ReturnType<typeof invoke>,
    );
    const pending = useAuthStore.getState().bootstrap('Old account', 'test-password', 'en-US');
    await useAuthStore.getState().login('new', 'test-password');
    const current = useAuthStore.getState();
    const timers = vi.getTimerCount();
    bootstrap.resolve(oldAccount);
    await pending;
    expect(useAuthStore.getState()).toEqual(current);
    expect(localStorage.getItem(LAST_ACCOUNT_KEY)).toBe('new');
    expect(vi.getTimerCount()).toBe(timers);
  });
});
