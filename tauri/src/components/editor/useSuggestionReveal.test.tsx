import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';
import { useSuggestionReveal, suggestionItemId } from './useSuggestionReveal';
import type { FieldSuggestion } from './FieldSuggestions';

const invokeMock = vi.hoisted(() => vi.fn());

vi.mock('@/lib/ipcClient', () => ({
  invokeCommand: invokeMock,
}));

vi.mock('@/lib/logger', () => ({
  logger: { warn: vi.fn(), error: vi.fn(), info: vi.fn(), debug: vi.fn() },
}));

const baseItem: FieldSuggestion = {
  objectId: 'obj-1',
  objectName: '我的身份证',
  fieldKey: 'citizen_no',
  fieldName: '身份证号码',
  sensitivityLevel: 'critical',
  value: '110101199001011234',
};

function logWriteCall() {
  return invokeMock.mock.calls.find((c) => c[0] === 'log_write');
}

describe('useSuggestionReveal', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockImplementation((cmd: string) => {
      switch (cmd) {
        case 'biometric_check_availability':
          return Promise.resolve({ available: false, configured: false });
        case 'vault_list_accounts':
          return Promise.resolve([{ id: 'acc-1', passwordHint: 'hint' }]);
        case 'unlock_with_password':
          return Promise.resolve(undefined);
        case 'log_write':
          return Promise.resolve(undefined);
        default:
          return Promise.resolve(undefined);
      }
    });
  });

  it('sensitive 点击揭示、再次点击隐藏', async () => {
    const { result } = renderHook(() => useSuggestionReveal('acc-1'));
    // 等待探测副作用（biometric / password hint）结算，避免 act 警告
    await waitFor(() => expect(result.current.passwordHint).toBe('hint'));
    const item = { ...baseItem, sensitivityLevel: 'sensitive' };
    const id = suggestionItemId(item);

    act(() => result.current.handleItemClick(item));
    expect(result.current.isRevealed(id)).toBe(true);
    expect(result.current.showPwDialog).toBe(false);

    act(() => result.current.handleItemClick(item));
    expect(result.current.isRevealed(id)).toBe(false);
  });

  it('public / internal 点击无操作（不揭示、不弹验证框）', async () => {
    const { result } = renderHook(() => useSuggestionReveal('acc-1'));
    await waitFor(() => expect(result.current.passwordHint).toBe('hint'));
    for (const sensitivityLevel of ['public', 'internal'] as const) {
      const item = { ...baseItem, sensitivityLevel };
      const id = suggestionItemId(item);

      act(() => result.current.handleItemClick(item));
      expect(result.current.isRevealed(id)).toBe(false);
      expect(result.current.showPwDialog).toBe(false);
    }
  });

  it('critical 点击弹出验证框；密码错误不揭示，密码正确揭示并写登录日志', async () => {
    const { result } = renderHook(() => useSuggestionReveal('acc-1'));
    await waitFor(() => expect(result.current.passwordHint).toBe('hint'));
    const item = { ...baseItem, sensitivityLevel: 'critical' };
    const id = suggestionItemId(item);

    act(() => result.current.handleItemClick(item));
    expect(result.current.showPwDialog).toBe(true);

    // 密码错误：unlock_with_password 拒绝
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'unlock_with_password') return Promise.reject('Invalid password');
      return Promise.resolve(undefined);
    });
    await act(async () => {
      const ok = await result.current.handlePwDialogVerify('wrong');
      expect(ok).toBe(false);
    });
    expect(result.current.isRevealed(id)).toBe(false);

    // 密码正确
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'unlock_with_password') return Promise.resolve(undefined);
      return Promise.resolve(undefined);
    });
    await act(async () => {
      const ok = await result.current.handlePwDialogVerify('right');
      expect(ok).toBe(true);
    });
    expect(result.current.isRevealed(id)).toBe(true);

    const call = logWriteCall();
    expect(call).toBeTruthy();
    expect(call![1].request.actionType).toBe('critical_field_login');
    expect(call![1].request.entityType).toBe('auth');
    expect(call![1].request.entityId).toBe('obj-1');
    expect(call![1].request.entityName).toBe('我的身份证');
    expect(call![1].request.details).toContain('fieldName=身份证号码');
  });

  it('critical 关闭验证框不揭示', async () => {
    const { result } = renderHook(() => useSuggestionReveal('acc-1'));
    await waitFor(() => expect(result.current.passwordHint).toBe('hint'));
    const item = { ...baseItem, sensitivityLevel: 'critical' };
    const id = suggestionItemId(item);

    act(() => result.current.handleItemClick(item));
    expect(result.current.showPwDialog).toBe(true);
    act(() => result.current.handlePwDialogClose());
    expect(result.current.showPwDialog).toBe(false);
    expect(result.current.isRevealed(id)).toBe(false);
  });

  it('critical PIN 成功揭示并写 pin 日志', async () => {
    const { result } = renderHook(() => useSuggestionReveal('acc-1'));
    await waitFor(() => expect(result.current.passwordHint).toBe('hint'));
    const item = { ...baseItem, sensitivityLevel: 'critical' };
    const id = suggestionItemId(item);

    act(() => result.current.handleItemClick(item));
    await act(async () => result.current.handlePwDialogPinSuccess());
    expect(result.current.isRevealed(id)).toBe(true);
    expect(logWriteCall()![1].request.actionType).toBe('critical_field_pin');
  });

  it('critical 生物识别成功揭示并写对应日志', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      switch (cmd) {
        case 'biometric_check_availability':
          return Promise.resolve({
            available: true,
            configured: true,
            biometryType: 'faceId',
          });
        case 'biometric_unlock':
          return Promise.resolve(undefined);
        case 'vault_list_accounts':
          return Promise.resolve([{ id: 'acc-1' }]);
        default:
          return Promise.resolve(undefined);
      }
    });
    const { result } = renderHook(() => useSuggestionReveal('acc-1'));
    await waitFor(() => expect(result.current.bioAvailable.available).toBe(true));

    const item = { ...baseItem, sensitivityLevel: 'critical' };
    const id = suggestionItemId(item);
    act(() => result.current.handleItemClick(item));
    await act(async () => {
      const ok = await result.current.handleBiometricUnlock();
      expect(ok).toBe(true);
    });
    expect(result.current.isRevealed(id)).toBe(true);
    expect(logWriteCall()![1].request.actionType).toBe('critical_field_face_id');
  });

  it('无 accountId 时密码验证直接失败', async () => {
    const { result } = renderHook(() => useSuggestionReveal(undefined));
    await act(async () => {
      const ok = await result.current.handlePwDialogVerify('pw');
      expect(ok).toBe(false);
    });
    expect(invokeMock).not.toHaveBeenCalledWith('unlock_with_password');
  });
});
