import { useState, useRef, useEffect, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { AppShell } from '@/components/layout/AppShell';
import { Button } from '@/components/ui/Button';
import { LoadingPlaceholder } from '@/components/ui/LoadingPlaceholder';
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
import {
  MessageSquare,
  Settings,
  BarChart3,
} from 'lucide-react';
import { markConversationPending, setAiPageOpen } from '@/lib/notification';
import type { ChatMsg } from './ChatMessageBubble';
import { ConversationSidebar } from '@/components/llm/ConversationSidebar';
import { MessageArea } from '@/components/llm/MessageArea';
import { TrashConversationCard } from '@/components/llm/TrashConversationCard';

interface Conversation {
  id: string;
  name: string;
  isTemporary: boolean;
  messages: ChatMsg[];
  updatedAt: string;
  deletedAt?: string;
}

interface ConversationSummary {
  id: string;
  name: string;
  updatedAt: string;
  messageCount: number;
  deletedAt?: string;
}

function isOllama(baseUrl: string): boolean {
  return baseUrl.toLowerCase().includes('localhost') || baseUrl.toLowerCase().includes('127.0.0.1');
}

function nowISO(): string {
  return new Date().toISOString();
}

function generateId(): string {
  return 'conv_' + crypto.randomUUID();
}

export function LlmChatPage() {
  const navigate = useNavigate();
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

  // Load providers and config
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

  // Load conversations & trash
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

  // Check online status
  const checkOnline = useCallback(() => {
    if (!activeProvider || !accountId) return;
    const { isCancelled } = makeCancellable();
    setCheckingOnline(true);
    (async () => {
      try {
        let key = '';
        try {
          key = await invoke<string>('llm_get_api_key', { accountId, providerId: activeProvider.id });
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

  // Scroll to bottom
  const lastMessageKey = messages.length > 0 ? messages[messages.length - 1].createdAt : null;
  useEffect(() => {
    const el = document.querySelector('[data-chat-end]');
    el?.scrollIntoView({ behavior: 'auto' });
  }, [lastMessageKey]);

  useEffect(() => {
    return () => {
      if (copyTimeoutRef.current) clearTimeout(copyTimeoutRef.current);
    };
  }, []);

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

  const handleNewConversation = () => {
    const id = generateId();
    setCurrentConvId(id);
    setCurrentConv({ id, name: '', isTemporary: true, messages: [], updatedAt: nowISO() });
    setMessages([]);
  };

  const llmStore = useLlmStore();

  // Stream listeners
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

  const sendMessage = async () => {
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
  };

  const handleRename = async (convId: string, newName: string) => {
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
  };

  const handleSoftDelete = async (convId: string) => {
    if (!accountId) return;
    await invoke('llm_soft_delete_conversation', { accountId, conversationId: convId });
    setConversations((prev) => prev.filter((c) => c.id !== convId));
    if (currentConvId === convId) handleNewConversation();
    loadAllLists();
  };

  const handleRestore = async (convId: string) => {
    if (!accountId) return;
    await invoke('llm_restore_conversation', { accountId, conversationId: convId });
    loadAllLists();
  };

  const handlePermanentDelete = async (convId: string) => {
    if (!accountId) return;
    await invoke('llm_permanent_delete', { accountId, conversationId: convId });
    setTrashList((prev) => prev.filter((c) => c.id !== convId));
    setConfirmPermanentDelete(null);
    setFloatingConv((prev) => (prev?.id === convId ? null : prev));
  };

  const handleViewTrashConv = async (convId: string) => {
    if (!accountId) return;
    try {
      const conv = await invoke<Conversation>('llm_get_conversation', {
        accountId,
        conversationId: convId,
      });
      setFloatingConv(floatingConv?.id === convId ? null : conv);
    } catch {
      /* ignore */
    }
  };

  const handleCopy = async (content: string, index: number) => {
    try {
      await navigator.clipboard.writeText(content);
      setCopiedIndex(index);
      if (copyTimeoutRef.current) clearTimeout(copyTimeoutRef.current);
      copyTimeoutRef.current = setTimeout(() => setCopiedIndex(null), COPY_FEEDBACK_DURATION_MS);
    } catch {
      /* fallback */
    }
  };

  const isLocal = activeProvider ? isOllama(activeProvider.baseUrl) : false;

  if (loading) {
    return (
      <AppShell title={t('settings:ai_chat')} onBack={() => navigate('/home')}>
        <div
          style={{
            position: 'fixed',
            top: 56,
            left: 48,
            right: 0,
            bottom: 0,
            display: 'flex',
            overflow: 'hidden',
          }}
        >
          <div
            style={{
              width: 220,
              minWidth: 180,
              maxWidth: 360,
              borderRight: '1px solid var(--border-subtle)',
              background: 'var(--bg-toolbar)',
            }}
          />
          <div
            style={{
              flex: 1,
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              minWidth: 0,
            }}
          >
            <LoadingPlaceholder variant="base" />
          </div>
        </div>
      </AppShell>
    );
  }

  if (!isAiEnabled || !isConfigured) {
    return (
      <AppShell
        title={t('settings:ai_chat')}
        onBack={() => navigate('/home')}
        actions={
          <Button
            variant="secondary"
            size="sm"
            onClick={() => navigate('/settings/llm', { state: { from: '/llm-chat' } })}
          >
            <Settings size={14} style={{ marginRight: 4 }} /> {t('settings:ai_chat_configure')}
          </Button>
        }
      >
        <div style={{ maxWidth: 600, margin: '0 auto', textAlign: 'center', padding: '48px 24px' }}>
          <MessageSquare
            size={48}
            style={{ marginBottom: 16, opacity: 0.3, color: 'var(--text-tertiary)' }}
          />
          <h2 style={{ fontSize: 18, fontWeight: 600, margin: '0 0 8px' }}>
            {t('settings:ai_chat')}
          </h2>
          <p style={{ fontSize: 14, color: 'var(--text-secondary)', marginBottom: 16 }}>
            {t('settings:ai_chat_disabled')}
          </p>
          <Button onClick={() => navigate('/settings/llm', { state: { from: '/llm-chat' } })}>
            {t('settings:ai_chat_configure')}
          </Button>
        </div>
      </AppShell>
    );
  }

  return (
    <AppShell
      title={t('settings:ai_chat')}
      onBack={() => navigate('/home')}
      actions={
        <div style={{ display: 'flex', gap: 8 }}>
          <span className="tooltip-btn" data-tooltip={t('settings:llm_stats_title')}>
            <button
              onClick={() => navigate('/settings/llm/stats', { state: { from: '/llm-chat' } })}
              style={{
                padding: 8,
                borderRadius: 8,
                border: '1px solid var(--border-subtle)',
                background: 'transparent',
                cursor: 'pointer',
                color: 'var(--text-secondary)',
              }}
            >
              <BarChart3 size={16} />
            </button>
          </span>
          <span className="tooltip-btn" data-tooltip={t('settings:llm_config')}>
            <button
              onClick={() => navigate('/settings/llm', { state: { from: '/llm-chat' } })}
              style={{
                padding: 8,
                borderRadius: 8,
                border: '1px solid var(--border-subtle)',
                background: 'transparent',
                cursor: 'pointer',
                color: 'var(--text-secondary)',
              }}
            >
              <Settings size={16} />
            </button>
          </span>
        </div>
      }
    >
      <div
        style={{
          position: 'fixed',
          top: 56,
          left: 48,
          right: 0,
          bottom: 0,
          display: 'flex',
          overflow: 'hidden',
        }}
      >
        <ConversationSidebar
          conversations={conversations}
          trashList={trashList}
          currentConvId={currentConvId}
          showTrash={showTrash}
          onNewConversation={handleNewConversation}
          onLoadConversation={loadConversation}
          onSoftDelete={handleSoftDelete}
          onRename={handleRename}
          onToggleTrash={() => setShowTrash(!showTrash)}
          onRestore={handleRestore}
          confirmPermanentDeleteId={confirmPermanentDelete}
          onRequestPermanentDelete={(id) => {
            if (confirmPermanentDelete === id) {
              handlePermanentDelete(id);
            } else {
              setConfirmPermanentDelete(id);
            }
          }}
          onViewTrashConv={handleViewTrashConv}
        />

        <MessageArea
          messages={messages}
          input={input}
          isSending={isSending}
          isOnline={isOnline}
          checkingOnline={checkingOnline}
          activeProvider={activeProvider}
          isLocal={isLocal}
          copiedIndex={copiedIndex}
          onInputChange={setInput}
          onSend={sendMessage}
          onCopy={handleCopy}
          onCheckOnline={checkOnline}
        />

        <TrashConversationCard
          floatingConv={floatingConv}
          copiedIndex={copiedIndex}
          onClose={() => setFloatingConv(null)}
          onCopy={handleCopy}
        />
      </div>

      <style>{`
        .sidebar-action-btn { transition: opacity 0.1s; }
        .sidebar-action-btn:hover { opacity: 1 !important; }
        div[style*="cursor: pointer"]:hover .sidebar-action-btn { opacity: 0.5; }
        .markdown-content pre {
          background: var(--bg-toolbar); border: 1px solid var(--border-subtle); border-radius: 8px;
          padding: 10px 14px; overflow-x: auto; font-size: 13px; line-height: 1.5; margin: 8px 0;
        }
        .markdown-content code { font-family: 'Menlo', 'Monaco', 'Courier New', monospace; font-size: 13px; }
        .markdown-content p > code, .markdown-content li > code { background: rgba(128,128,128,0.1); padding: 1px 4px; border-radius: 3px; }
        .markdown-content p { margin: 0 0 6px; }
        .markdown-content p:last-child { margin-bottom: 0; }
        .markdown-content ul, .markdown-content ol { margin: 4px 0; padding-left: 20px; }
        .markdown-content blockquote { border-left: 3px solid var(--accent-primary); margin: 6px 0; padding-left: 10px; color: var(--text-secondary); }
        .markdown-content table { border-collapse: collapse; margin: 6px 0; font-size: 13px; }
        .markdown-content th, .markdown-content td { border: 1px solid var(--border-subtle); padding: 4px 8px; text-align: left; }
        .markdown-content th { background: var(--bg-toolbar); font-weight: 600; }
        .typing-animation .dot:nth-child(1) { animation: blink 1.4s infinite 0s; }
        .typing-animation .dot:nth-child(2) { animation: blink 1.4s infinite 0.2s; }
        .typing-animation .dot:nth-child(3) { animation: blink 1.4s infinite 0.4s; }
        .tooltip-btn { position: relative; display: inline-flex; }
        .tooltip-btn::after {
          content: attr(data-tooltip);
          position: absolute;
          top: calc(100% + 6px);
          left: 50%;
          transform: translateX(-50%);
          padding: 4px 8px;
          border-radius: 6px;
          background: var(--bg-elevated);
          color: var(--text-secondary);
          border: 1px solid var(--border-subtle);
          font-size: 11px;
          white-space: nowrap;
          opacity: 0;
          pointer-events: none;
          transition: opacity 0.12s ease;
          box-shadow: 0 4px 12px rgba(0,0,0,0.12);
          z-index: 10;
        }
        .tooltip-btn:hover::after { opacity: 1; }
        .conv-item {
          border: 1px solid transparent;
          border-radius: 8px;
          margin: 2px 8px;
          transition: transform 0.15s ease, border-color 0.15s ease, box-shadow 0.15s ease;
        }
        .conv-item:hover {
          transform: translateY(-2px);
          border-color: var(--accent-primary);
          box-shadow: 0 6px 16px rgba(0,0,0,0.08);
        }
        @keyframes blink { 0%, 80%, 100% { opacity: 0.3; } 40% { opacity: 1; } }
      `}</style>
    </AppShell>
  );
}
