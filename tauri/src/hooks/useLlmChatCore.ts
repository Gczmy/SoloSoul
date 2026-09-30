import { useState, useEffect, useCallback, useRef } from 'react';
import { useAuthStore } from '@/stores/authStore';
import { useLlmStore, selectLlmStream, isConversationBusy } from '@/stores/llmStore';
import { COPY_FEEDBACK_DURATION_MS } from '@/lib/constants';
import { useCopyToClipboard } from '@/hooks/useCopyToClipboard';
import { logger } from '@/lib/logger';
import { useTranslation } from 'react-i18next';
import { markConversationPending } from '@/lib/notification';
import { useLlmProviderConfig } from '@/hooks/useLlmProviderConfig';
import { useLlmOnlineStatus } from '@/hooks/useLlmOnlineStatus';
import { useLlmStreaming } from '@/hooks/useLlmStreaming';
import { buildChatRequest } from '@/lib/llm/chatRequest';
import { saveConversationSafely } from '@/lib/llm/conversationPersistence';
import { createSessionRequests, onRequestSessionChange } from '@/lib/sessionRequests';
import {
  type ChatMsg,
  type Conversation,
  type ConversationSummary,
  type ActiveProvider,
  nowISO,
  isOllama,
  generateId,
} from '@/types/llmChat';

export type { ChatMsg, ConversationSummary, Conversation, ActiveProvider };

export interface UseLlmChatCoreOptions {
  /** Whether to include system prompt when sending messages. */
  includeSystemPrompt?: boolean;
  /** Callback invoked after a conversation is saved/updated (e.g. to refresh lists). */
  onConversationSaved?: () => void;
}

export interface UseLlmChatCoreReturn {
  activeProvider: ActiveProvider | null;
  isConfigured: boolean;
  isAiEnabled: boolean;
  loading: boolean;
  conversations: ConversationSummary[];
  setConversations: React.Dispatch<React.SetStateAction<ConversationSummary[]>>;
  messages: ChatMsg[];
  input: string;
  isSending: boolean;
  isOnline: boolean | null;
  checkingOnline: boolean;
  copiedIndex: number | null;
  isLocal: boolean;
  currentConvId: string | null;
  streamBuffer: string;
  setInput: (v: string) => void;
  setMessages: React.Dispatch<React.SetStateAction<ChatMsg[]>>;
  setCurrentConvId: (v: string | null) => void;
  isCurrentConversation: (id: string) => boolean;
  sendMessage: () => Promise<void>;
  loadConversation: (convId: string) => Promise<void>;
  loadConversationList: () => Promise<void>;
  invalidateReads: () => void;
  handleCopy: (content: string, index: number) => Promise<void>;
  checkOnline: () => void;
}

