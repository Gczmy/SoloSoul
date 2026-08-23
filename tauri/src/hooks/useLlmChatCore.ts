import { useState, useEffect, useCallback, useRef } from 'react';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { useAuthStore } from '@/stores/authStore';
import { useLlmStore } from '@/stores/llmStore';
import { COPY_FEEDBACK_DURATION_MS } from '@/lib/constants';
import { useCopyToClipboard } from '@/hooks/useCopyToClipboard';
import { logger } from '@/lib/logger';
import { useTranslation } from 'react-i18next';
import { markConversationPending } from '@/lib/notification';
import { useLlmProviderConfig } from '@/hooks/useLlmProviderConfig';
import { useLlmOnlineStatus } from '@/hooks/useLlmOnlineStatus';
import { useLlmStreaming } from '@/hooks/useLlmStreaming';
import { buildChatRequestMessages } from '@/lib/llm/chatRequest';
import { saveConversationSafely } from '@/lib/llm/conversationPersistence';
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
  sendMessage: () => Promise<void>;
  loadConversation: (convId: string) => Promise<void>;
  loadConversationList: () => Promise<void>;
  handleCopy: (content: string, index: number) => Promise<void>;
  checkOnline: () => void;
}

export function useLlmChatCore(options: UseLlmChatCoreOptions = {}): UseLlmChatCoreReturn {
  const { includeSystemPrompt: optIncludeSystemPrompt, onConversationSaved } = options;

  const { t } = useTranslation(['settings', 'common']);
  const accountId = useAuthStore((s) => s.currentAccount?.id);
  const abortRef = useRef<AbortController | null>(null);
  // P117: 字段级选择器——避免整店订阅导致每次 token 更新整页重渲染；
  // action（startStream/onChunk/reset）在 store 中定义一次，引用稳定，
  // 使 useCallback 依赖不随 store 更新而漂移。
  const streamBuffer = useLlmStore((s) => s.streamBuffer);
  const startStream = useLlmStore((s) => s.startStream);
  const onChunk = useLlmStore((s) => s.onChunk);
  const reset = useLlmStore((s) => s.reset);

  const [conversations, setConversations] = useState<ConversationSummary[]>([]);
  const [currentConvId, setCurrentConvId] = useState<string | null>(null);
  const [messages, setMessages] = useState<ChatMsg[]>([]);
  const [input, setInput] = useState('');
  const [isSending, setIsSending] = useState(false);
  // P025：复制反馈收敛至共享 hook（按消息下标键控）
  const { copy, copiedKey } = useCopyToClipboard(COPY_FEEDBACK_DURATION_MS);
  const copiedIndex = copiedKey === null ? null : Number(copiedKey);

  // 子 hook：provider 配置加载 / 在线状态轮询 / 流式副作用编排
  const { activeProvider, isConfigured, isAiEnabled, loading } = useLlmProviderConfig({ accountId });
  const { isOnline, checkingOnline, checkOnline } = useLlmOnlineStatus({
    activeProvider,
    accountId,
    abortRef,
  });
  useLlmStreaming({
    messages,
    setMessages,
    accountId,
    setIsSending,
    onConversationSaved,
    t,
  });

  /* Load conversation list */
  const loadConversationList = useCallback(async () => {
    if (!accountId || !isAiEnabled || !isConfigured) return;
    abortRef.current?.abort();
    const controller = new AbortController();
    abortRef.current = controller;
    try {
      const list = await invoke<ConversationSummary[]>('llm_list_conversations', { accountId: accountId });
      if (!controller.signal.aborted) setConversations(list);
    } catch (err) {
      // P227: 会话列表加载失败静默降级（列表留空），留痕。
      logger.warn('[useLlmChatCore] Load conversation list failed:', err);
    }
  }, [accountId, isAiEnabled, isConfigured]);

  useEffect(() => {
    loadConversationList();
  }, [loadConversationList]);

  /* Load single conversation */
  const loadConversation = useCallback(
    async (convId: string) => {
      if (!accountId) return;
      try {
        const conv = await invoke<Conversation>('llm_get_conversation', {
          accountId: accountId,
          conversationId: convId,
        });
        setCurrentConvId(conv.id);
        setMessages(conv.messages.map((m) => (m.id ? m : { ...m, id: generateId() })));
      } catch (err) {
        // P227: 会话可能已被删除（可接受降级），留痕。
        logger.warn('[useLlmChatCore] Load conversation failed:', err);
      }
    },
    [accountId],
  );

  /* Send message */
  const sendMessage = useCallback(async () => {
    const text = input.trim();
    if (!text || !activeProvider || !accountId) return;

    const ts = nowISO();
    const userMsg: ChatMsg = { id: generateId(), role: 'user', content: text, createdAt: ts };
    const updatedMessages = [...messages, userMsg];
    setMessages(updatedMessages);
    setInput('');
    setIsSending(true);

    const convId = currentConvId || generateId();
    setCurrentConvId(convId);

    const isFirstMsg = messages.length === 0;
    const convName = isFirstMsg ? text.slice(0, 30) : '';

    if (isFirstMsg) {
      const partialConv: Conversation = {
        id: convId,
        name: convName,
        isTemporary: false,
        messages: updatedMessages,
        updatedAt: nowISO(),
      };
      // P007: 首次保存失败若静默，整段新对话将不会被持久化且无提示。
      const saved = await saveConversationSafely(accountId, partialConv, t);
      if (saved) onConversationSaved?.();
    }

    const assistantMsg: ChatMsg = {
      id: generateId(),
      role: 'assistant',
      content: '',
      createdAt: nowISO(),
    };
    const streamingMessages = [...updatedMessages, assistantMsg];
    setMessages(streamingMessages);
    startStream(convId);

    try {
      const apiKey = await invoke<string>('llm_get_api_key', {
        accountId: accountId,
        providerId: activeProvider.id,
      });

      const effectiveIncludeSystemPrompt = optIncludeSystemPrompt ?? true;
      const allMessages = await buildChatRequestMessages({
        text,
        history: updatedMessages,
        includeSystemPrompt: effectiveIncludeSystemPrompt,
      });

      markConversationPending(convId);

      invoke('llm_send_message_stream', {
        accountId: accountId,
        conversationId: convId,
        baseUrl: activeProvider.baseUrl,
        apiKey: apiKey,
        model: activeProvider.model,
        apiType: activeProvider.apiType,
        messages: allMessages,
      }).catch((err) => {
        onChunk({
          conversationId: convId,
          chunk: '',
          isDone: false,
          error: String(err),
        });
      });
    } catch (e) {
      const errMsg = typeof e === 'string' ? e : e instanceof Error ? e.message : String(e);
      const errorAssistantMsg: ChatMsg = {
        id: generateId(),
        role: 'assistant',
        content: `${t('settings:ai_chat_error_prefix')}: ${errMsg}`,
        createdAt: nowISO(),
        isError: true,
      };
      const errorMessages = [...updatedMessages, errorAssistantMsg];
      setMessages(errorMessages);

      const errorConv: Conversation = {
        id: convId,
        name: convName,
        isTemporary: false,
        messages: errorMessages,
        updatedAt: nowISO(),
      };
      // P007: 错误会话的保存失败同样不应静默（saveConversationSafely 内已 toast）。
      await saveConversationSafely(accountId, errorConv, t);
      reset();
      setIsSending(false);
    }
  }, [
    input,
    activeProvider,
    accountId,
    messages,
    currentConvId,
    optIncludeSystemPrompt,
    startStream,
    onChunk,
    reset,
    onConversationSaved,
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
    messages,
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
    sendMessage,
    loadConversation,
    loadConversationList,
    handleCopy,
    checkOnline,
  };
}
