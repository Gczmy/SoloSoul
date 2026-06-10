import { describe, it, expect, vi, beforeEach } from 'vitest';
import { useVaultStore } from './vaultStore';

vi.mock('@/lib/ipc', () => ({
  commands: {
    vaultGetState: vi.fn(),
    vaultUnlock: vi.fn(),
    vaultLock: vi.fn(),
  },
}));

import { commands } from '@/lib/ipc';

describe('vaultStore', () => {
  beforeEach(() => {
    useVaultStore.setState({
      vaultState: 'locked',
      accounts: [],
      isLoading: false,
      error: null,
    });
    vi.clearAllMocks();
  });

  describe('loadAccounts', () => {
    it('should load vault state successfully', async () => {
      vi.mocked(commands.vaultGetState).mockResolvedValue('unlocked');
      await useVaultStore.getState().loadAccounts();
      expect(useVaultStore.getState().vaultState).toBe('unlocked');
      expect(useVaultStore.getState().isLoading).toBe(false);
      expect(useVaultStore.getState().error).toBeNull();
    });

    it('should handle error during load', async () => {
      vi.mocked(commands.vaultGetState).mockRejectedValue(new Error('db corrupt'));
      await useVaultStore.getState().loadAccounts();
      expect(useVaultStore.getState().vaultState).toBe('locked');
      expect(useVaultStore.getState().error).toBe('Error: db corrupt');
      expect(useVaultStore.getState().isLoading).toBe(false);
    });
  });

  describe('unlock', () => {
    it('should unlock vault and update state', async () => {
      vi.mocked(commands.vaultUnlock).mockResolvedValue(undefined);
      await useVaultStore.getState().unlock('acc-1', 'password123');
      expect(useVaultStore.getState().vaultState).toBe('unlocked');
      expect(useVaultStore.getState().isLoading).toBe(false);
      expect(useVaultStore.getState().error).toBeNull();
    });

    it('should set error on wrong password', async () => {
      vi.mocked(commands.vaultUnlock).mockRejectedValue(new Error('wrong password'));
      await useVaultStore.getState().unlock('acc-1', 'wrong');
      expect(useVaultStore.getState().vaultState).toBe('locked');
      expect(useVaultStore.getState().error).toBe('Error: wrong password');
      expect(useVaultStore.getState().isLoading).toBe(false);
    });
  });

  describe('lock', () => {
    it('should lock vault', async () => {
      useVaultStore.setState({ vaultState: 'unlocked' });
      vi.mocked(commands.vaultLock).mockResolvedValue(undefined);
      await useVaultStore.getState().lock();
      expect(useVaultStore.getState().vaultState).toBe('locked');
    });
  });
});
