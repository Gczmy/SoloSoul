import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { useAuthStore } from './authStore';

// Mock the IPC invoke function
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

import { invoke } from '@tauri-apps/api/core';

describe('authStore', () => {
  beforeEach(() => {
    useAuthStore.setState({
      isAuthenticated: false,
      isLoading: false,
      currentAccount: null,
      accounts: [],
      error: null,
      hasAccount: null,
      backendError: false,
    });
    vi.clearAllMocks();
  });

  describe('checkHasAccount', () => {
    it('should set hasAccount to true when backend returns true', async () => {
      vi.mocked(invoke).mockResolvedValue(true);
      await useAuthStore.getState().checkHasAccount();
      expect(useAuthStore.getState().hasAccount).toBe(true);
      expect(useAuthStore.getState().backendError).toBe(false);
    });

    it('should set hasAccount to false when backend returns false', async () => {
      vi.mocked(invoke).mockResolvedValue(false);
      await useAuthStore.getState().checkHasAccount();
      expect(useAuthStore.getState().hasAccount).toBe(false);
      expect(useAuthStore.getState().backendError).toBe(false);
    });

    it('should set backendError on exception', async () => {
      vi.mocked(invoke).mockRejectedValue(new Error('backend down'));
      await useAuthStore.getState().checkHasAccount();
      expect(useAuthStore.getState().hasAccount).toBeNull();
      expect(useAuthStore.getState().backendError).toBe(true);
    });
  });

  describe('listAccounts', () => {
    it('should load accounts and update state', async () => {
      const accounts = [
        { id: 'acc-1', name: 'Alice' },
        { id: 'acc-2', name: 'Bob' },
      ];
      vi.mocked(invoke).mockImplementation(async (cmd: string) => {
        if (cmd === 'vault_list_accounts') return accounts;
        return undefined;
      });
      await useAuthStore.getState().listAccounts();
      expect(useAuthStore.getState().accounts).toEqual(accounts);
      expect(useAuthStore.getState().hasAccount).toBe(true);
    });

    it('should preserve currentAccount if still present in refreshed list', async () => {
      useAuthStore.setState({ currentAccount: { id: 'acc-1', name: 'Alice' } });
      const refreshed = [
        { id: 'acc-1', name: 'Alice Updated' },
        { id: 'acc-2', name: 'Bob' },
      ];
      vi.mocked(invoke).mockImplementation(async (cmd: string) => {
        if (cmd === 'vault_list_accounts') return refreshed;
        return undefined;
      });
      await useAuthStore.getState().listAccounts();
      expect(useAuthStore.getState().currentAccount).toEqual({
        id: 'acc-1',
        name: 'Alice Updated',
      });
    });

    it('should silently fail when vault is locked', async () => {
      vi.mocked(invoke).mockRejectedValue(new Error('locked'));
      await useAuthStore.getState().listAccounts();
      expect(useAuthStore.getState().accounts).toEqual([]);
    });
  });

  describe('bootstrap', () => {
    it('should create account and set authenticated state', async () => {
      const account = { id: 'new-acc', name: 'Charlie' };
      vi.mocked(invoke).mockImplementation(async (cmd: string) => {
        if (cmd === 'bootstrap') return account;
        return undefined;
      });
      await useAuthStore.getState().bootstrap('Charlie', 'password123', 'en-US');
      expect(useAuthStore.getState().isAuthenticated).toBe(true);
      expect(useAuthStore.getState().currentAccount).toEqual(account);
      expect(useAuthStore.getState().accounts).toEqual([account]);
      expect(useAuthStore.getState().hasAccount).toBe(true);
      expect(useAuthStore.getState().isLoading).toBe(false);
    });

    it('should set error on bootstrap failure', async () => {
      vi.mocked(invoke).mockRejectedValue(new Error('name taken'));
      await useAuthStore.getState().bootstrap('Charlie', 'password123', 'en-US');
      expect(useAuthStore.getState().isAuthenticated).toBe(false);
      expect(useAuthStore.getState().error).toBe('Error: name taken');
      expect(useAuthStore.getState().isLoading).toBe(false);
    });
  });

  describe('login', () => {
    it('should authenticate and load accounts', async () => {
      const accounts = [{ id: 'acc-1', name: 'Alice' }];
      vi.mocked(invoke).mockImplementation(async (cmd: string) => {
        if (cmd === 'login') return undefined;
        if (cmd === 'vault_list_accounts') return accounts;
        return undefined;
      });
      await useAuthStore.getState().login('acc-1', 'password123');
      expect(useAuthStore.getState().isAuthenticated).toBe(true);
      expect(useAuthStore.getState().currentAccount).toEqual({ id: 'acc-1', name: 'Alice' });
      expect(useAuthStore.getState().accounts).toEqual(accounts);
      expect(useAuthStore.getState().isLoading).toBe(false);
    });

    it('should use fallback account info when list returns empty', async () => {
      vi.mocked(invoke).mockImplementation(async (cmd: string) => {
        if (cmd === 'login') return undefined;
        if (cmd === 'vault_list_accounts') return [];
        return undefined;
      });
      await useAuthStore.getState().login('acc-1', 'password123');
      expect(useAuthStore.getState().currentAccount).toEqual({ id: 'acc-1', name: 'acc-1' });
    });

    it('should set error on login failure', async () => {
      vi.mocked(invoke).mockRejectedValue(new Error('wrong password'));
      await useAuthStore.getState().login('acc-1', 'wrong');
      expect(useAuthStore.getState().isAuthenticated).toBe(false);
      expect(useAuthStore.getState().error).toBe('Error: wrong password');
      expect(useAuthStore.getState().isLoading).toBe(false);
    });
  });

  describe('logout', () => {
    it('should clear authenticated state', async () => {
      useAuthStore.setState({
        isAuthenticated: true,
        currentAccount: { id: 'acc-1', name: 'Alice' },
      });
      vi.mocked(invoke).mockResolvedValue(undefined);
      await useAuthStore.getState().logout();
      expect(useAuthStore.getState().isAuthenticated).toBe(false);
      expect(useAuthStore.getState().currentAccount).toBeNull();
    });
  });

  describe('clearError', () => {
    it('should reset error to null', () => {
      useAuthStore.setState({ error: 'some error' });
      useAuthStore.getState().clearError();
      expect(useAuthStore.getState().error).toBeNull();
    });
  });
});

