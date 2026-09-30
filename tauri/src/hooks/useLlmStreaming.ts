import { useEffect, useRef } from 'react';
import type { TFunction } from 'i18next';
import { useLlmStore, selectLlmStream } from '@/stores/llmStore';
import { notifyConversationSaveFailed } from '@/lib/llm/conversationPersistence';
import type { ChatMsg } from '@/types/llmChat';

export interface UseLlmStreamingOptions {
  messages: ChatMsg[];
  accountId?: string;
  currentConvId: string | null;
  onConversationSaved?: () => void;
  t: TFunction;
}
/** 只投影归属当前账户/会话的请求，不修改别的会话或在结束时再次保存。 */
export function useLlmStreaming({
  messages,
  accountId,
  currentConvId,
  onConversationSaved,
  t,
}: UseLlmStreamingOptions): ChatMsg[] {
  const stream = useLlmStore((state) => selectLlmStream(state, accountId, currentConvId));
  const streams = useLlmStore((state) => state.streams);
  const completed = useRef(new Set<string>());
  useEffect(() => {
    for (const record of Object.values(streams)) {
      if (record.accountId !== accountId) continue;
      if (useLlmStore.getState().claimPersistFailure(record)) notifyConversationSaveFailed(t);
      if (
        record.settled &&
        !record.error &&
        !record.persistFailed &&
        !completed.current.has(record.requestId)
      ) {
        completed.current.add(record.requestId);
        onConversationSaved?.();
      }
    }
  }, [streams, accountId, onConversationSaved, t]);
  if (!stream || !useLlmStore.getState().isCurrent(stream)) return messages;
  return stream.messages.map((message) =>
    message.id === stream.assistantMessageId && message.role === 'assistant'
      ? {
          ...message,
          content: stream.error
            ? `${t('settings:ai_chat_error_prefix')}: ${stream.error}`
            : stream.buffer,
          ...(stream.error ? { isError: true } : {}),
        }
      : message,
  );
}
