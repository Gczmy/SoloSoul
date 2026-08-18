import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

// registry 顶层静态 import authStore（loader 读当前账户）；mock getState 按用例覆盖。
vi.mock('@/stores/authStore', () => ({
  useAuthStore: {
    getState: vi.fn(() => ({ currentAccount: { id: 'acc-1' } })),
  },
}));

import { invoke } from '@tauri-apps/api/core';
import { useAuthStore } from '@/stores/authStore';
import { prefetchRegistry } from './registry';

describe('prefetchRegistry.exportScope（导入导出页导出范围树）', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    vi.mocked(invoke).mockResolvedValue([]);
    vi.mocked(useAuthStore).getState.mockReset();
    vi.mocked(useAuthStore).getState.mockReturnValue({ currentAccount: { id: 'acc-1' } } as never);
    prefetchRegistry.exportScope.reset();
  });

  it('loader 用当前账户调用 export_get_scope_tree（锁定命令名与参数）', async () => {
    const groups = [{ sectionType: 'identity', objects: [] }];
    vi.mocked(invoke).mockResolvedValue(groups);

    const data = await prefetchRegistry.exportScope.load({ force: true });

    expect(invoke).toHaveBeenCalledWith('export_get_scope_tree', { accountId: 'acc-1' });
    expect(data).toEqual(groups);
  });

  it('无当前账户时 loader 抛错，data 保持 null（不缓存假数据）', async () => {
    vi.mocked(useAuthStore).getState.mockReturnValue({ currentAccount: null } as never);

    const data = await prefetchRegistry.exportScope.load({ force: true });

    expect(data).toBeNull();
    expect(invoke).not.toHaveBeenCalled();
    expect(prefetchRegistry.exportScope.getSnapshot().error).not.toBeNull();
  });

  it('预热策略为 afterAuth（登录后后台填充，进入页面直接渲染）', () => {
    expect(prefetchRegistry.exportScope.options.warmupPolicy).toBe('afterAuth');
  });
});

describe('prefetchRegistry.llmConfig（AI 对话弹层/聊天页 provider 配置）', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    vi.mocked(invoke).mockResolvedValue(undefined);
    vi.mocked(useAuthStore).getState.mockReset();
    vi.mocked(useAuthStore).getState.mockReturnValue({ currentAccount: { id: 'acc-1' } } as never);
    prefetchRegistry.llmConfig.reset();
  });

  it('loader 合并 llm_get_config + llm_get_providers（锁定命令名与 accountId 参数）', async () => {
    const providers = [
      {
        id: 'openai',
        name: 'OpenAI',
        model: 'gpt-4o',
        baseUrl: 'https://api.openai.com/v1',
        apiType: 'openAI',
      },
    ];
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'llm_get_config') {
        return { activeProviderId: 'openai', aiFeaturesEnabled: { chat: true } };
      }
      if (cmd === 'llm_get_providers') return providers;
      return undefined;
    });

    const data = await prefetchRegistry.llmConfig.load({ force: true });

    expect(invoke).toHaveBeenCalledWith('llm_get_config', { accountId: 'acc-1' });
    expect(invoke).toHaveBeenCalledWith('llm_get_providers', { accountId: 'acc-1' });
    expect(data).toEqual({
      activeProviderId: 'openai',
      aiFeaturesEnabled: { chat: true },
      providers,
    });
  });

  it('无当前账户时 loader 抛错，data 保持 null（不缓存假数据）', async () => {
    vi.mocked(useAuthStore).getState.mockReturnValue({ currentAccount: null } as never);

    const data = await prefetchRegistry.llmConfig.load({ force: true });

    expect(data).toBeNull();
    expect(invoke).not.toHaveBeenCalled();
    expect(prefetchRegistry.llmConfig.getSnapshot().error).not.toBeNull();
  });
});
