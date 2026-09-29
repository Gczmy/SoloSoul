import { act, renderHook, waitFor } from '@testing-library/react';
import { expect, it, vi } from 'vitest';
import { invokeCommand } from '@/lib/ipcClient';
import { usePasswordVerification } from './usePasswordVerification';

vi.mock('@/lib/ipcClient', () => ({ invokeCommand: vi.fn() }));

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}

it('does not show the previous account PIN when its availability check finishes late', async () => {
  const oldAvailability = deferred<{ configured: boolean; locked: boolean }>();
  vi.mocked(invokeCommand).mockImplementation((command, args) => {
    if (command !== 'pin_check_availability') return Promise.resolve(undefined) as never;
    return (
      (args as { accountId: string }).accountId === 'a'
        ? oldAvailability.promise
        : Promise.resolve({ configured: false, locked: false })
    ) as never;
  });
  const common = { open: true, onClose: vi.fn(), onVerify: vi.fn(async () => false) };
  const { result, rerender } = renderHook(
    ({ pinAccountId }) => usePasswordVerification({ ...common, pinAccountId }),
    { initialProps: { pinAccountId: 'a' } },
  );
  rerender({ pinAccountId: 'b' });
  await waitFor(() => expect(result.current.loginMethod).toBe('password'));

  await act(async () => {
    oldAvailability.resolve({ configured: true, locked: false });
    await oldAvailability.promise;
  });
  expect(result.current.loginMethod).toBe('password');
  expect(result.current.methods.some((method) => method.id === 'pin')).toBe(false);
});
