import { useState, useRef, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useAuthStore } from '@/stores/authStore';
import { useLlmStore } from '@/stores/llmStore';
import { useCancellable } from '@/hooks/useCancellable';
import i18n from '@/lib/i18n';
import { COPY_FEEDBACK_DURATION_MS } from '@/lib/constants';
import {
  buildSystemPrompt,
  buildMessagesWithSystemPromptAndChunks,
} from '@/lib/llm/systemPromptBuilder';
import { searchGuideChunks, formatChunksAsSystemMessage } from '@/lib/llm/guideService';
import { markConversationPending, setAiPageOpen } from '@/lib/notification';
import { useTranslation } from 'react-i18next';
import type { ChatMsg }    from '../ChatMessageBubble';

export interface Conversation {
  id: string;
  name: string;
  isTemporary: boolean;
  messages: ChatMsg[];
  updatedAt: string;
  deletedAt?: string;
}

export interface ConversationSummary {
  id: string;
  name: string;
  updatedAt: string;
  messageCount: number;
  deletedAt?: string;
}

function isOllama(baseUrl: string): boolean {
  return (
    baseUrl.toLowerCase().includes('localhost') || baseUrl.toLowerCase().includes('127.0.0.1')
  );
}

function nowISO(): string {
  return new Date().toISOString();
}

function generateId(): string {
  return 'conv_' + crypto.randomUUID();
}

export interface UseLlmChatReturn {
  activeProvider: {
    id: string;
    name: string;
    model: string;
    baseUrl: string;
    apiType: string;
  } | null;
  isConfigured: boolean;
  isAiEnabled: boolean;
  includeSystemPrompt: boolean;
  loading: boolean;
  conversations: ConversationSummary[];
  trashList: ConversationSummary[];
  showTrash: boolean;
  currentConvId: string | null;
  currentConv: Conversation | null;
  messages: ChatMsg[];
  input: string;
  isSending: boolean;
  isOnline: boolean | null;
  checkingOnline: boolean;
  copiedIndex: number | null;
  floatingConv: Conversation | null;
  confirmPermanentDelete: string | null;
  isLocal: boolean;
  setInput: (v: string) => void;
  setShowTrash: (v: boolean) => void;
  setConfirmPermanentDelete: (v: string | null) => void;
  setFloatingConv: (v: Conversation | null) => void;
  sendMessage: () => Promise<void>;
  handleNewConversation: () => void;
  loadConversation: (convId: string) => Promise<void>;
  handleRename: (convId: string, newName: string) => Promise<void>;
  handleSoftDelete: (convId: string) => Promise<void>;
  handleRestore: (convId: string) => Promise<void>;
  handlePermanentDelete: (convId: string) => Promise<void>;
  handleViewTrashConv: (convId: string) => Promise<void>;
  handleCopy: (content: string, index: number) => Promise<void>;
  checkOnline: () => void;
}

