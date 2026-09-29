import { act, renderHook } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { useAuthStore } from '@/stores/authStore';
import { useLoginUnlockFlows } from './useLoginUnlockFlows';

describe('existing-directory password unlock', () => {
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

  function renderFlow() {
    return renderHook(
      () =>
        useLoginUnlockFlows({
          selectedAccountId: 'acc-a',
          fromExisting: true,
          bioLockout: false,
          biometryTypeRaw: 'touchId',
          setLoginMethod: vi.fn(),
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
});
