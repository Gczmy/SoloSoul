import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { LAST_ACCOUNT_KEY } from '@/stores/authStore';
import {
  preflightLoginAvailability,
  preflightForLastAccount,
  normalizeBiometryType,
  invalidateLoginAvailabilityPreflight,
  __resetLoginAvailabilityPreflightForTest,
} from './loginAvailabilityPreflight';

describe('loginAvailabilityPreflight（方案 C：启动期预探测）', () => {
  beforeEach(() => {
    __resetLoginAvailabilityPreflightForTest();
    vi.mocked(invoke).mockReset();
    localStorage.clear();
  });

  it('同一账户重复调用只发一次探测（模块缓存去重）', async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce({ available: true, configured: true, biometryType: 'touchId' })
      .mockResolvedValueOnce({ configured: true, locked: false });

    const [a, b] = await Promise.all([
      preflightLoginAvailability('acc-1'),
      preflightLoginAvailability('acc-1'),
    ]);

    expect(a).toEqual(b);
    expect(invoke).toHaveBeenCalledTimes(2); // biometric + pin 各一次
    expect(invoke).toHaveBeenCalledWith('biometric_check_availability', { accountId: 'acc-1' });
    expect(invoke).toHaveBeenCalledWith('pin_check_availability', { accountId: 'acc-1' });
  });

  it('换账户重新探测', async () => {
    vi.mocked(invoke).mockResolvedValue({ configured: false, locked: false });

    await preflightLoginAvailability('acc-1');
    await preflightLoginAvailability('acc-2');

    expect(invoke).toHaveBeenCalledTimes(4); // 两个账户 × 2 命令
  });

  it('invalidateLoginAvailabilityPreflight 失效同账户缓存，下次调用重新探测', async () => {
    vi.mocked(invoke).mockResolvedValue({ configured: false, locked: false });

    const first = await preflightLoginAvailability('acc-1');
    expect(invoke).toHaveBeenCalledTimes(2);

    // 登录方式修改后失效缓存：同账户再次调用必须重新发起探测（而非复用旧结果）
    invalidateLoginAvailabilityPreflight('acc-1');
    await preflightLoginAvailability('acc-1');
    expect(invoke).toHaveBeenCalledTimes(4);
    expect(invoke).toHaveBeenCalledWith('biometric_check_availability', { accountId: 'acc-1' });
    expect(first).toEqual(await preflightLoginAvailability('acc-1'));
  });

  it('invalidateLoginAvailabilityPreflight 不影响其他账户的缓存', async () => {
    vi.mocked(invoke).mockResolvedValue({ configured: false, locked: false });

    await preflightLoginAvailability('acc-1');
    await preflightLoginAvailability('acc-2');
    expect(invoke).toHaveBeenCalledTimes(4);

    // 修改 acc-1 的登录方式：acc-2 的缓存仍有效，不会重新探测
    invalidateLoginAvailabilityPreflight('acc-1');
    await preflightLoginAvailability('acc-2');
    expect(invoke).toHaveBeenCalledTimes(4);
  });

  it('正确映射可用性（configured + available → bioAvailable）', async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce({ available: true, configured: true, biometryType: 'faceId' })
      .mockResolvedValueOnce({ configured: true, locked: false });

    const r = await preflightLoginAvailability('acc-1');
    expect(r.bioAvailable).toBe(true);
    expect(r.bioLockout).toBe(false);
    expect(r.biometryTypeRaw).toBe('faceId');
    expect(r.pinAvailable).toBe(true);
  });

  it('lockout 时保留指纹项并标记锁定（available=false 也显示）', async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce({ available: false, configured: true, lockout: true })
      .mockResolvedValueOnce({ configured: false, locked: false });

    const r = await preflightLoginAvailability('acc-1');
    expect(r.bioAvailable).toBe(true);
    expect(r.bioLockout).toBe(true);
    expect(r.pinAvailable).toBe(false);
  });

  it('未配置凭证 → 不可用', async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce({ available: true, configured: false })
      .mockResolvedValueOnce({ configured: false, locked: true });

    const r = await preflightLoginAvailability('acc-1');
    expect(r.bioAvailable).toBe(false);
    expect(r.pinAvailable).toBe(false);
  });

  it('探测失败清除缓存，下次调用重新发起', async () => {
    // 首次：biometric 失败（Promise.all 立即拒绝）；二次：两个命令均正常返回
    vi.mocked(invoke)
      .mockRejectedValueOnce(new Error('boom'))
      .mockResolvedValueOnce({ configured: false, locked: false })
      .mockResolvedValueOnce({ available: false, configured: false })
      .mockResolvedValueOnce({ configured: false, locked: false });

    await expect(preflightLoginAvailability('acc-1')).rejects.toThrow('boom');
    const r = await preflightLoginAvailability('acc-1');
    expect(r.bioAvailable).toBe(false);
    expect(r.pinAvailable).toBe(false);
    // 失败后缓存被清：第二次调用重新发起 2 个命令（共 4 次）
    expect(invoke).toHaveBeenCalledTimes(4);
    expect(invoke).toHaveBeenCalledWith('biometric_check_availability', { accountId: 'acc-1' });
  });

  it('preflightForLastAccount 从 LAST_ACCOUNT_KEY 读取并预探测；无记录时跳过', async () => {
    vi.mocked(invoke).mockResolvedValue({ configured: false, locked: false });

    localStorage.setItem(LAST_ACCOUNT_KEY, 'acc-9');
    preflightForLastAccount();
    await preflightLoginAvailability('acc-9'); // 复用同账户缓存，不重复探测
    expect(invoke).toHaveBeenCalledTimes(2);
    expect(invoke).toHaveBeenCalledWith('biometric_check_availability', { accountId: 'acc-9' });

    vi.mocked(invoke).mockReset();
    localStorage.clear();
    preflightForLastAccount();
    expect(invoke).not.toHaveBeenCalled();
  });

  it('normalizeBiometryType 白名单归一化（未知/缺失回退 touchId）', () => {
    expect(normalizeBiometryType('faceId')).toBe('faceId');
    expect(normalizeBiometryType('windowsHello')).toBe('windowsHello');
    expect(normalizeBiometryType('weird')).toBe('touchId');
    expect(normalizeBiometryType(undefined)).toBe('touchId');
  });
});
