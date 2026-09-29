import { act, renderHook, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { invokeCommand } from '@/lib/ipcClient';
import { setRequestSession } from '@/lib/sessionRequests';
import { useWorkspacePasswordGuard } from './useWorkspacePasswordGuard';

vi.mock('@/lib/ipcClient', () => ({ invokeCommand: vi.fn() }));

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}

beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(invokeCommand).mockImplementation(async (command) => {
    if (command === 'biometric_check_availability')
      return { available: false, configured: false } as never;
    if (command === 'vault_list_accounts') return [] as never;
    return undefined as never;
  });
});

afterEach(() => act(() => setRequestSession(null)));

it('does not apply old account biometric availability or password hints after a switch', async () => {
  const oldBio = deferred<{ available: boolean; configured: boolean; biometryType: string }>();
  const oldAccounts = deferred<{ id: string; passwordHint: string }[]>();
  let accountListCalls = 0;
  vi.mocked(invokeCommand).mockImplementation((command, args) => {
    if (command === 'biometric_check_availability') {
      return (
        (args as { accountId: string }).accountId === 'a'
          ? oldBio.promise
          : Promise.resolve({ available: false, configured: false })
      ) as never;
    }
    if (command === 'vault_list_accounts') {
      return (
        accountListCalls++ === 0
          ? oldAccounts.promise
          : Promise.resolve([{ id: 'b', passwordHint: 'B hint' }])
      ) as never;
    }
    return Promise.resolve(undefined) as never;
  });
  setRequestSession('a');
  const { result, rerender } = renderHook(({ accountId }) => useWorkspacePasswordGuard(accountId), {
    initialProps: { accountId: 'a' as string | undefined },
  });
  await waitFor(() => expect(accountListCalls).toBe(1));

  act(() => setRequestSession('b'));
  rerender({ accountId: 'b' });
  await waitFor(() => expect(result.current.passwordHint).toBe('B hint'));
  await act(async () => {
    oldBio.resolve({ available: true, configured: true, biometryType: 'touchId' });
    oldAccounts.resolve([{ id: 'a', passwordHint: 'A hint' }]);
    await Promise.all([oldBio.promise, oldAccounts.promise]);
  });
  expect(result.current.bioAvailable.available).toBe(false);
  expect(result.current.passwordHint).toBe('B hint');

  act(() => setRequestSession(null));
  rerender({ accountId: undefined });
  expect(result.current.bioAvailable.available).toBe(false);
  expect(result.current.passwordHint).toBeNull();
});

it('cancels a pending verification on account switch and rejects a late biometric success', async () => {
  const oldUnlock = deferred<void>();
  vi.mocked(invokeCommand).mockImplementation(async (command) => {
    if (command === 'biometric_check_availability')
      return { available: true, configured: true, biometryType: 'touchId' } as never;
    if (command === 'vault_list_accounts') return [] as never;
    if (command === 'biometric_unlock') return oldUnlock.promise as never;
    return undefined as never;
  });
  setRequestSession('a');
  const { result, rerender } = renderHook(({ accountId }) => useWorkspacePasswordGuard(accountId), {
    initialProps: { accountId: 'a' },
  });
  await waitFor(() => expect(result.current.bioAvailable.available).toBe(true));
  let oldVerification!: Promise<{ ok: boolean }>;
  let oldAction!: Promise<boolean>;
  act(() => {
    oldVerification = result.current.passwordVerify();
    oldAction = result.current.handleBiometricUnlock();
  });

  act(() => setRequestSession('b'));
  rerender({ accountId: 'b' });
  await waitFor(() => expect(result.current.showPwDialog).toBe(false));
  expect(await oldVerification).toMatchObject({ ok: false });

  let nextVerification!: Promise<{ ok: boolean }>;
  act(() => {
    nextVerification = result.current.passwordVerify();
  });
  await act(async () => {
    oldUnlock.resolve();
    await oldUnlock.promise;
  });
  expect(await oldAction).toBe(false);
  expect(result.current.showPwDialog).toBe(true);
  act(() => result.current.handlePwDialogClose());
  expect(await nextVerification).toMatchObject({ ok: false });
});

