import { useEffect, useRef } from 'react';
import type { TFunction } from 'i18next';
import { useLlmStore } from '@/stores/llmStore';
import { saveConversationSafely, notifyConversationSaveFailed } from '@/lib/llm/conversationPersistence';
import { nowISO } from '@/types/llmChat';
import type { ChatMsg, Conversation } from '@/types/llmChat';

export interface UseLlmStreamingOptions {
  messages: ChatMsg[];
  setMessages: React.Dispatch<React.SetStateAction<ChatMsg[]>>;
  accountId?: string;
  setIsSending: (v: boolean) => void;
  /** Callback invoked after a conversation is saved/updated (e.g. to refresh lists). */
  onConversationSaved?: () => void;
  t: TFunction;
}

/**
 * 流式对话的副作用编排：assistant 缓冲更新、持久化失败提示、
 * 流结束后收敛保存、流级错误替换展示。内部经 ref 读取最新 messages/accountId，
 * 使各 effect 依赖不随消息变化而频繁重跑。
 */
export function useLlmStreaming({
  messages,
  setMessages,
  accountId,
  setIsSending,
  onConversationSaved,
  t,
}: UseLlmStreamingOptions) {
  const streamBuffer = useLlmStore((s) => s.streamBuffer);
  const isStreaming = useLlmStore((s) => s.isStreaming);
  const streamingConvId = useLlmStore((s) => s.streamingConvId);
  const streamError = useLlmStore((s) => s.streamError);
  const persistFailed = useLlmStore((s) => s.persistFailed);
  const reset = useLlmStore((s) => s.reset);

  const messagesRef = useRef(messages);
  messagesRef.current = messages;
  const accountIdRef = useRef(accountId);
  accountIdRef.current = accountId;

  /* Stream: update assistant message buffer */
  useEffect(() => {
    if (!isStreaming || !streamingConvId) return;
    setMessages((prev) => {
      if (prev.length === 0) return prev;
      const lastIdx = prev.length - 1;
      if (prev[lastIdx].role !== 'assistant') return prev;
      const updated = [...prev];
      updated[lastIdx] = { ...updated[lastIdx], content: streamBuffer };
      return updated;
    });
  }, [streamBuffer, isStreaming, streamingConvId, setMessages]);

  /* Stream: persist failure — keep displayed reply, only toast (P002-R1) */
  const persistFailedHandledRef = useRef(false);
  useEffect(() => {
    if (!persistFailed || persistFailedHandledRef.current) return;
    persistFailedHandledRef.current = true;
    // 后端 Auto-save 失败：回复已完整流式展示，不替换内容，仅提示用户记录可能未持久化。
    notifyConversationSaveFailed(t);
  }, [persistFailed, t]);

  /* Stream: finalize after done */
  useEffect(() => {
    if (!isStreaming && streamingConvId && streamBuffer) {
      // P002-R1: 持久化失败时后端已报错（Auto-save 失败）——回复已完整展示，
      // 由 persistFailed effect 提示；此处跳过前端再次保存（重试大概率同样失败，
      // 且避免「后端失败 toast + 前端重试成功」的重复/矛盾提示），仅收敛状态。
      const wasPersistFailed = useLlmStore.getState().persistFailed;
      if (!wasPersistFailed) {
        const convId = streamingConvId;
        const currentMsgs = messagesRef.current;
        if (currentMsgs.length > 0 && currentMsgs[currentMsgs.length - 1].role === 'assistant') {
          const firstUser = currentMsgs.find((m) => m.role === 'user');
          const convName = firstUser ? firstUser.content.slice(0, 30) : '';
          const finalConv: Conversation = {
            id: convId,
            name: convName,
            isTemporary: false,
            messages: currentMsgs,
            updatedAt: nowISO(),
          };
          void saveConversationSafely(accountIdRef.current, finalConv, t);
          onConversationSaved?.();
        }
      }
      reset();
      setIsSending(false);
      persistFailedHandledRef.current = false;
    }
  }, [isStreaming, streamingConvId, streamBuffer, onConversationSaved, t, reset, setIsSending]);

  /* Stream: error handling */
  useEffect(() => {
    if (streamError) {
      const errMsg = streamError;
      // P002-R1: 仅流级错误（生成中断）替换已展示内容；持久化失败走
      // persistFailed 分支（保留回复、只 toast），不进入此路径。
      setMessages((prev) => {
        if (prev.length === 0) return prev;
        const lastIdx = prev.length - 1;
        if (prev[lastIdx].role !== 'assistant') return prev;
        const updated = [...prev];
        updated[lastIdx] = {
          ...updated[lastIdx],
          content: `${t('settings:ai_chat_error_prefix')}: ${errMsg}`,
          isError: true,
        };
        return updated;
      });
      reset();
      setIsSending(false);
      persistFailedHandledRef.current = false;
    }
  }, [streamError, t, reset, setMessages, setIsSending]);
}
