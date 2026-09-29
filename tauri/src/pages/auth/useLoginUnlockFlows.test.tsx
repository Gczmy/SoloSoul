import { act, renderHook } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { useAuthStore } from '@/stores/authStore';
import { useLoginUnlockFlows } from './useLoginUnlockFlows';

describe('login unlock flows', () => {
  const listAccounts = vi.fn().mockResolvedValue([]);

  beforeEach(() => {
    vi.mocked(invoke).mockReset().mockResolvedValue(undefined);
    listAccounts.mockClear();
    useAuthStore.setState({
      currentAccount: null,
      isAuthenticated: false,
      error: null,
      clearError: vi.fn(() => useAuthStore.setState({ error: null })),
      listAccounts,
    });
  });

  function renderFlow(setLoginMethod = vi.fn()) {
    return renderHook(
      () =>
        useLoginUnlockFlows({
          selectedAccountId: 'acc-a',
          fromExisting: true,
          bioLockout: false,
          biometryTypeRaw: 'touchId',
          setLoginMethod,
          setBioLockout: vi.fn(),
          setPinAvailable: vi.fn(),
        }),
      { wrapper: MemoryRouter },
    );
  }

  it('does not clear security credentials after an invalid master password', async () => {
    useAuthStore.setState({
      login: vi.fn(async () => useAuthStore.setState({ error: 'Invalid password' })),
    });
    const { result } = renderFlow();

    act(() => result.current.setPassword('wrong password'));
    await act(async () => result.current.handleSubmit());

    expect(result.current.passwordFieldError).toBe('common:invalid_password');
    expect(invoke).not.toHaveBeenCalledWith('reset_security_flags', {
      accountId: 'acc-a',
    });
    expect(listAccounts).not.toHaveBeenCalled();
  });

  it('resets stale security flags only after successful password unlock', async () => {
    useAuthStore.setState({
      login: vi.fn(async () =>
        useAuthStore.setState({
          isAuthenticated: true,
          currentAccount: { id: 'acc-a', name: 'Account A' },
        }),
      ),
    });
    const { result } = renderFlow();

    act(() => result.current.setPassword('correct password'));
    await act(async () => result.current.handleSubmit());

    expect(invoke).toHaveBeenCalledWith('reset_security_flags', {
      accountId: 'acc-a',
    });
    expect(listAccounts).toHaveBeenCalledTimes(1);
  });

  it('keeps biometric unlock successful when account-list refresh fails', async () => {
    const account = { id: 'acc-a', name: 'Account A' };
    const setLoginMethod = vi.fn();
    useAuthStore.setState({ accounts: [account] });
    vi.mocked(invoke).mockImplementation(async (command) => {
      if (command === 'vault_list_accounts') throw new Error('refresh unavailable');
      return undefined;
    });
    const { result } = renderFlow(setLoginMethod);

    await act(async () => result.current.handleBiometricUnlock());

    expect(invoke).toHaveBeenCalledWith('biometric_unlock', {
      accountId: 'acc-a',
      location: 'login_page',
      action: 'unlock',
      biometryType: 'touchId',
    });
    expect(useAuthStore.getState().isAuthenticated).toBe(true);
    expect(useAuthStore.getState().currentAccount).toEqual(account);
    expect(result.current.bioError).toBeNull();
    expect(setLoginMethod).not.toHaveBeenCalledWith('password');
  });

  it('does not authenticate when native biometric unlock itself fails', async () => {
    vi.mocked(invoke).mockRejectedValueOnce(new Error('__BIO_ERR__:invalid_password'));
    const setLoginMethod = vi.fn();
    const { result } = renderFlow(setLoginMethod);

    await act(async () => result.current.handleBiometricUnlock());

    expect(invoke).toHaveBeenCalledTimes(1);
    expect(useAuthStore.getState().isAuthenticated).toBe(false);
    expect(result.current.bioError).not.toBeNull();
    expect(setLoginMethod).toHaveBeenCalledWith('password');
  });
});
