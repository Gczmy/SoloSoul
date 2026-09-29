import { act, renderHook, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { useAuthStore, LAST_ACCOUNT_KEY } from '@/stores/authStore';
import {
  LOGIN_METHOD_CACHE_KEY,
  readCachedLoginMethod,
  writeCachedLoginMethod,
} from '@/lib/loginMethodCache';
import {
  preflightLoginAvailability,
  type LoginAvailability,
} from '@/lib/loginAvailabilityPreflight';
import { useLoginPage } from './useLoginPage';

vi.mock('@/hooks/useApplyThemeFromSettings', () => ({ useApplyThemeFromSettings: vi.fn() }));
vi.mock('@/lib/loginAvailabilityPreflight', () => ({
  normalizeBiometryType: (raw?: string) => raw ?? 'touchId',
  preflightLoginAvailability: vi.fn(),
}));

const accounts = [
  { id: 'acc-a', name: 'Account A' },
  { id: 'acc-b', name: 'Account B' },
];

describe('useLoginPage account-scoped method selection', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    localStorage.clear();
    localStorage.setItem(LAST_ACCOUNT_KEY, 'acc-a');
    writeCachedLoginMethod('acc-a', 'pin');
    useAuthStore.setState({
      accounts,
      hasAccount: true,
      isAuthenticated: false,
      checkHasAccount: vi.fn().mockResolvedValue(true),
      listAccounts: vi.fn().mockResolvedValue(accounts),
    });
    vi.mocked(invoke).mockImplementation(async (command) => {
      if (command === 'vault_list_accounts') return accounts;
      if (command === 'biometric_check_availability') return { available: false };
      return undefined;
    });
  });

  it('does not display or persist account A’s PIN while B is being probed', async () => {
    let resolveB!: (availability: LoginAvailability) => void;
    vi.mocked(preflightLoginAvailability).mockImplementation((accountId) => {
      if (accountId === 'acc-a') {
        return Promise.resolve({
          bioAvailable: false,
          bioLockout: false,
          biometryTypeRaw: 'touchId',
          pinAvailable: true,
        });
      }
      return new Promise((resolve) => {
        resolveB = resolve;
      });
    });

    const { result } = renderHook(() => useLoginPage(), { wrapper: MemoryRouter });
    await waitFor(() => {
      expect(result.current.selectedAccountId).toBe('acc-a');
      expect(result.current.iconMethods.map((method) => method.id)).toContain('pin');
    });

    act(() => result.current.setSelectedAccountId('acc-b'));

    expect(result.current.loginMethod).toBeNull();
    expect(readCachedLoginMethod('acc-b')).toBeNull();
    expect(JSON.parse(localStorage.getItem(LOGIN_METHOD_CACHE_KEY) || '{}')).toEqual({
      accountId: 'acc-a',
      method: 'pin',
    });

    await act(async () => {
      resolveB({
        bioAvailable: false,
        bioLockout: false,
        biometryTypeRaw: 'touchId',
        pinAvailable: false,
      });
    });
    await waitFor(() => expect(result.current.loginMethod).toBe('password'));
    expect(readCachedLoginMethod('acc-b')).toBe('password');
  });
});
