import type { PropsWithChildren } from 'react';
import { MemoryRouter } from 'react-router-dom';
import { act, renderHook } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import { useRecoveryReceive } from './useRecoveryReceive';
import type { RecoveryResultSummary } from '@/components/recovery/recoveryReceiveTypes';

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  checkHasAccount: vi.fn().mockResolvedValue(undefined),
  saveLast: vi.fn(),
}));
vi.mock('@/lib/ipcClient', () => ({ invokeCommand: mocks.invoke }));
vi.mock('react-i18next', async (importOriginal) => {
  const actual = await importOriginal<typeof import('react-i18next')>();
  const { default: engine } = await import('i18next');
  const t = engine.t.bind(engine);
  return { ...actual, useTranslation: () => ({ t, i18n: engine }) };
});
vi.mock('@/hooks/useCameraCapability', () => ({ useCameraCapability: () => 'unsupported' }));
vi.mock('@/stores/authStore', () => ({
  saveLastAccountId: mocks.saveLast,
  useAuthStore: { getState: () => ({ checkHasAccount: mocks.checkHasAccount }) },
}));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn().mockResolvedValue(() => {}) }));

beforeEach(() => vi.clearAllMocks());

it.each(['complete', 'partial', 'notCommitted'] as const)(
  'RF-020 recovery shows success only for complete imports (%s)',
  async (status) => {
    const summary: RecoveryResultSummary = {
      sessionGeneration: 7,
      status,
      accountId: 'account',
      accountName: 'Synthetic',
      objectCount: status === 'notCommitted' ? 0 : 1,
      attachmentCount: 0,
      templateCount: 0,
      snapshotCount: 0,
      preferencesImported: false,
      attachmentFilesWritten: 0,
      failureStage: status === 'complete' ? null : 'objects',
      errorCode: status === 'complete' ? null : 'IMPORT_FAILED',
    };
    mocks.invoke.mockResolvedValue(summary);
    const wrapper = ({ children }: PropsWithChildren) => <MemoryRouter>{children}</MemoryRouter>;
    const { result } = renderHook(() => useRecoveryReceive({ isOpen: true, onClose: vi.fn() }), {
      wrapper,
    });
    act(() => {
      result.current.setHostAddr('127.0.0.1:12545');
      result.current.setPin('123456');
    });
    act(() => result.current.handleManualNext());
    expect(result.current.step).toBe('account');
    act(() => {
      result.current.handleMasterPasswordChange('password123');
      result.current.handleConfirmPasswordChange('password123');
    });
    await act(async () => {
      await result.current.handleStartRecovery();
    });
    expect(mocks.invoke).toHaveBeenCalledWith('recovery_restore_from_host', expect.any(Object));
    expect(mocks.checkHasAccount).toHaveBeenCalledOnce();
    expect(result.current.successConfirmOpen).toBe(status === 'complete');
    if (status === 'complete') {
      expect(result.current.step).toBe('success');
      expect(mocks.saveLast).toHaveBeenCalledWith('account');
    } else {
      expect(result.current.step).toBe('account');
      expect(result.current.success).toBeNull();
      expect(result.current.error).toBeTruthy();
      expect(mocks.saveLast).not.toHaveBeenCalled();
    }
  },
);

it('keeps the latest scanned recovery account when an earlier conflict check finishes later', async () => {
  const checks = new Map<string, (accounts: Array<{ id: string }>) => void>();
  mocks.invoke.mockImplementation((command: string) => {
    if (command !== 'vault_list_accounts') throw new Error(`Unexpected command: ${command}`);
    return new Promise((resolve) => {
      checks.set(String(checks.size), resolve);
    });
  });
  const wrapper = ({ children }: PropsWithChildren) => <MemoryRouter>{children}</MemoryRouter>;
  const { result } = renderHook(() => useRecoveryReceive({ isOpen: true, onClose: vi.fn() }), {
    wrapper,
  });

  let first!: Promise<void>;
  let second!: Promise<void>;
  act(() => {
    first = result.current.handleScan(
      JSON.stringify({ t: 'rec', a: 'host-a', p: '123456', u: 'a' }),
    );
    second = result.current.handleScan(
      JSON.stringify({ t: 'rec', a: 'host-b', p: '654321', u: 'b' }),
    );
  });
  await act(async () => {
    checks.get('1')?.([{ id: 'b' }]);
    await second;
  });
  expect(result.current.pending?.accountId).toBe('b');
  expect(result.current.idConflict).toBe(true);

  await act(async () => {
    checks.get('0')?.([]);
    await first;
  });
  expect(result.current.pending?.accountId).toBe('b');
  expect(result.current.idConflict).toBe(true);
});