it('does not let an old password result authorize the next account verification', async () => {
  const oldUnlock = deferred<void>();
  vi.mocked(invokeCommand).mockImplementation(async (command) => {
    if (command === 'biometric_check_availability')
      return { available: false, configured: false } as never;
    if (command === 'vault_list_accounts') return [] as never;
    if (command === 'unlock_with_password') return oldUnlock.promise as never;
    return undefined as never;
  });
  setRequestSession('a');
  const { result, rerender } = renderHook(({ accountId }) => useWorkspacePasswordGuard(accountId), {
    initialProps: { accountId: 'a' },
  });
  let oldVerification!: Promise<{ ok: boolean }>;
  let oldAttempt!: Promise<boolean>;
  act(() => {
    oldVerification = result.current.passwordVerify();
    oldAttempt = result.current.handlePwDialogVerify('secret');
  });

  act(() => setRequestSession('b'));
  rerender({ accountId: 'b' });
  expect(await oldVerification).toMatchObject({ ok: false });
  let nextVerification!: Promise<{ ok: boolean }>;
  act(() => {
    nextVerification = result.current.passwordVerify();
  });
  await act(async () => {
    oldUnlock.resolve();
    await oldUnlock.promise;
  });
  expect(await oldAttempt).toBe(false);
  expect(result.current.showPwDialog).toBe(true);
  act(() => result.current.handlePwDialogClose());
  expect(await nextVerification).toMatchObject({ ok: false });
});

it('rejects a PIN callback retained from a previous verification', async () => {
  setRequestSession('a');
  const { result } = renderHook(() => useWorkspacePasswordGuard('a'));
  let oldVerification!: Promise<{ ok: boolean }>;
  act(() => {
    oldVerification = result.current.passwordVerify();
  });
  const oldPinSuccess = result.current.handlePwDialogPinSuccess;
  act(() => result.current.handlePwDialogClose());
  expect(await oldVerification).toMatchObject({ ok: false });

  let nextVerification!: Promise<{ ok: boolean }>;
  act(() => {
    nextVerification = result.current.passwordVerify();
  });
  act(() => oldPinSuccess());
  expect(result.current.showPwDialog).toBe(true);
  act(() => result.current.handlePwDialogClose());
  expect(await nextVerification).toMatchObject({ ok: false });
});

it('reports PIN as the verification method for a current request', async () => {
  setRequestSession('a');
  const { result } = renderHook(() => useWorkspacePasswordGuard('a'));
  let verification!: ReturnType<typeof result.current.passwordVerify>;
  act(() => {
    verification = result.current.passwordVerify();
  });
  act(() => result.current.handlePwDialogPinSuccess());
  expect(await verification).toEqual({ ok: true, method: 'pin' });
});

it('reports Windows Hello as the verification method for a current request', async () => {
  vi.mocked(invokeCommand).mockImplementation(async (command) => {
    if (command === 'biometric_check_availability')
      return { available: true, configured: true, biometryType: 'windowsHello' } as never;
    if (command === 'vault_list_accounts') return [] as never;
    return undefined as never;
  });
  setRequestSession('a');
  const { result } = renderHook(() => useWorkspacePasswordGuard('a'));
  await waitFor(() => expect(result.current.bioAvailable.biometryType).toBe('windowsHello'));
  let verification!: ReturnType<typeof result.current.passwordVerify>;
  act(() => {
    verification = result.current.passwordVerify();
  });
  await act(async () => {
    expect(await result.current.handleBiometricUnlock()).toBe(true);
  });
  expect(await verification).toEqual({ ok: true, method: 'windowsHello' });
});
