import { useState, useEffect, useCallback } from 'react';
import type { MutableRefObject } from 'react';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { logger } from '@/lib/logger';
import type { ActiveProvider } from '@/types/llmChat';

export interface UseLlmOnlineStatusOptions {
  activeProvider: ActiveProvider | null;
  accountId?: string;
  /** 共享的请求中止控制器（与父 hook 的会话列表加载共用）。 */
  abortRef: MutableRefObject<AbortController | null>;
}

/** LLM provider 在线状态检查（手动触发 + 60s 定时轮询）。 */
export function useLlmOnlineStatus({ activeProvider, accountId, abortRef }: UseLlmOnlineStatusOptions) {
  const [isOnline, setIsOnline] = useState<boolean | null>(null);
  const [checkingOnline, setCheckingOnline] = useState(false);

  const checkOnline = useCallback(() => {
    if (!activeProvider || !accountId) return;
    abortRef.current?.abort();
    const controller = new AbortController();
    abortRef.current = controller;
    setCheckingOnline(true);
    (async () => {
      try {
        let key = '';
        try {
          key = await invoke<string>('llm_get_api_key', {
            accountId: accountId,
            providerId: activeProvider.id,
          });
        } catch {
          /* may not have key */
        }
        const online = await invoke<boolean>('llm_check_connection', {
          baseUrl: activeProvider.baseUrl,
          apiKey: key,
          model: activeProvider.model,
          apiType: activeProvider.apiType,
        });
        if (!controller.signal.aborted) setIsOnline(online);
      } catch (err) {
        // P227: 在线检查失败视为离线（可接受降级），留痕。
        logger.warn('[useLlmChatCore] Online check failed:', err);
        if (!controller.signal.aborted) setIsOnline(false);
      } finally {
        if (!controller.signal.aborted) setCheckingOnline(false);
      }
    })();
  }, [activeProvider, accountId, abortRef]);

  useEffect(() => {
    if (activeProvider && accountId) checkOnline();
  }, [activeProvider, accountId, checkOnline]);

  useEffect(() => {
    if (!activeProvider) return;
    const interval = setInterval(checkOnline, 60000);
    return () => clearInterval(interval);
  }, [activeProvider, checkOnline]);

  return { isOnline, checkingOnline, checkOnline };
}
