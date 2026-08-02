import { useState, useEffect, useCallback, useRef } from 'react';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { useAuthStore } from '@/stores/authStore';
import { useLlmStore } from '@/stores/llmStore';
import { useUiStore } from '@/stores/uiStore';
import i18n from '@/lib/i18n';
import { COPY_FEEDBACK_DURATION_MS } from '@/lib/constants';
import { logger } from '@/lib/logger';
import { useTranslation } from 'react-i18next';
import {
  buildSystemPrompt,
  buildMessagesWithSystemPromptAndGuide,
} from '@/lib/llm/systemPromptBuilder';
import { searchGuideChunks, formatChunksAsSystemMessage } from '@/lib/llm/guideService';
import { markConversationPending } from '@/lib/notification';
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
  const isStreaming = useLlmStore((s) => s.isStreaming);
  const streamingConvId = useLlmStore((s) => s.streamingConvId);
  const streamError = useLlmStore((s) => s.streamError);
  const startStream = useLlmStore((s) => s.startStream);
  const onChunk = useLlmStore((s) => s.onChunk);
  const reset = useLlmStore((s) => s.reset);

  const [activeProvider, setActiveProvider] = useState<ActiveProvider | null>(null);
  const [isConfigured, setIsConfigured] = useState(false);
  const [isAiEnabled, setIsAiEnabled] = useState(false);
  const [loading, setLoading] = useState(true);

  const [conversations, setConversations] = useState<ConversationSummary[]>([]);
  const [currentConvId, setCurrentConvId] = useState<string | null>(null);
  const [messages, setMessages] = useState<ChatMsg[]>([]);
  const [input, setInput] = useState('');
  const [isSending, setIsSending] = useState(false);
  const [isOnline, setIsOnline] = useState<boolean | null>(null);
  const [checkingOnline, setCheckingOnline] = useState(false);
  const [copiedIndex, setCopiedIndex] = useState<number | null>(null);
  const copyTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const messagesRef = useRef(messages);
  messagesRef.current = messages;
  const accountIdRef = useRef(accountId);
  accountIdRef.current = accountId;
  const currentConvIdRef = useRef(currentConvId);
  currentConvIdRef.current = currentConvId;

  /* Load provider + config */
  useEffect(() => {
    if (!accountId) {
      setLoading(false);
      return;
    }
    (async () => {
      try {
        const cfg = await invoke<{
          activeProviderId?: string;
          aiFeaturesEnabled?: { chat: boolean };
          includeSystemPrompt?: boolean;
        }>('llm_get_config', { accountId: accountId });
        setIsAiEnabled(cfg.aiFeaturesEnabled?.chat ?? false);
        if (!cfg.activeProviderId) {
          setIsConfigured(false);
          setLoading(false);
          return;
        }
        const providers = await invoke<
          Array<{ id: string; name: string; model: string; baseUrl: string; apiType: string }>
        >('llm_get_providers', { accountId: accountId });
        const active = providers.find((p) => p.id === cfg.activeProviderId);
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
      } catch {
        setIsConfigured(false);
      } finally {
        setLoading(false);
      }
    })();
  }, [accountId]);

  /* Load conversation list */
  const loadConversationList = useCallback(async () => {
    if (!accountId || !isAiEnabled || !isConfigured) return;
    abortRef.current?.abort();
    const controller = new AbortController();
    abortRef.current = controller;
    try {
      const list = await invoke<ConversationSummary[]>('llm_list_conversations', { accountId: accountId });
      if (!controller.signal.aborted) setConversations(list);
    } catch {
      /* ignore */
    }
  }, [accountId, isAiEnabled, isConfigured]);

  useEffect(() => {
    loadConversationList();
  }, [loadConversationList]);

  /* Online status */
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
      } catch {
        if (!controller.signal.aborted) setIsOnline(false);
      } finally {
        if (!controller.signal.aborted) setCheckingOnline(false);
      }
    })();
  }, [activeProvider, accountId]);

  useEffect(() => {
    if (activeProvider && accountId) checkOnline();
  }, [activeProvider, accountId, checkOnline]);

  useEffect(() => {
    if (!activeProvider) return;
    const interval = setInterval(checkOnline, 60000);
    return () => clearInterval(interval);
  }, [activeProvider, checkOnline]);

  /* Cleanup copy timeout */
  useEffect(() => {
    return () => {
      if (copyTimeoutRef.current) clearTimeout(copyTimeoutRef.current);
    };
  }, []);

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
      } catch {
        /* may be deleted */
      }
    },
    [accountId],
  );

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
  }, [streamBuffer, isStreaming, streamingConvId]);

  /* Stream: finalize after done */
  useEffect(() => {
    if (!isStreaming && streamingConvId && streamBuffer) {
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
        invoke('llm_save_conversation', {
          accountId: accountIdRef.current,
          conversation: finalConv,
        }).catch((err) => {
          // P007: 保存失败不得静默——提示用户记录可能丢失，避免丢失后无感知。
          logger.warn('[useLlmChatCore] Save conversation failed:', err);
          useUiStore.getState().showToast({
            type: 'error',
            message: t('settings:ai_save_conversation_failed', {
              defaultValue: '对话保存失败，记录可能丢失，请重试',
            }),
            duration: 5000,
          });
        });
        onConversationSaved?.();
      }
      reset();
      setIsSending(false);
    }
  }, [
    isStreaming,
    streamingConvId,
    streamBuffer,
    onConversationSaved,
    t,
    reset,
  ]);

  /* Stream: error handling */
  useEffect(() => {
    if (streamError) {
      const errMsg = streamError;
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
    }
  }, [streamError, t, reset]);

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
      try {
        await invoke('llm_save_conversation', { accountId: accountId, conversation: partialConv });
        onConversationSaved?.();
      } catch (err) {
        // P007: 首次保存失败若静默，整段新对话将不会被持久化且无提示。
        logger.warn('[useLlmChatCore] First conversation save failed:', err);
        useUiStore.getState().showToast({
          type: 'error',
          message: t('settings:ai_save_conversation_failed', {
            defaultValue: '对话保存失败，记录可能丢失，请重试',
          }),
          duration: 5000,
        });
      }
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

      let allMessages: Array<{ role: string; content: string }> = [];
      const effectiveIncludeSystemPrompt = optIncludeSystemPrompt ?? true;
      if (effectiveIncludeSystemPrompt) {
        const systemPrompt = buildSystemPrompt();
        const chunks = await searchGuideChunks(text, i18n.language || 'zh-CN');
        const docPrompt = formatChunksAsSystemMessage(chunks);
        allMessages = buildMessagesWithSystemPromptAndGuide(
          text,
          updatedMessages,
          systemPrompt,
          docPrompt,
        );
      } else {
        allMessages = updatedMessages.map((m) => ({ role: m.role, content: m.content }));
        allMessages.push({ role: 'user', content: text });
      }

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
      try {
        await invoke('llm_save_conversation', { accountId: accountId, conversation: errorConv });
      } catch (err) {
        // P007: 错误会话的保存失败同样不应静默。
        logger.warn('[useLlmChatCore] Save error conversation failed:', err);
        useUiStore.getState().showToast({
          type: 'error',
          message: t('settings:ai_save_conversation_failed', {
            defaultValue: '对话保存失败，记录可能丢失，请重试',
          }),
          duration: 5000,
        });
      }
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

  const handleCopy = useCallback(async (content: string, index: number) => {
    try {
      await navigator.clipboard.writeText(content);
      setCopiedIndex(index);
      if (copyTimeoutRef.current) clearTimeout(copyTimeoutRef.current);
      copyTimeoutRef.current = setTimeout(() => setCopiedIndex(null), COPY_FEEDBACK_DURATION_MS);
    } catch {
      /* fallback */
    }
  }, []);

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
