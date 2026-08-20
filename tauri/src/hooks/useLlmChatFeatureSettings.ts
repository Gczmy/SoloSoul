/**
 * LlmConfigPage AI 功能开关域：chat 开关（含风险确认）、系统提示词开关。
 */
import { useState } from 'react';
import type { TFunction } from 'i18next';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { prefetchRegistry } from '@/lib/prefetch/registry';
import { logger } from '@/lib/logger';

/** AI 功能中始终处于禁用状态的功能（UI 尚未提供开关） */
const ALWAYS_DISABLED_FEATURES = {
  smartFill: false,
  commandGen: false,
  naturalLanguageSearch: false,
} as const;

export interface UseLlmChatFeatureSettingsOptions {
  accountId?: string;
  t: TFunction;
  onError: (err: unknown, context: string) => void;
}

export function useLlmChatFeatureSettings({
  accountId,
  t,
  onError,
}: UseLlmChatFeatureSettingsOptions) {
  const [chatEnabled, setChatEnabled] = useState(false);
  const [includeSystemPrompt, setIncludeSystemPrompt] = useState(true);
  const [hasAcceptedRisk, setHasAcceptedRisk] = useState(false);
  const [showRiskDialog, setShowRiskDialog] = useState(false);

  const handleFeatureToggle = async () => {
    const next = !chatEnabled;
    if (!hasAcceptedRisk && next) {
      setShowRiskDialog(true);
      return;
    }
    setChatEnabled(next);
    if (accountId) {
      try {
        await invoke('llm_set_ai_features', {
          accountId: accountId,
          features: { chat: next, ...ALWAYS_DISABLED_FEATURES },
        });
        // Prefetch Runtime: AI 功能开关变更 → 刷新 LLM 配置缓存
        void prefetchRegistry.llmConfig.invalidate();
      } catch (err) {
        // P028-R1: 失败回滚需防竞态——仅当当前状态仍是本次操作写入的 next 时才回滚
        //（函数式比对），连续快速切换时旧失败不得覆盖后一次成功操作。
        logger.warn('[LLMConfig] Set AI features failed:', err);
        setChatEnabled((cur) => (cur === next ? !next : cur));
        onError(err, t('settings:llm_set_features_failed'));
      }
    }
  };

  const handleAcceptRisk = async () => {
    if (!accountId) return;
    try {
      await invoke('llm_accept_risk', { accountId: accountId });
    } catch (err) {
      // P028-R1: accept_risk 失败不得置 hasAcceptedRisk/chatEnabled——
      // 否则后端未接受风险、UI 却放行 AI 功能（功能开关与后端真实状态不符）。
      logger.warn('[LLMConfig] Accept risk failed:', err);
      onError(err, t('settings:llm_accept_risk_failed'));
      return;
    }
    setHasAcceptedRisk(true);
    setShowRiskDialog(false);
    setChatEnabled(true);
    try {
      await invoke('llm_set_ai_features', {
        accountId: accountId,
        features: { chat: true, ...ALWAYS_DISABLED_FEATURES },
      });
      // Prefetch Runtime: AI 功能启用 → 刷新 LLM 配置缓存
      void prefetchRegistry.llmConfig.invalidate();
    } catch (err) {
      // P028-R1: 启用功能失败需回滚 chatEnabled（函数式比对防竞态）
      logger.warn('[LLMConfig] Set features after risk accept failed:', err);
      setChatEnabled((cur) => (cur === true ? false : cur));
      onError(err, t('settings:llm_set_features_failed'));
    }
  };

  const handleSystemPromptToggle = async () => {
    const next = !includeSystemPrompt;
    setIncludeSystemPrompt(next);
    if (accountId) {
      try {
        await invoke('llm_set_system_prompt_switch', { accountId: accountId, enabled: next });
      } catch (err) {
        // P028-R1: 失败回滚需防竞态——仅当当前状态仍是本次操作写入的 next 时才回滚
        logger.warn('[LLMConfig] Set system prompt switch failed:', err);
        setIncludeSystemPrompt((cur) => (cur === next ? !next : cur));
        onError(err, t('settings:llm_set_prompt_failed'));
      }
    }
  };

  return {
    chatEnabled,
    includeSystemPrompt,
    hasAcceptedRisk,
    showRiskDialog,
    setChatEnabled,
    setIncludeSystemPrompt,
    setHasAcceptedRisk,
    setShowRiskDialog,
    handleFeatureToggle,
    handleAcceptRisk,
    handleSystemPromptToggle,
  };
}
