import type { TFunction } from 'i18next';
import { invokeTypedCommand } from '@/lib/typedIpc';
import { logger } from '@/lib/logger';
import { useUiStore } from '@/stores/uiStore';
import type { Conversation } from '@/types/llmChat';

/** 会话保存失败提示（P007：不得静默——提示用户记录可能丢失）。 */
export function notifyConversationSaveFailed(t: TFunction) {
  useUiStore.getState().showToast({
    type: 'error',
    message: t('settings:ai_save_conversation_failed', {
      defaultValue: '对话保存失败，记录可能丢失，请重试',
    }),
    duration: 5000,
  });
}

/**
 * 保存会话；失败时留痕并 toast 提示。返回是否成功。
 * 供明确的发送前历史保存使用；最终 assistant 回复仅由后端写入。
 */
export async function saveConversationSafely(
  accountId: string | undefined,
  conversation: Conversation,
  t: TFunction,
  request?: { invokeTyped: typeof invokeTypedCommand; isCurrent: () => boolean },
): Promise<boolean> {
  if (!accountId) return false;
  try {
    await (request?.invokeTyped ?? invokeTypedCommand)('llm_save_conversation', {
      accountId,
      conversation,
    });
    return true;
  } catch (err) {
    if (request && !request.isCurrent()) return false;
    logger.warn('[useLlmChatCore] Save conversation failed:', err);
    notifyConversationSaveFailed(t);
    return false;
  }
}
