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