describe('native authentication checkpoints', () => {
  const checkpoints: Array<[number, string]> = [];
  const host = window as typeof window & { __SOLOSOUL_NATIVE_PERF__?: unknown };
  beforeEach(() => {
    vi.useFakeTimers();
    vi.mocked(invoke).mockReset();
    checkpoints.length = 0;
    useAuthStore.setState({
      isAuthenticated: false,
      isLoading: false,
      currentAccount: null,
      accounts: [],
      error: null,
    });
    host.__SOLOSOUL_NATIVE_PERF__ = {
      beginAuthFlow: () => 1,
      markAuthFlow: (id: number, stage: string) => checkpoints.push([id, stage]),
      authSnapshot: () => ({}),
    };
  });
  afterEach(() => {
    delete host.__SOLOSOUL_NATIVE_PERF__;
    vi.clearAllTimers();
    vi.useRealTimers();
  });
  it('records the two actual awaits and the state transition without forwarding authentication data', async () => {
    let finishLogin!: () => void;
    let finishAccounts!: (value: Array<{ id: string; name: string }>) => void;
    const login = new Promise<void>((done) => {
      finishLogin = done;
    });
    const accounts = new Promise<Array<{ id: string; name: string }>>((done) => {
      finishAccounts = done;
    });
    vi.mocked(invoke).mockImplementation(
      (cmd) => (cmd === 'login' ? login : accounts) as ReturnType<typeof invoke>,
    );
    const pending = useAuthStore
      .getState()
      .login('account-do-not-record', 'password-do-not-record');
    expect(checkpoints).toEqual([[1, 'login-await-start']]);
    expect(useAuthStore.getState().isAuthenticated).toBe(false);
    finishLogin();
    await vi.waitFor(() => expect(checkpoints.at(-1)).toEqual([1, 'accounts-await-start']));
    expect(useAuthStore.getState().isAuthenticated).toBe(false);
    finishAccounts([{ id: 'account-do-not-record', name: 'Name' }]);
    await pending;
    expect(checkpoints.map((x) => x[1])).toEqual([
      'login-await-start',
      'login-await-ok',
      'accounts-await-start',
      'accounts-await-ok',
      'state-set-start',
      'state-set-done',
      'finished',
    ]);
    expect(useAuthStore.getState().isAuthenticated).toBe(true);
    expect(JSON.stringify(checkpoints)).not.toContain('do-not-record');
  });
  it('records a failed login without recording the error and guards throwing bridge accessors', async () => {
    vi.mocked(invoke).mockRejectedValue(Error('private-password-error'));
    await useAuthStore.getState().login('account', 'password');
    expect(checkpoints.map((x) => x[1])).toEqual([
      'login-await-start',
      'login-await-error',
      'failed',
    ]);
    expect(useAuthStore.getState().isAuthenticated).toBe(false);
    expect(JSON.stringify(checkpoints)).not.toContain('private-password-error');
    host.__SOLOSOUL_NATIVE_PERF__ = Object.defineProperty({}, 'beginAuthFlow', {
      get: () => {
        throw Error('observer-getter');
      },
    });
    vi.mocked(invoke).mockResolvedValue(undefined);
    await useAuthStore.getState().login('account', 'password');
    expect(useAuthStore.getState().isAuthenticated).toBe(true);
  });
  it('keeps refresh failure as a successful login and observer errors cannot reject login', async () => {
    vi.mocked(invoke).mockImplementation(async (cmd) => {
      if (cmd === 'vault_list_accounts') throw Error('private-error');
    });
    await useAuthStore.getState().login('account', 'password');
    expect(checkpoints.map((x) => x[1])).toContain('accounts-await-error');
    expect(useAuthStore.getState().isAuthenticated).toBe(true);
    host.__SOLOSOUL_NATIVE_PERF__ = {
      beginAuthFlow: () => {
        throw Error('observer-error');
      },
      markAuthFlow: () => {},
      authSnapshot: () => ({}),
    };
    vi.mocked(invoke).mockResolvedValue(undefined);
    await useAuthStore.getState().login('account', 'password');
    expect(useAuthStore.getState().isAuthenticated).toBe(true);
  });
});
