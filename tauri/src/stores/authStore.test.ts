import { describe, it, expect, vi, beforeEach } from 'vitest';
import { useAuthStore } from './authStore';

// Mock the IPC commands module
vi.mock('@/lib/ipc', () => ({
  commands: {
    checkHasAccount: vi.fn(),
    vaultListAccounts: vi.fn(),
    bootstrap: vi.fn(),
    login: vi.fn(),
    logout: vi.fn(),
  },
}));

import { commands } from '@/lib/ipc';

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
      vi.mocked(commands.checkHasAccount).mockResolvedValue(true);
      await useAuthStore.getState().checkHasAccount();
      expect(useAuthStore.getState().hasAccount).toBe(true);
      expect(useAuthStore.getState().backendError).toBe(false);
    });

    it('should set hasAccount to false when backend returns false', async () => {
      vi.mocked(commands.checkHasAccount).mockResolvedValue(false);
      await useAuthStore.getState().checkHasAccount();
      expect(useAuthStore.getState().hasAccount).toBe(false);
      expect(useAuthStore.getState().backendError).toBe(false);
    });

    it('should set backendError on exception', async () => {
      vi.mocked(commands.checkHasAccount).mockRejectedValue(new Error('backend down'));
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
      vi.mocked(commands.vaultListAccounts).mockResolvedValue(accounts);
      await useAuthStore.getState().listAccounts();
      expect(useAuthStore.getState().accounts).toEqual(accounts);
      expect(useAuthStore.getState().hasAccount).toBe(true);
    });

    it('should preserve currentAccount if still present in refreshed list', async () => {
      useAuthStore.setState({ currentAccount: { id: 'acc-1', name: 'Alice' } });
      const refreshed = [{ id: 'acc-1', name: 'Alice Updated' }, { id: 'acc-2', name: 'Bob' }];
      vi.mocked(commands.vaultListAccounts).mockResolvedValue(refreshed);
      await useAuthStore.getState().listAccounts();
      expect(useAuthStore.getState().currentAccount).toEqual({ id: 'acc-1', name: 'Alice Updated' });
    });

    it('should silently fail when vault is locked', async () => {
      vi.mocked(commands.vaultListAccounts).mockRejectedValue(new Error('locked'));
      await useAuthStore.getState().listAccounts();
      expect(useAuthStore.getState().accounts).toEqual([]);
    });
  });

  describe('bootstrap', () => {
    it('should create account and set authenticated state', async () => {
      const account = { id: 'new-acc', name: 'Charlie' };
      vi.mocked(commands.bootstrap).mockResolvedValue(account);
      await useAuthStore.getState().bootstrap('Charlie', 'password123', 'en-US');
      expect(useAuthStore.getState().isAuthenticated).toBe(true);
      expect(useAuthStore.getState().currentAccount).toEqual(account);
      expect(useAuthStore.getState().accounts).toEqual([account]);
      expect(useAuthStore.getState().hasAccount).toBe(true);
      expect(useAuthStore.getState().isLoading).toBe(false);
    });

    it('should set error on bootstrap failure', async () => {
      vi.mocked(commands.bootstrap).mockRejectedValue(new Error('name taken'));
      await useAuthStore.getState().bootstrap('Charlie', 'password123', 'en-US');
      expect(useAuthStore.getState().isAuthenticated).toBe(false);
      expect(useAuthStore.getState().error).toBe('Error: name taken');
      expect(useAuthStore.getState().isLoading).toBe(false);
    });
  });

  describe('login', () => {
    it('should authenticate and load accounts', async () => {
      vi.mocked(commands.login).mockResolvedValue(undefined);
      const accounts = [{ id: 'acc-1', name: 'Alice' }];
      vi.mocked(commands.vaultListAccounts).mockResolvedValue(accounts);
      await useAuthStore.getState().login('acc-1', 'password123');
      expect(useAuthStore.getState().isAuthenticated).toBe(true);
      expect(useAuthStore.getState().currentAccount).toEqual({ id: 'acc-1', name: 'Alice' });
      expect(useAuthStore.getState().accounts).toEqual(accounts);
      expect(useAuthStore.getState().isLoading).toBe(false);
    });

    it('should use fallback account info when list returns empty', async () => {
      vi.mocked(commands.login).mockResolvedValue(undefined);
      vi.mocked(commands.vaultListAccounts).mockResolvedValue([]);
      await useAuthStore.getState().login('acc-1', 'password123');
      expect(useAuthStore.getState().currentAccount).toEqual({ id: 'acc-1', name: 'acc-1' });
    });

    it('should set error on login failure', async () => {
      vi.mocked(commands.login).mockRejectedValue(new Error('wrong password'));
      await useAuthStore.getState().login('acc-1', 'wrong');
      expect(useAuthStore.getState().isAuthenticated).toBe(false);
      expect(useAuthStore.getState().error).toBe('Error: wrong password');
      expect(useAuthStore.getState().isLoading).toBe(false);
    });
  });

  describe('logout', () => {
    it('should clear authenticated state', async () => {
      useAuthStore.setState({ isAuthenticated: true, currentAccount: { id: 'acc-1', name: 'Alice' } });
      vi.mocked(commands.logout).mockResolvedValue(undefined);
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
