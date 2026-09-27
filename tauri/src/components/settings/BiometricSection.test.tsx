import { beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import type { PasswordVerificationDialogProps } from '@/components/forms/PasswordVerificationDialog';
import { BiometricSection } from './BiometricSection';

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  update: vi.fn(),
  success: vi.fn(),
  invalidate: vi.fn(),
  clearMethod: vi.fn(),
  verified: vi.fn(),
  configured: false,
}));
vi.mock('@/lib/ipcClient', () => ({ invokeCommand: mocks.invoke }));
vi.mock('@/hooks/useSettingAction', () => ({ useSettingAction: () => mocks.update }));
vi.mock('@/hooks/useToastError', () => ({
  useToastError: () => ({ onSuccess: mocks.success, onError: vi.fn() }),
}));
vi.mock('@/lib/loginAvailabilityPreflight', () => ({
  invalidateLoginAvailabilityPreflight: mocks.invalidate,
}));
vi.mock('@/lib/loginMethodCache', () => ({ clearCachedLoginMethod: mocks.clearMethod }));
vi.mock('@/stores/authStore', () => ({
  useAuthStore: (
    select: (state: { currentAccount: { id: string; passwordHint: null } }) => unknown,
  ) => select({ currentAccount: { id: 'acc-a', passwordHint: null } }),
}));
vi.mock('@/components/forms/PasswordVerificationDialog', () => ({
  PasswordVerificationDialog: (props: PasswordVerificationDialogProps) =>
    props.open ? (
      <div role="dialog">
        <span>{props.errorMessage}</span>
        <button
          onClick={async () => {
            const result = await props.onVerify('synthetic-password');
            mocks.verified(result);
            if (result) props.onClose();
          }}
        >
          Verify test credential
        </button>
      </div>
    ) : null,
}));

beforeEach(() => {
  cleanup();
  vi.clearAllMocks();
  mocks.configured = false;
  mocks.invoke.mockImplementation(async (command: string) => {
    if (command === 'biometric_check_availability')
      return {
        available: true,
        strongAvailable: true,
        weakAvailable: false,
        strongConfigured: mocks.configured,
        weakConfigured: false,
        biometryType: 'touchId',
      };
    if (command === 'biometric_save_credential') mocks.configured = true;
    if (command === 'biometric_delete_credential') mocks.configured = false;
  });
});

describe('RF111 生物识别凭证已改变但设置保存失败', () => {
  it.each([
    ['enable', 'failed'],
    ['disable', 'failed'],
    ['enable', 'stale'],
    ['disable', 'stale'],
  ] as const)('%s / %s 保留真实能力缓存更新，不误报成功或关闭验证框', async (action, status) => {
    mocks.configured = action === 'disable';
    mocks.update.mockResolvedValue(
      status === 'stale' ? { status } : { status, isCurrent: () => true },
    );
    render(<BiometricSection accountId="acc-a" />);
    fireEvent.click(await screen.findByRole('checkbox'));
    fireEvent.click(screen.getByRole('button', { name: 'Verify test credential' }));
    await waitFor(() => expect(mocks.verified).toHaveBeenCalledWith(false));
    expect(mocks.update).toHaveBeenCalledWith('acc-a', 'biometricEnabled', action === 'enable');
    expect(mocks.invalidate).toHaveBeenCalledWith('acc-a');
    expect(mocks.invalidate.mock.invocationCallOrder[0]).toBeLessThan(
      mocks.update.mock.invocationCallOrder[0],
    );
    if (action === 'disable') expect(mocks.clearMethod).toHaveBeenCalledWith('acc-a', 'touchId');
    expect(mocks.success).not.toHaveBeenCalled();
    expect(screen.getByRole('dialog')).toBeVisible();
    if (status === 'failed') expect(screen.getByText('common:save_failed')).toBeVisible();
  });

  it('当前设置写入已保存时保留成功反馈与关闭行为', async () => {
    mocks.update.mockResolvedValue({ status: 'saved', isCurrent: () => true });
    render(<BiometricSection accountId="acc-a" />);
    fireEvent.click(await screen.findByRole('checkbox'));
    fireEvent.click(screen.getByRole('button', { name: 'Verify test credential' }));
    await waitFor(() => expect(mocks.verified).toHaveBeenCalledWith(true));
    expect(mocks.success).toHaveBeenCalledTimes(1);
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });
});
