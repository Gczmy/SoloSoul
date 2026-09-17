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

  it('P027 默认守卫：首次引导存储目录命令（vault_pick_directory/init_vault_directory）未解锁时可调', async () => {
    vi.stubEnv('MODE', 'development');
    vi.mocked(useAuthStore).getState.mockReturnValue({ isAuthenticated: false } as never);
    try {
      await expect(invokeCommand<void>('vault_pick_directory')).resolves.toBeUndefined();
      await expect(
        invokeCommand<void>('init_vault_directory', { payload: { safTreeUri: null } }),
      ).resolves.toBeUndefined();
      expect(invoke).toHaveBeenCalledTimes(2);
      expect(invoke).toHaveBeenNthCalledWith(2, 'init_vault_directory', {
        payload: { safTreeUri: null },
      });
    } finally {
      vi.unstubAllEnvs();
    }
  });

  it('P027 默认守卫：登录页恢复数据与主题应用命令（recovery_*/get_system_theme）未解锁时可调', async () => {
    vi.stubEnv('MODE', 'development');
    vi.mocked(useAuthStore).getState.mockReturnValue({ isAuthenticated: false } as never);
    try {
      await expect(invokeCommand<void>('recovery_discover_hosts', {})).resolves.toBeUndefined();
      await expect(invokeCommand<void>('recovery_restore_from_host', {})).resolves.toBeUndefined();
      await expect(invokeCommand<void>('get_system_theme')).resolves.toBeUndefined();
      expect(invoke).toHaveBeenCalledTimes(3);
      expect(invoke).toHaveBeenNthCalledWith(1, 'recovery_discover_hosts', {});
    } finally {
      vi.unstubAllEnvs();
    }
  });

  it('P027 默认守卫：自更新管线命令（android_check_update 等）未解锁时可调', async () => {
    vi.stubEnv('MODE', 'development');
    vi.mocked(useAuthStore).getState.mockReturnValue({ isAuthenticated: false } as never);
    try {
      await expect(invokeCommand<void>('android_check_update')).resolves.toBeUndefined();
      await expect(
        invokeCommand<void>('android_download_apk', { version: '2.11.1' }),
      ).resolves.toBeUndefined();
      await expect(
        invokeCommand<void>('android_get_apk_path', { version: '2.11.1' }),
      ).resolves.toBeUndefined();
      await expect(
        invokeCommand<void>('android_is_apk_downloaded', { version: '2.11.1' }),
      ).resolves.toBeUndefined();
      await expect(invokeCommand<void>('desktop_check_update')).resolves.toBeUndefined();
      expect(invoke).toHaveBeenCalledTimes(5);
      expect(invoke).toHaveBeenNthCalledWith(2, 'android_download_apk', { version: '2.11.1' });
    } finally {
      vi.unstubAllEnvs();
    }
  });

  it.each([
    ['create_update_download', undefined],
    ['cancel_update_download', { operationId: 41 }],
    ['desktop_download_update', { updateRid: 17, operationId: 41, onEvent: 9 }],
    ['desktop_install_update', { downloadRid: 91 }],
  ] as const)('登录前允许新下载管线命令 %s，不受 Vault 解锁守卫阻断', async (command, args) => {
    vi.stubEnv('MODE', 'development');
    try {
      await expect(invokeCommand(command, args)).resolves.toBeUndefined();
      if (args) {
        expect(invoke).toHaveBeenCalledExactlyOnceWith(command, args);
      } else {
        expect(invoke).toHaveBeenCalledExactlyOnceWith(command);
      }
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

  it('P039：异步鉴权后会话已变更时不发送 IPC', async () => {
    vi.stubEnv('MODE', 'development');
    let current = true;
    vi.mocked(useAuthStore).getState.mockImplementation(() => {
      current = false;
      return { isAuthenticated: true } as never;
    });
    try {
      await expect(
        invokeCommand('object_list', {}, { requestIsCurrent: () => current }),
      ).rejects.toThrow('expired session');
      expect(invoke).not.toHaveBeenCalled();
    } finally {
      vi.unstubAllEnvs();
    }
  });

  it('P039：无需解锁的偏好写入也检查调用方会话', async () => {
    await expect(
      invokeCommand('ui_update_preference', {}, { requestIsCurrent: () => false }),
    ).rejects.toThrow('expired session');
    expect(invoke).not.toHaveBeenCalled();
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
