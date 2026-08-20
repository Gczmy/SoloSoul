import { useState, useEffect } from 'react';
import { usePrefetchData } from '@/lib/prefetch/usePrefetchData';
import { prefetchRegistry } from '@/lib/prefetch/registry';
import { logger } from '@/lib/logger';
import type { ActiveProvider } from '@/types/llmChat';

export interface UseLlmProviderConfigOptions {
  accountId?: string;
}

/**
 * LLM provider + config 加载。Prefetch Runtime：AI 快捷对话弹层/聊天页共享缓存，
 * 登录后后台预热，打开弹层直接渲染（无 LoadingPlaceholder 占位期）；
 * 设置页保存/删除/切换 provider 后 invalidate 强制刷新。
 */
export function useLlmProviderConfig({ accountId }: UseLlmProviderConfigOptions) {
  const [activeProvider, setActiveProvider] = useState<ActiveProvider | null>(null);
  const [isConfigured, setIsConfigured] = useState(false);
  const [isAiEnabled, setIsAiEnabled] = useState(false);
  const [loading, setLoading] = useState(true);

  const { data: llmConfig, error: llmConfigError } = usePrefetchData(prefetchRegistry.llmConfig, {
    enabled: !!accountId,
  });

  useEffect(() => {
    if (!accountId) {
      setLoading(false);
      return;
    }
    // 数据未就绪（冷启动加载中）：loading 保持 true，store 兜底加载完成后触发本 effect。
    if (llmConfig === null && !llmConfigError) return;
    if (llmConfig) {
      setIsAiEnabled(llmConfig.aiFeaturesEnabled.chat ?? false);
      const active = llmConfig.providers.find((p) => p.id === llmConfig.activeProviderId);
      if (active) {
        setActiveProvider({
          id: active.id,
          name: active.name,
          model: active.model,
          baseUrl: active.baseUrl,
          apiType: active.apiType,
        });
        setIsConfigured(true);
      } else {
        setIsConfigured(false);
      }
    } else {
      // P227: 配置加载失败静默降级为未配置，留痕便于排查。
      logger.warn('[useLlmChatCore] Load config failed:', llmConfigError);
      setIsConfigured(false);
    }
    setLoading(false);
  }, [accountId, llmConfig, llmConfigError]);

  return { activeProvider, isConfigured, isAiEnabled, loading };
}
