import type { TFunction } from 'i18next';
import { invokeCommand as invoke } from '@/lib/ipcClient';
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
 * 供首次保存 / 流结束保存 / 错误会话保存三处共用（P007 统一处理路径）。
 */
export async function saveConversationSafely(
  accountId: string | undefined,
  conversation: Conversation,
  t: TFunction,
): Promise<boolean> {
  if (!accountId) return false;
  try {
    await invoke('llm_save_conversation', { accountId, conversation });
    return true;
  } catch (err) {
    logger.warn('[useLlmChatCore] Save conversation failed:', err);
    notifyConversationSaveFailed(t);
    return false;
  }
}
