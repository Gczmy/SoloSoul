import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

// 动态 import 的 authStore 守卫需要 mock（默认未登录）
vi.mock('@/stores/authStore', () => ({
  useAuthStore: {
    getState: () => ({ isAuthenticated: false }),
  },
}));

import { invoke } from '@tauri-apps/api/core';
import { invokeCommand } from './ipcClient';

describe('invokeCommand（统一 IPC 调用层）', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    vi.mocked(invoke).mockResolvedValue(undefined);
  });

  it('args 缺省时以单参调用原生 invoke（兼容既有 toHaveBeenCalledWith 断言）', async () => {
    await invokeCommand<void>('lock');
    expect(invoke).toHaveBeenCalledTimes(1);
    expect(invoke).toHaveBeenCalledWith('lock');
  });

  it('args 提供时透传第二参', async () => {
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

  it('requireUnlocked 守卫：未解锁时直接抛 No account is currently unlocked', async () => {
    await expect(invokeCommand<void>('object_list', {}, { requireUnlocked: true })).rejects.toThrow(
      'No account is currently unlocked',
    );
    expect(invoke).not.toHaveBeenCalled();
  });
});