export function useLlmChat(): UseLlmChatReturn {
  const { t } = useTranslation(['settings', 'common']);
  const accountId = useAuthStore((s) => s.currentAccount?.id);
  const makeCancellable = useCancellable();

  const [activeProvider, setActiveProvider] = useState<{
    id: string;
    name: string;
    model: string;
    baseUrl: string;
    apiType: string;
  } | null>(null);
  const [isConfigured, setIsConfigured] = useState(false);
  const [isAiEnabled, setIsAiEnabled] = useState(false);
  const [includeSystemPrompt, setIncludeSystemPrompt] = useState(true);
  const [loading, setLoading] = useState(true);

  const [conversations, setConversations] = useState<ConversationSummary[]>([]);
  const [trashList, setTrashList] = useState<ConversationSummary[]>([]);
  const [showTrash, setShowTrash] = useState(false);
  const [currentConvId, setCurrentConvId] = useState<string | null>(null);
  const [currentConv, setCurrentConv] = useState<Conversation | null>(null);
  const [messages, setMessages] = useState<ChatMsg[]>([]);
  const [input, setInput] = useState('');
  const [isSending, setIsSending] = useState(false);
  const [isOnline, setIsOnline] = useState<boolean | null>(null);
  const [checkingOnline, setCheckingOnline] = useState(false);
  const [copiedIndex, setCopiedIndex] = useState<number | null>(null);
  const copyTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [floatingConv, setFloatingConv] = useState<Conversation | null>(null);
  const [confirmPermanentDelete, setConfirmPermanentDelete] = useState<string | null>(null);

  const messagesRef = useRef(messages);
  messagesRef.current = messages;
  const currentConvRef = useRef(currentConv);
  currentConvRef.current = currentConv;
  const accountIdRef = useRef(accountId);
  accountIdRef.current = accountId;

  useEffect(() => {
    setAiPageOpen(true);
    return () => setAiPageOpen(false);
  }, []);

  /* Load provider + config */
  useEffect(() => {
    if (!accountId) return;
    (async () => {
      try {
        const cfg = await invoke<{
          activeProviderId?: string;
          aiFeaturesEnabled?: { chat: boolean };
          includeSystemPrompt?: boolean;
        }>('llm_get_config', { accountId });
        setIsAiEnabled(cfg.aiFeaturesEnabled?.chat ?? false);
        setIncludeSystemPrompt(cfg.includeSystemPrompt ?? true);
        if (!cfg.activeProviderId) {
          setIsConfigured(false);
          setLoading(false);
          return;
        }
        const providers = await invoke<
          Array<{ id: string; name: string; model: string; baseUrl: string; apiType: string }>
        >('llm_get_providers', { accountId });
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

  /* Load conversation & trash lists */
  const loadAllLists = useCallback(() => {
    if (!accountId || !isAiEnabled || !isConfigured) return;
    const { isCancelled } = makeCancellable();
    Promise.all([
      invoke<ConversationSummary[]>('llm_list_conversations', { accountId }),
      invoke<ConversationSummary[]>('llm_list_trash', { accountId }),
    ])
      .then(([list, trash]) => {
        if (!isCancelled()) {
          setConversations(list);
          setTrashList(trash);
        }
      })
      .catch(() => {});
  }, [accountId, isAiEnabled, isConfigured, makeCancellable]);

  const loadAllListsRef = useRef(loadAllLists);
  loadAllListsRef.current = loadAllLists;

  useEffect(() => {
    loadAllLists();
  }, [loadAllLists]);

  /* Online status */
  const checkOnline = useCallback(() => {
    if (!activeProvider || !accountId) return;
    const { isCancelled } = makeCancellable();
    setCheckingOnline(true);
    (async () => {
      try {
        let key = '';
        try {
          key = await invoke<string>('llm_get_api_key', {
            accountId,
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
        if (!isCancelled()) setIsOnline(online);
      } catch {
        if (!isCancelled()) setIsOnline(false);
      } finally {
        if (!isCancelled()) setCheckingOnline(false);
      }
    })();
  }, [activeProvider, accountId, makeCancellable]);

  useEffect(() => {
    if (activeProvider && accountId) checkOnline();
  }, [activeProvider, accountId, checkOnline]);

  useEffect(() => {
    if (!activeProvider) return;
    const interval = setInterval(checkOnline, 60000);
    return () => clearInterval(interval);
  }, [activeProvider, checkOnline]);

  /* Scroll to bottom */
  const lastMessageKey = messages.length > 0 ? messages[messages.length - 1].createdAt : null;
  useEffect(() => {
    const el = document.querySelector('[data-chat-end]');
    el?.scrollIntoView({ behavior: 'auto' });
  }, [lastMessageKey]);

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
          accountId,
          conversationId: convId,
        });
        setCurrentConvId(conv.id);
        setCurrentConv(conv);
        setMessages(conv.messages);
      } catch {
        /* may be deleted */
      }
    },
    [accountId],
  );

  const handleNewConversation = useCallback(() => {
    const id = generateId();
    setCurrentConvId(id);
    setCurrentConv({ id, name: '', isTemporary: true, messages: [], updatedAt: nowISO() });
    setMessages([]);
  }, []);

  const llmStore = useLlmStore();

  /* Stream: update assistant message buffer */
  useEffect(() => {
    if (!llmStore.isStreaming || !llmStore.streamingConvId) return;
    setMessages((prev) => {
      if (prev.length === 0) return prev;
      const lastIdx = prev.length - 1;
      if (prev[lastIdx].role !== 'assistant') return prev;
      const updated = [...prev];
      updated[lastIdx] = { ...updated[lastIdx], content: llmStore.streamBuffer };
      return updated;
    });
  }, [llmStore.streamBuffer, llmStore.isStreaming, llmStore.streamingConvId]);

  /* Stream: finalize after done */
  useEffect(() => {
    if (!llmStore.isStreaming && llmStore.streamingConvId && llmStore.streamBuffer) {
      const convId = llmStore.streamingConvId;
      const currentMsgs = messagesRef.current;
      if (currentMsgs.length > 0 && currentMsgs[currentMsgs.length - 1].role === 'assistant') {
        const finalConv: Conversation = {
          id: convId,
          name: currentConvRef.current?.name || '',
          isTemporary: false,
          messages: currentMsgs,
          updatedAt: nowISO(),
        };
        invoke('llm_save_conversation', {
          accountId: accountIdRef.current,
          conversation: finalConv,
        }).catch(() => {});
        setCurrentConv(finalConv);
        loadAllListsRef.current();
      }
      llmStore.reset();
      setIsSending(false);
    }
  }, [llmStore, llmStore.isStreaming, llmStore.streamingConvId, llmStore.streamBuffer]);

  /* Stream: error handling */
  useEffect(() => {
    if (llmStore.streamError) {
      const errMsg = llmStore.streamError;
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
      useLlmStore.getState().reset();
      setIsSending(false);
    }
  }, [llmStore, llmStore.streamError, t]);

  /* Send message */
  const sendMessage = useCallback(async () => {
    const text = input.trim();
    if (!text || !activeProvider || !accountId) return;

    const ts = nowISO();
    const userMsg: ChatMsg = { role: 'user', content: text, createdAt: ts };
    const updatedMessages = [...messages, userMsg];
    setMessages(updatedMessages);
    setInput('');
    setIsSending(true);

    const isFirstMsg = messages.length === 0;
    const convName = isFirstMsg ? text.slice(0, 30) : currentConv?.name || '';
    const convId = currentConvId || generateId();

    if (isFirstMsg || currentConv?.isTemporary) {
      const partialConv: Conversation = {
        id: convId,
        name: convName,
        isTemporary: false,
        messages: updatedMessages,
        updatedAt: nowISO(),
      };
      try {
        await invoke('llm_save_conversation', { accountId, conversation: partialConv });
        setCurrentConvId(convId);
        setCurrentConv(partialConv);
        loadAllLists();
      } catch {
        /* continue */
      }
    }

    const assistantMsg: ChatMsg = { role: 'assistant', content: '', createdAt: nowISO() };
    const streamingMessages = [...updatedMessages, assistantMsg];
    setMessages(streamingMessages);

    try {
      const apiKey = await invoke<string>('llm_get_api_key', {
        accountId,
        providerId: activeProvider.id,
      });

      let allMessages: Array<{ role: string; content: string }> = [];
      if (includeSystemPrompt) {
        const systemPrompt = buildSystemPrompt();
        const chunks = await searchGuideChunks(text, i18n.language || 'zh-CN');
        const docPrompt = formatChunksAsSystemMessage(chunks);
        allMessages = buildMessagesWithSystemPromptAndChunks(
          text,
          updatedMessages,
          systemPrompt,
          docPrompt,
        );
      } else {
        allMessages = updatedMessages.map((m) => ({ role: m.role, content: m.content }));
        allMessages.push({ role: 'user', content: text });
      }

      const messagesPayload = allMessages.map((m) => ({ role: m.role, content: m.content }));

      markConversationPending(convId);
      llmStore.startStream(convId);

      invoke('llm_send_message_stream', {
        accountId,
        conversationId: convId,
        baseUrl: activeProvider.baseUrl,
        apiKey,
        model: activeProvider.model,
        apiType: activeProvider.apiType,
        messages: messagesPayload,
      }).catch((err) => {
        llmStore.onChunk({
          conversationId: convId,
          chunk: '',
          isDone: false,
          error: String(err),
        });
      });
    } catch (e) {
      const errMsg = typeof e === 'string' ? e : e instanceof Error ? e.message : String(e);
      const errorAssistantMsg: ChatMsg = {
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
        await invoke('llm_save_conversation', { accountId, conversation: errorConv });
        setCurrentConv(errorConv);
        loadAllLists();
      } catch {
        /* best effort */
      }
      setIsSending(false);
    }
  }, [
    input,
    activeProvider,
    accountId,
    messages,
    currentConv,
    currentConvId,
    includeSystemPrompt,
    llmStore,
    loadAllLists,
    t,
  ]);

  const handleRename = useCallback(
    async (convId: string, newName: string) => {
      if (!accountId || !newName.trim()) return;
      await invoke('llm_rename_conversation', {
        accountId,
        conversationId: convId,
        name: newName.trim(),
      });
      setConversations((prev) =>
        prev.map((c) => (c.id === convId ? { ...c, name: newName.trim() } : c)),
      );
      if (currentConv?.id === convId)
        setCurrentConv((prev) => (prev ? { ...prev, name: newName.trim() } : prev));
    },
    [accountId, currentConv],
  );

  const handleSoftDelete = useCallback(
    async (convId: string) => {
      if (!accountId) return;
      await invoke('llm_soft_delete_conversation', { accountId, conversationId: convId });
      setConversations((prev) => prev.filter((c) => c.id !== convId));
      if (currentConvId === convId) handleNewConversation();
      loadAllLists();
    },
    [accountId, currentConvId, handleNewConversation, loadAllLists],
  );

  const handleRestore = useCallback(
    async (convId: string) => {
      if (!accountId) return;
      await invoke('llm_restore_conversation', { accountId, conversationId: convId });
      loadAllLists();
    },
    [accountId, loadAllLists],
  );

  const handlePermanentDelete = useCallback(
    async (convId: string) => {
      if (!accountId) return;
      await invoke('llm_permanent_delete', { accountId, conversationId: convId });
      setTrashList((prev) => prev.filter((c) => c.id !== convId));
      setConfirmPermanentDelete(null);
      setFloatingConv((prev) => (prev?.id === convId ? null : prev));
    },
    [accountId],
  );

  const handleViewTrashConv = useCallback(
    async (convId: string) => {
      if (!accountId) return;
      try {
        const conv = await invoke<Conversation>('llm_get_conversation', {
          accountId,
          conversationId: convId,
        });
        setFloatingConv((prev) => (prev?.id === convId ? null : conv));
      } catch {
        /* ignore */
      }
    },
    [accountId],
  );

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
    includeSystemPrompt,
    loading,
    conversations,
    trashList,
    showTrash,
    currentConvId,
    currentConv,
    messages,
    input,
    isSending,
    isOnline,
    checkingOnline,
    copiedIndex,
    floatingConv,
    confirmPermanentDelete,
    isLocal,
    setInput,
    setShowTrash,
    setConfirmPermanentDelete,
    setFloatingConv,
    sendMessage,
    handleNewConversation,
    loadConversation,
    handleRename,
    handleSoftDelete,
    handleRestore,
    handlePermanentDelete,
    handleViewTrashConv,
    handleCopy,
    checkOnline,
  };
}
