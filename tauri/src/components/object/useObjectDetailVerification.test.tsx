import { act, renderHook } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import { invokeCommand } from '@/lib/ipcClient';
import { useObjectDetailVerification } from './useObjectDetailVerification';

vi.mock('@/lib/ipcClient', () => ({ invokeCommand: vi.fn() }));
beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(invokeCommand).mockImplementation(async (cmd) => {
    if (cmd === 'vault_list_accounts') return [] as never;
    if (cmd === 'biometric_check_availability') return { available: false } as never;
    return undefined as never;
  });
});
const options = { accountId: 'a', obj: null, resolveCollectionLabelLocal: () => 'Page' };
it('critical reveal waits for authentication; cancellation never authorizes it', async () => {
  const { result } = renderHook(() => useObjectDetailVerification(options));
  let outcome!: Promise<boolean>;
  act(() => {
    outcome = result.current.handleRevealField('field', 'critical', 'Secret');
  });
  expect(result.current.showPwDialog).toBe(true);
  expect(result.current.isRevealed('field')).toBe(false);
  act(() => result.current.handlePwDialogClose());
  expect(await outcome).toBe(false);
  expect(result.current.isRevealed('field')).toBe(false);
  act(() => {
    outcome = result.current.handleRevealField('field', 'critical', 'Secret');
  });
  await act(async () => {
    expect(await result.current.handlePwDialogVerify('test')).toBe(true);
    await outcome;
  });
  expect(result.current.isRevealed('field')).toBe(true);
});
it('unmounting a pending verification resolves it as cancelled', async () => {
  const { result, unmount } = renderHook(() => useObjectDetailVerification(options));
  let outcome!: Promise<boolean>;
  act(() => {
    outcome = result.current.handleRevealField('field', 'critical', 'Secret');
  });
  unmount();
  expect(await outcome).toBe(false);
});
it('a late password response cannot authorize a replacement copy request', async () => {
  const { result } = renderHook(() => useObjectDetailVerification(options));
  let finish!: () => void;
  vi.mocked(invokeCommand).mockImplementation(async (cmd) => {
    if (cmd === 'unlock_with_password')
      await new Promise<void>((resolve) => {
        finish = resolve;
      });
    return undefined as never;
  });
  let first!: Promise<boolean>, second!: Promise<boolean>, verify!: Promise<boolean>;
  act(() => {
    first = result.current.handleRevealField('one', 'critical', 'One');
  });
  act(() => {
    verify = result.current.handlePwDialogVerify('test');
  });
  act(() => {
    second = result.current.handleRevealField('two', 'critical', 'Two');
  });
  expect(await first).toBe(false);
  await act(async () => {
    finish();
    expect(await verify).toBe(false);
  });
  expect(result.current.isRevealed('two')).toBe(false);
  act(() => result.current.handlePwDialogClose());
  expect(await second).toBe(false);
});

it('a late PIN callback cannot authorize a replacement field request', async () => {
  const { result } = renderHook(() => useObjectDetailVerification(options));
  let first!: Promise<boolean>, second!: Promise<boolean>;
  act(() => {
    first = result.current.handleRevealField('one', 'critical', 'One');
  });
  const oldPinSuccess = result.current.handlePwDialogPinSuccess;
  act(() => result.current.handlePwDialogClose());
  expect(await first).toBe(false);
  act(() => {
    second = result.current.handleRevealField('two', 'critical', 'Two');
  });
  await act(async () => oldPinSuccess());
  expect(result.current.isRevealed('two')).toBe(false);
  act(() => result.current.handlePwDialogClose());
  expect(await second).toBe(false);
});
