import { act, cleanup, renderHook, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { TFunction } from 'i18next';
import { useLlmProviderConfig } from './useLlmProviderConfig';
import { useLlmChatFeatureSettings } from './useLlmChatFeatureSettings';
import { prefetchRegistry } from '@/lib/prefetch/registry';

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  onError: vi.fn(),
  savedIncludeSystemPrompt: true,
  failSave: false,
}));
vi.mock('@/lib/ipcClient', () => ({ invokeCommand: mocks.invoke }));
vi.mock('@/stores/authStore', () => ({
  useAuthStore: {
    getState: () => ({ currentAccount: { id: 'account' }, isAuthenticated: true }),
  },
}));
vi.mock('@/lib/logger', () => ({ logger: { warn: vi.fn() } }));

const t = ((key: string) => key) as TFunction;
const providers = [
  {
    id: 'provider',
    name: 'Provider',
    model: 'model',
    baseUrl: 'https://example.test',
    apiType: 'openAI',
  },
];
function config() {
  return {
    activeProviderId: 'provider',
    aiFeaturesEnabled: { chat: true },
    includeSystemPrompt: mocks.savedIncludeSystemPrompt,
  };
}
let pendingConfig: Promise<ReturnType<typeof config>> | null;

function useSettingsAndProvider() {
  return {
    settings: useLlmChatFeatureSettings({ accountId: 'account', t, onError: mocks.onError }),
    provider: useLlmProviderConfig({ accountId: 'account' }),
  };
}

beforeEach(() => {
  vi.clearAllMocks();
  mocks.savedIncludeSystemPrompt = true;
  mocks.failSave = false;
  pendingConfig = null;
  prefetchRegistry.llmConfig.reset();
  mocks.invoke.mockImplementation(async (command: string, args?: { enabled: boolean }) => {
    if (command === 'llm_get_config') return pendingConfig ?? config();
    if (command === 'llm_get_providers') return providers;
    if (command === 'llm_set_system_prompt_switch') {
      if (mocks.failSave) throw new Error('Save failed');
      mocks.savedIncludeSystemPrompt = args!.enabled;
    }
    return undefined;
  });
});
afterEach(() => {
  cleanup();
  prefetchRegistry.llmConfig.reset();
});

describe('RF-004 persisted context preference propagation', () => {
  it('refreshes the real prefetch store after saving and notifies an already mounted consumer', async () => {
    const { result } = renderHook(useSettingsAndProvider);
    await waitFor(() => expect(result.current.provider.loading).toBe(false));
    expect(result.current.provider.includeSystemPrompt).toBe(true);
    const configReads = () =>
      mocks.invoke.mock.calls.filter(([command]) => command === 'llm_get_config').length;
    const readsBeforeToggle = configReads();

    await act(async () => {
      await result.current.settings.handleSystemPromptToggle();
    });
    await waitFor(() => expect(result.current.provider.includeSystemPrompt).toBe(false));

    expect(mocks.invoke).toHaveBeenCalledWith('llm_set_system_prompt_switch', {
      accountId: 'account',
      enabled: false,
    });
    expect(result.current.settings.includeSystemPrompt).toBe(false);
    expect(prefetchRegistry.llmConfig.getSnapshot().data?.includeSystemPrompt).toBe(false);
    expect(configReads()).toBe(readsBeforeToggle + 1);
    expect(mocks.onError).not.toHaveBeenCalled();
  });

  it('retains the known disabled value while invalidation clears the cached snapshot', async () => {
    mocks.savedIncludeSystemPrompt = false;
    const { result } = renderHook(() => useLlmProviderConfig({ accountId: 'account' }));
    await waitFor(() => expect(result.current.includeSystemPrompt).toBe(false));
    let resolveConfig!: (value: ReturnType<typeof config>) => void;
    pendingConfig = new Promise((resolve) => {
      resolveConfig = resolve;
    });
    let refresh!: ReturnType<typeof prefetchRegistry.llmConfig.invalidate>;
    act(() => {
      refresh = prefetchRegistry.llmConfig.invalidate();
    });

    expect(prefetchRegistry.llmConfig.getSnapshot().data).toBeNull();
    expect(result.current.includeSystemPrompt).toBe(false);

    await act(async () => {
      resolveConfig(config());
      await refresh;
    });
    expect(result.current.includeSystemPrompt).toBe(false);
    expect(prefetchRegistry.llmConfig.getSnapshot().data?.includeSystemPrompt).toBe(false);
  });

  it('preserves the persisted and cached configuration when saving the toggle fails', async () => {
    const { result } = renderHook(useSettingsAndProvider);
    await waitFor(() => expect(result.current.provider.loading).toBe(false));
    const cached = prefetchRegistry.llmConfig.getSnapshot().data;
    const readsBeforeToggle = mocks.invoke.mock.calls.filter(
      ([command]) => command === 'llm_get_config',
    ).length;
    mocks.failSave = true;

    await act(async () => {
      await result.current.settings.handleSystemPromptToggle();
    });

    expect(result.current.settings.includeSystemPrompt).toBe(true);
    expect(result.current.provider.includeSystemPrompt).toBe(true);
    expect(mocks.savedIncludeSystemPrompt).toBe(true);
    expect(prefetchRegistry.llmConfig.getSnapshot().data).toBe(cached);
    expect(
      mocks.invoke.mock.calls.filter(([command]) => command === 'llm_get_config'),
    ).toHaveLength(readsBeforeToggle);
    expect(mocks.onError).toHaveBeenCalledOnce();
  });
});
