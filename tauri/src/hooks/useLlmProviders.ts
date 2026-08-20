/**
 * LlmConfigPage provider 管理域：列表、激活（含云端隐私确认）、保存、删除、连通性测试。
 */
import { useState } from 'react';
import type { TFunction } from 'i18next';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { prefetchRegistry } from '@/lib/prefetch/registry';
import { MASK_PLACEHOLDER } from '@/lib/masking';
import type { ProviderConfig } from '@/types/llmProvider';
import { isOllama } from '@/types/llmChat';
import { logger } from '@/lib/logger';

/**
 * P035: 云端 LLM 隐私确认标志（设备级 localStorage）——首次启用非本地 provider 前弹确认，
 * 用户同意后写入该标志，后续同一设备切换云端 provider 不再重复拦截。
 */
const LLM_CLOUD_PRIVACY_ACK = 'llm_cloud_privacy_accepted';

export interface UseLlmProvidersOptions {
  accountId?: string;
  t: TFunction;
  onError: (err: unknown, context: string) => void;
  onSuccess: (message: string) => void;
  requestConfirm: (
    title: string,
    message: string,
    onConfirm: () => void,
    options?: { confirmLabel?: string; cancelLabel?: string },
  ) => void;
}

export function useLlmProviders({
  accountId,
  t,
  onError,
  onSuccess,
  requestConfirm,
}: UseLlmProvidersOptions) {
  const [providers, setProviders] = useState<ProviderConfig[]>([]);
  const [activeId, setActiveId] = useState<string>('');

  const applyActiveProvider = async (id: string) => {
    if (!accountId) return;
    const prevActive = activeId;
    setActiveId(id);
    try {
      await invoke('llm_set_active_provider', { accountId: accountId, providerId: id });
      // Prefetch Runtime: 激活 provider 变更 → 刷新 LLM 配置缓存（AI 对话弹层/聊天页即时生效）
      void prefetchRegistry.llmConfig.invalidate();
    } catch (err) {
      // P028-R1: 失败回滚需防竞态——仅当当前状态仍是本次操作写入的 id 时才回滚
      //（函数式比对），否则可能是后续更快的成功切换，误回滚会覆盖新状态。
      logger.warn('[LLMConfig] Set active provider failed:', err);
      setActiveId((cur) => (cur === id ? prevActive : cur));
      onError(err, t('settings:llm_set_active_failed'));
    }
  };

  const handleSetActive = (id: string) => {
    if (!accountId) return;
    const target = providers.find((p) => p.id === id);
    // P035: 首次启用云端（非 localhost/127.0.0.1）provider 前弹隐私确认；
    // 同意后写设备级标志，后续切换云端 provider 不再拦截（本地 LLM 服务器不拦截）。
    if (target && !isOllama(target.baseUrl) && !localStorage.getItem(LLM_CLOUD_PRIVACY_ACK)) {
      requestConfirm(
        t('settings:llm_cloud_privacy_confirm_title'),
        t('settings:llm_cloud_privacy_confirm_body', { name: target.name }),
        () => {
          localStorage.setItem(LLM_CLOUD_PRIVACY_ACK, '1');
          void applyActiveProvider(id);
        },
        { confirmLabel: t('common:confirm'), cancelLabel: t('common:cancel') },
      );
      return;
    }
    void applyActiveProvider(id);
  };

  const handleSaveProvider = async (provider: ProviderConfig) => {
    if (!accountId) return;
    try {
      await invoke('llm_save_provider', { accountId: accountId, provider });
    } catch (err) {
      // N-4: 新外部 URL 登记需原生确认对话框；用户取消或校验失败时
      // 后端返回 Err——本地列表不得误报「保存成功」。取消属预期路径，
      // 其余失败给出 toast 提示（避免点击保存后无任何反馈）。
      logger.warn('[LLMConfig] Save provider failed:', err);
      const msg = err instanceof Error ? err.message : String(err);
      if (!msg.includes('已取消')) {
        onError(err, t('settings:llm_save_provider_failed'));
      }
      return;
    }
    setProviders((prev) => {
      const idx = prev.findIndex((p) => p.id === provider.id);
      const updated = { ...provider, apiKey: provider.apiKey ? MASK_PLACEHOLDER : '' };
      if (idx >= 0) {
        const n = [...prev];
        n[idx] = updated;
        return n;
      }
      return [...prev, updated];
    });
    // Prefetch Runtime: provider 变更 → 刷新 LLM 配置缓存（AI 对话弹层/聊天页即时生效）
    void prefetchRegistry.llmConfig.invalidate();
    onSuccess(t('common:success'));
  };

  const handleDeleteProvider = (id: string) => {
    if (!accountId) return;
    requestConfirm(
      t('settings:llm_delete_provider_title', { defaultValue: 'Delete provider' }),
      t('settings:llm_delete_provider_body', {
        defaultValue: 'Delete this provider configuration?',
      }),
      async () => {
        try {
          await invoke('llm_delete_provider', { accountId: accountId, providerId: id });
        } catch (err) {
          // P007: 删除失败不得更新本地状态——否则后端未删、UI 已移除，重启后
          // 复现且误清 activeId 导致功能开关与实际不符；失败给出 toast 反馈。
          logger.warn('[LLMConfig] Delete provider failed:', err);
          onError(err, t('settings:llm_delete_provider_failed'));
          return;
        }
        setProviders((prev) => prev.filter((p) => p.id !== id));
        if (activeId === id) setActiveId('');
        // Prefetch Runtime: provider 删除 → 刷新 LLM 配置缓存
        void prefetchRegistry.llmConfig.invalidate();
      },
      {
        confirmLabel: t('common:delete', { defaultValue: 'Delete' }),
        cancelLabel: t('common:cancel', { defaultValue: 'Cancel' }),
      },
    );
  };

  const handleTestConnection = async (provider: ProviderConfig, accId: string): Promise<string> => {
    let key = provider.apiKey;
    if (key === MASK_PLACEHOLDER) {
      key = await invoke<string>('llm_get_api_key', {
        accountId: accId,
        providerId: provider.id,
      });
    }
    const result = await invoke<string>('llm_test_provider', {
      baseUrl: provider.baseUrl,
      apiKey: key,
      model: provider.model,
      apiType: provider.apiType,
    });
    return t('settings:llm_test_ok') + ' "' + result.slice(0, 80) + '"';
  };

  return {
    providers,
    activeId,
    setProviders,
    setActiveId,
    handleSetActive,
    handleSaveProvider,
    handleDeleteProvider,
    handleTestConnection,
  };
}
