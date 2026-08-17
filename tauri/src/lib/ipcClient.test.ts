import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

// 动态 import 的 authStore 守卫需要 mock（默认未登录，测试内可按需覆盖）
vi.mock('@/stores/authStore', () => ({
  useAuthStore: {
    getState: vi.fn(() => ({ isAuthenticated: false })),
  },
}));

import { useAuthStore } from '@/stores/authStore';

import { invoke } from '@tauri-apps/api/core';
import { invokeCommand } from './ipcClient';

describe('invokeCommand（统一 IPC 调用层）', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    vi.mocked(invoke).mockResolvedValue(undefined);
    // P027: 默认未解锁（测试间复位，防止上一个用例的已解锁 mock 泄漏）
    vi.mocked(useAuthStore).getState.mockReset();
    vi.mocked(useAuthStore).getState.mockReturnValue({ isAuthenticated: false } as never);
  });

  it('args 缺省时以单参调用原生 invoke（兼容既有 toHaveBeenCalledWith 断言）', async () => {
    await invokeCommand<void>('lock');
    expect(invoke).toHaveBeenCalledTimes(1);
    expect(invoke).toHaveBeenCalledWith('lock');
  });

  it('args 提供时透传第二参（测试环境守卫放行）', async () => {
    await invokeCommand<{ ok: boolean }>('object_get', { accountId: 'a1', objectId: 'o1' });
    expect(invoke).toHaveBeenCalledWith('object_get', { accountId: 'a1', objectId: 'o1' });
  });

  it('返回类型透传原生 invoke 结果', async () => {
    vi.mocked(invoke).mockResolvedValue({ ok: true });
    const res = await invokeCommand<{ ok: boolean }>('check_has_account');
    expect(res).toEqual({ ok: true });
  });

  it('错误原样抛出（消息不翻译，翻译留在展示层）', async () => {
    vi.mocked(invoke).mockRejectedValue(new Error('Invalid password'));
    await expect(invokeCommand<void>('login')).rejects.toThrow('Invalid password');
  });

  // ── P027 守卫（MODE=development 模拟生产环境，测试环境默认放行）──

  it('P027 默认守卫：非豁免敏感命令未解锁时直接抛 No account is currently unlocked', async () => {
    vi.stubEnv('MODE', 'development');
    try {
      await expect(invokeCommand<void>('object_list', {})).rejects.toThrow(
        'No account is currently unlocked',
      );
      expect(invoke).not.toHaveBeenCalled();
    } finally {
      vi.unstubAllEnvs();
    }
  });

  it('P027 默认守卫：已解锁时放行非豁免命令', async () => {
    vi.stubEnv('MODE', 'development');
    vi.mocked(useAuthStore).getState.mockReturnValue({ isAuthenticated: true } as never);
    try {
      await expect(invokeCommand<void>('object_list', {})).resolves.toBeUndefined();
      expect(invoke).toHaveBeenCalledWith('object_list', {});
    } finally {
      vi.unstubAllEnvs();
    }
  });

  it('P027 默认守卫：豁免名单命令未解锁时可调（认证/启动期命令）', async () => {
    vi.stubEnv('MODE', 'development');
    vi.mocked(useAuthStore).getState.mockReturnValue({ isAuthenticated: false } as never);
    try {
      await expect(invokeCommand<void>('vault_list_accounts')).resolves.toBeUndefined();
      await expect(invokeCommand<void>('get_system_locale')).resolves.toBeUndefined();
      await expect(invokeCommand<void>('ui_get_preferences')).resolves.toBeUndefined();
      expect(invoke).toHaveBeenCalledTimes(3);
    } finally {
      vi.unstubAllEnvs();
    }
  });

  it('P027 默认守卫：登录页可用性探测命令（biometric/pin check availability）未解锁时可调', async () => {
    vi.stubEnv('MODE', 'development');
    vi.mocked(useAuthStore).getState.mockReturnValue({ isAuthenticated: false } as never);
    try {
      await expect(
        invokeCommand<{ configured: boolean }>('biometric_check_availability', {
          accountId: 'acc_1',
        }),
      ).resolves.toBeUndefined();
      await expect(
        invokeCommand<{ configured: boolean }>('pin_check_availability', { accountId: 'acc_1' }),
      ).resolves.toBeUndefined();
      expect(invoke).toHaveBeenCalledTimes(2);
      expect(invoke).toHaveBeenNthCalledWith(1, 'biometric_check_availability', {
        accountId: 'acc_1',
      });
      expect(invoke).toHaveBeenNthCalledWith(2, 'pin_check_availability', { accountId: 'acc_1' });
    } finally {
      vi.unstubAllEnvs();
    }
  });

  it('P027 默认守卫：requireUnlocked:false 显式豁免时未解锁可调', async () => {
    vi.stubEnv('MODE', 'development');
    vi.mocked(useAuthStore).getState.mockReturnValue({ isAuthenticated: false } as never);
    try {
      await expect(
        invokeCommand<void>('object_list', {}, { requireUnlocked: false }),
      ).resolves.toBeUndefined();
      expect(invoke).toHaveBeenCalledWith('object_list', {});
    } finally {
      vi.unstubAllEnvs();
    }
  });

  it('P027 默认守卫：requireUnlocked:true 强制拦截豁免名单命令', async () => {
    vi.stubEnv('MODE', 'development');
    vi.mocked(useAuthStore).getState.mockReturnValue({ isAuthenticated: false } as never);
    try {
      await expect(
        invokeCommand<void>('vault_list_accounts', {}, { requireUnlocked: true }),
      ).rejects.toThrow('No account is currently unlocked');
      expect(invoke).not.toHaveBeenCalled();
    } finally {
      vi.unstubAllEnvs();
    }
  });
});