it('ignores a scanned account after leaving the scan flow', async () => {
  let finishCheck!: (accounts: Array<{ id: string }>) => void;
  mocks.invoke.mockImplementation(
    () =>
      new Promise((resolve) => {
        finishCheck = resolve;
      }),
  );
  const wrapper = ({ children }: PropsWithChildren) => <MemoryRouter>{children}</MemoryRouter>;
  const { result } = renderHook(() => useRecoveryReceive({ isOpen: true, onClose: vi.fn() }), {
    wrapper,
  });

  let scan!: Promise<void>;
  act(() => {
    scan = result.current.handleScan(
      JSON.stringify({ t: 'rec', a: 'old-host', p: '123456', u: 'old-account' }),
    );
    result.current.switchTab('manual');
  });
  await act(async () => {
    finishCheck([]);
    await scan;
  });
  expect(result.current.tab).toBe('manual');
  expect(result.current.step).toBe('collect');
  expect(result.current.pending).toBeNull();
});

it('does not apply a conflict check from a previous dialog opening', async () => {
  let finishCheck!: (accounts: Array<{ id: string }>) => void;
  mocks.invoke.mockImplementation(
    () =>
      new Promise((resolve) => {
        finishCheck = resolve;
      }),
  );
  const wrapper = ({ children }: PropsWithChildren) => <MemoryRouter>{children}</MemoryRouter>;
  const { result, rerender } = renderHook(
    ({ isOpen }) => useRecoveryReceive({ isOpen, onClose: vi.fn() }),
    { wrapper, initialProps: { isOpen: true } },
  );

  let scan!: Promise<void>;
  act(() => {
    scan = result.current.handleScan(
      JSON.stringify({ t: 'rec', a: 'old-host', p: '123456', u: 'old-account' }),
    );
  });
  rerender({ isOpen: false });
  rerender({ isOpen: true });
  await act(async () => {
    finishCheck([]);
    await scan;
  });
  expect(result.current.step).toBe('collect');
  expect(result.current.pending).toBeNull();
});

it('keeps recovery open until an in-progress import finishes', async () => {
  let finishRestore!: (summary: RecoveryResultSummary) => void;
  mocks.invoke.mockImplementation((command: string) => {
    if (command !== 'recovery_restore_from_host') throw new Error(`Unexpected command: ${command}`);
    return new Promise((resolve) => {
      finishRestore = resolve;
    });
  });
  const onClose = vi.fn();
  const wrapper = ({ children }: PropsWithChildren) => <MemoryRouter>{children}</MemoryRouter>;
  const { result } = renderHook(() => useRecoveryReceive({ isOpen: true, onClose }), { wrapper });
  act(() => {
    result.current.setHostAddr('127.0.0.1:12545');
    result.current.setPin('123456');
  });
  act(() => result.current.handleManualNext());
  act(() => {
    result.current.handleMasterPasswordChange('password123');
    result.current.handleConfirmPasswordChange('password123');
  });

  let recovery!: Promise<void>;
  act(() => {
    recovery = result.current.handleStartRecovery();
  });
  expect(result.current.loading).toBe(true);
  act(() => result.current.handleClose());
  expect(onClose).not.toHaveBeenCalled();
  expect(result.current.step).toBe('account');

  await act(async () => {
    finishRestore({
      sessionGeneration: 7,
      status: 'complete',
      accountId: 'restored-account',
      accountName: 'Synthetic',
      objectCount: 1,
      attachmentCount: 0,
      templateCount: 0,
      snapshotCount: 0,
      preferencesImported: false,
      attachmentFilesWritten: 0,
      failureStage: null,
      errorCode: null,
    });
    await recovery;
  });
  expect(result.current.step).toBe('success');
  expect(result.current.successConfirmOpen).toBe(true);
  expect(onClose).not.toHaveBeenCalled();
  act(() => result.current.handleClose());
  expect(onClose).toHaveBeenCalledOnce();
});