export function useLlmChatCore(options: UseLlmChatCoreOptions = {}): UseLlmChatCoreReturn {
  const { includeSystemPrompt: optIncludeSystemPrompt, onConversationSaved } = options;

  const { t } = useTranslation(['settings', 'common']);
  const accountId = useAuthStore((s) => s.currentAccount?.id);
  const abortRef = useRef<AbortController | null>(null);
  const [readRequests] = useState(createSessionRequests);
  // P117: 字段级选择器——避免整店订阅导致每次 token 更新整页重渲染；
  // action（startStream/onChunk/reset）在 store 中定义一次，引用稳定，
  // 使 useCallback 依赖不随 store 更新而漂移。
  const startStream = useLlmStore((s) => s.startStream);

  const [conversations, setConversations] = useState<ConversationSummary[]>([]);
  const [currentConvId, setCurrentConvIdState] = useState<string | null>(null);
  const currentConvIdRef = useRef<string | null>(null);
  const isCurrentConversation = useCallback((id: string) => currentConvIdRef.current === id, []);
  const [messages, setMessages] = useState<ChatMsg[]>([]);
  const [input, setInput] = useState('');
  const loadedConversation = useRef<Conversation | null>(null);
  const isSending = useLlmStore((state) => isConversationBusy(state, accountId, currentConvId));
  const streamBuffer = useLlmStore(
    (state) => selectLlmStream(state, accountId, currentConvId)?.buffer ?? '',
  );
  const invalidateReads = useCallback(() => readRequests.invalidate(), [readRequests]);
  const setCurrentConvId = useCallback(
    (id: string | null) => {
      readRequests.invalidate('body');
      currentConvIdRef.current = id;
      if (loadedConversation.current?.id !== id) loadedConversation.current = null;
      setCurrentConvIdState(id);
    },
    [readRequests],
  );
  useEffect(() => {
    const clear = () => {
      invalidateReads();
      setConversations([]);
      currentConvIdRef.current = null;
      setCurrentConvIdState(null);
      setMessages([]);
      setInput('');
      loadedConversation.current = null;
    };
    clear();
    const unsubscribe = onRequestSessionChange(clear);
    return () => {
      unsubscribe();
      invalidateReads();
    };
  }, [accountId, invalidateReads]);
  // P025：复制反馈收敛至共享 hook（按消息下标键控）
  const { copy, copiedKey } = useCopyToClipboard(COPY_FEEDBACK_DURATION_MS);
  const copiedIndex = copiedKey === null ? null : Number(copiedKey);

  // 子 hook：provider 配置加载 / 在线状态轮询 / 流式副作用编排
  const {
    activeProvider,
    isConfigured,
    isAiEnabled,
    includeSystemPrompt: savedIncludeSystemPrompt,
    loading,
  } = useLlmProviderConfig({ accountId });
  const { isOnline, checkingOnline, checkOnline } = useLlmOnlineStatus({
    activeProvider,
    accountId,
    abortRef,
  });
  const visibleMessages = useLlmStreaming({
    messages,
    accountId,
    currentConvId,
    onConversationSaved,
    t,
  });

  /* Load conversation list */
  const loadConversationList = useCallback(async () => {
    if (!accountId || !isAiEnabled || !isConfigured) return;
    const request = readRequests.begin('list', accountId);
    try {
      const list = await request.invoke<ConversationSummary[]>('llm_list_conversations', {
        accountId: accountId,
      });
      if (request.isCurrent()) setConversations(list);
    } catch (err) {
      // P227: 会话列表加载失败静默降级（列表留空），留痕。
      if (request.isCurrent()) logger.warn('[useLlmChatCore] Load conversation list failed:', err);
    }
  }, [accountId, isAiEnabled, isConfigured, readRequests]);

  useEffect(() => {
    loadConversationList();
  }, [loadConversationList]);

  /* Load single conversation */
  const loadConversation = useCallback(
    async (convId: string) => {
      if (!accountId) return;
      const request = readRequests.begin('body', accountId);
      try {
        const conv = await request.invoke<Conversation>('llm_get_conversation', {
          accountId: accountId,
          conversationId: convId,
        });
        if (!request.isCurrent()) return;
        loadedConversation.current = conv;
        currentConvIdRef.current = conv.id;
        setCurrentConvIdState(conv.id);
        setMessages(conv.messages.map((m) => (m.id ? m : { ...m, id: generateId() })));
      } catch (err) {
        // P227: 会话可能已被删除（可接受降级），留痕。
        if (request.isCurrent()) logger.warn('[useLlmChatCore] Load conversation failed:', err);
      }
    },
    [accountId, readRequests],
  );

  /* 每轮先保存用户历史，最终回复唯一由后端保存。整个请求与视图选择无关。 */
  const sendMessage = useCallback(async () => {
    const text = input.trim();
    if (!text || !activeProvider || !accountId || isSending) return;
    const convId = currentConvId || generateId();
    const identity = { accountId, conversationId: convId, requestId: crypto.randomUUID() };
    const userMsg: ChatMsg = { id: generateId(), role: 'user', content: text, createdAt: nowISO() };
    let userMessages = [...visibleMessages, userMsg];
    const assistant: ChatMsg = {
      id: generateId(),
      role: 'assistant',
      content: '',
      createdAt: nowISO(),
    };
    const wasStored =
      loadedConversation.current?.id === convId ||
      selectLlmStream(useLlmStore.getState(), accountId, convId)?.persisted === true;
    const run = startStream({
      ...identity,
      assistantMessageId: assistant.id!,
      messages: [...userMessages, assistant],
    });
    if (!run) return; // 两个入口的同一会话只能有一个发送/元数据修改者。
    if (wasStored) useLlmStore.getState().markConversationPersisted(identity);
    setCurrentConvId(convId);
    setMessages(userMessages);
    setInput('');
    try {
      await run.ready; // 注册完成后才发送，不能漏掉最早的chunk。
      let conversation: Conversation;
      let history = visibleMessages;
      if (wasStored) {
        const stored = await run.invoke<Conversation>('llm_get_conversation', {
          accountId,
          conversationId: convId,
        });
        if (!stored || stored.id !== convId || stored.deletedAt)
          throw new Error('Conversation unavailable');
        // 另一入口可能已更新历史；发送和预保存必须采用刚读到的规范历史。
        history = stored.messages.map((message) => ({
          ...message,
          id: message.id || generateId(),
        }));
        userMessages = [...history, userMsg];
        conversation = { ...stored, messages: userMessages, updatedAt: nowISO() };
      } else {
        conversation = {
          id: convId,
          name: text.slice(0, 30),
          isTemporary: false,
          messages: userMessages,
          updatedAt: nowISO(),
        };
      }
      useLlmStore.getState().prepareStream(identity, [...userMessages, assistant]);
      const request = await buildChatRequest({
        text,
        history,
        includeSystemPrompt: optIncludeSystemPrompt !== false && savedIncludeSystemPrompt !== false,
      });
      run.assertCurrent();
      const saved = await saveConversationSafely(accountId, conversation, t, run);
      if (!run.isCurrent()) return;
      if (!saved) {
        useLlmStore.getState().cancelStream(identity, wasStored ? history : undefined);
        if (isCurrentConversation(convId)) {
          setMessages(history);
          setInput((draft) => draft || text);
        }
        return;
      }
      useLlmStore.getState().markConversationPersisted(identity);
      markConversationPending(identity);
      await run.invoke('llm_send_message_stream', {
        accountId,
        conversationId: convId,
        requestId: identity.requestId,
        providerId: activeProvider.id,
        messages: request.messages,
        contextSelection: request.contextSelection,
      });
      run.assertCurrent();
      const state = useLlmStore.getState();
      if (!selectLlmStream(state, accountId, convId)?.persistFailed) {
        try {
          const stored = await run.invoke<Conversation>('llm_get_conversation', {
            accountId,
            conversationId: convId,
          });
          // emit是best-effort；用Host已保存的规范正文补齐遗漏事件，不补写最终回复。
          const valid =
            stored?.id === convId &&
            stored.messages.length === userMessages.length + 1 &&
            userMessages.every(
              (message, index) =>
                stored.messages[index].role === message.role &&
                stored.messages[index].content === message.content,
            ) &&
            stored.messages.at(-1)?.role === 'assistant';
          if (!valid) throw new Error('Conversation completion was not confirmed');
          useLlmStore.getState().finishStream(identity, undefined, stored.messages.at(-1)!.content);
        } catch {
          if (!run.isCurrent()) return;
          useLlmStore.getState().markPersistFailure(identity);
          useLlmStore.getState().finishStream(identity);
        }
      } else state.finishStream(identity);
    } catch (error) {
      if (!run.isCurrent()) return;
      const message =
        typeof error === 'string' ? error : error instanceof Error ? error.message : String(error);
      useLlmStore.getState().finishStream(identity, message);
    }
  }, [
    input,
    activeProvider,
    accountId,
    isSending,
    currentConvId,
    visibleMessages,
    optIncludeSystemPrompt,
    savedIncludeSystemPrompt,
    startStream,
    setCurrentConvId,
    isCurrentConversation,
    t,
  ]);

  const handleCopy = useCallback(
    async (content: string, index: number) => {
      const ok = await copy(content, String(index));
      if (!ok) {
        // P227: 剪贴板写入失败（权限拒绝等）静默降级，留痕。
        logger.warn('[useLlmChatCore] Copy to clipboard failed');
      }
    },
    [copy],
  );

  const isLocal = activeProvider ? isOllama(activeProvider.baseUrl) : false;

  return {
    activeProvider,
    isConfigured,
    isAiEnabled,
    loading,
    conversations,
    setConversations,
    messages: visibleMessages,
    input,
    isSending,
    isOnline,
    checkingOnline,
    copiedIndex,
    isLocal,
    currentConvId,
    streamBuffer,
    setInput,
    setMessages,
    setCurrentConvId,
    isCurrentConversation,
    sendMessage,
    loadConversation,
    loadConversationList,
    invalidateReads,
    handleCopy,
    checkOnline,
  };
}
