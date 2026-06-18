import React, { useState, useRef, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { MessageSquare, Plus, History, ArrowUpRight, X } from 'lucide-react';
import { markConversationPending, setQuickChatOpen } from '@/lib/notification';
import {
  buildSystemPrompt,
  buildMessagesWithSystemPromptAndChunks,
} from '@/lib/llm/systemPromptBuilder';
import { searchGuideChunks, formatChunksAsSystemMessage } from '@/lib/llm/guideService';
import i18n from '@/lib/i18n';
import { COPY_FEEDBACK_DURATION_MS } from '@/lib/constants';
import { useAuthStore } from '@/stores/authStore';
import { LoadingPlaceholder } from '@/components/ui/LoadingPlaceholder';
import { ChatMessageList }  from '@/components/llm/ChatMessageList';
import { ChatInputBar }  from '@/components/llm/ChatInputBar';
import { ConversationHistory }  from '@/components/llm/ConversationHistory';
import { UnconfiguredHint } from '@/components/llm/UnconfiguredHint';

// ── AI Quick Chat types & helpers ───────────────────────────────────────────
interface ChatMsg {
  role: string;
  content: string;
  createdAt: string;
}
interface ConversationSummary {
  id: string;
  name: string;
  updatedAt: string;
  messageCount: number;
  deletedAt?: string;
}
interface Conversation {
  id: string;
  name: string;
  isTemporary: boolean;
  messages: ChatMsg[];
  updatedAt: string;
  deletedAt?: string;
}
interface LlmStreamPayload {
  conversationId: string;
  chunk: string;
  isDone: boolean;
  error?: string;
}

function nowISO(): string {
  return new Date().toISOString();
}
function isOllama(baseUrl: string): boolean {
  return baseUrl.toLowerCase().includes('localhost') || baseUrl.toLowerCase().includes('127.0.0.1');
}
function generateId(): string {
  return 'conv_' + crypto.randomUUID();
}

// =============================================================================
// AiQuickChatPopover — quick AI chat floating card beside sidebar
// =============================================================================

export function AiQuickChatPopover({
  position,
  onClose,
  placement = 'left',
}: {
  position: { top: number } | null;
  onClose: () => void;
  placement?: 'left' | 'right' | 'bottom' | 'top';
}) {
  const { t } = useTranslation(['settings', 'common']);
  const navigate = useNavigate();
  const accountId = useAuthStore((s) => s.currentAccount?.id);

  const [loading, setLoading] = useState(true);
  const [isConfigured, setIsConfigured] = useState(false);
  const [isAiEnabled, setIsAiEnabled] = useState(false);
  const [activeProvider, setActiveProvider] = useState<{
    id: string;
    name: string;
    model: string;
    baseUrl: string;
    apiType: string;
  } | null>(null);
  const [isOnline, setIsOnline] = useState<boolean | null>(null);
  const [checkingOnline, setCheckingOnline] = useState(false);

  useEffect(() => {
    setQuickChatOpen(true);
    return () => setQuickChatOpen(false);
  }, []);

  const [messages, setMessages] = useState<ChatMsg[]>([]);
  const [input, setInput] = useState('');
  const [isSending, setIsSending] = useState(false);
  const [streamBuffer, setStreamBuffer] = useState('');
  const [currentConvId, setCurrentConvId] = useState<string | null>(null);
  const [conversations, setConversations] = useState<ConversationSummary[]>([]);
  const [showHistory, setShowHistory] = useState(false);
  const [copiedIndex, setCopiedIndex] = useState<number | null>(null);
  const copyTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const outsideClickTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const chatEndRef = useRef<HTMLDivElement>(null);
  const scrollContainerRef = useRef<HTMLDivElement>(null);
  const cardRef = useRef<HTMLDivElement>(null);
  const historyRef = useRef<HTMLDivElement>(null);
  const unlistenRef = useRef<UnlistenFn | null>(null);
  const messagesRef = useRef<ChatMsg[]>([]);
  const streamBufferRef = useRef('');
  const currentConvIdRef = useRef<string | null>(null);
  const accountIdRef = useRef(accountId);

  messagesRef.current = messages;
  streamBufferRef.current = streamBuffer;
  currentConvIdRef.current = currentConvId;

  useEffect(() => {
    accountIdRef.current = accountId;
  }, [accountId]);

  useEffect(() => {
    return () => {
      if (copyTimeoutRef.current) clearTimeout(copyTimeoutRef.current);
    };
  }, []);

  const quickChatStorageKey = accountId ? `solosoul_quick_chat_conv_${accountId}` : null;

  // Load config & restore previous conversation
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
        }>('llm_get_config', { accountId });
        setIsAiEnabled(cfg.aiFeaturesEnabled?.chat ?? false);
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
        const list = await invoke<ConversationSummary[]>('llm_list_conversations', { accountId });
        setConversations(list);
        const savedConvId = quickChatStorageKey ? localStorage.getItem(quickChatStorageKey) : null;
        if (savedConvId) {
          try {
            const conv = await invoke<Conversation>('llm_get_conversation', {
              accountId,
              conversationId: savedConvId,
            });
            setCurrentConvId(conv.id);
            setMessages(conv.messages);
          } catch {
            localStorage.removeItem(quickChatStorageKey!);
          }
        }
      } catch {
        setIsConfigured(false);
      } finally {
        setLoading(false);
      }
    })();
  }, [accountId, quickChatStorageKey]);

  // Check online
  useEffect(() => {
    if (!activeProvider || !accountId || !isConfigured) return;
    (async () => {
      setCheckingOnline(true);
      try {
        let key = '';
        try {
          key = await invoke<string>('llm_get_api_key', {
            accountId,
            providerId: activeProvider.id,
          });
        } catch {
          /* ignore */
        }
        const online = await invoke<boolean>('llm_check_connection', {
          baseUrl: activeProvider.baseUrl,
          apiKey: key,
          model: activeProvider.model,
          apiType: activeProvider.apiType,
        });
        setIsOnline(online);
      } catch {
        setIsOnline(false);
      } finally {
        setCheckingOnline(false);
      }
    })();
  }, [activeProvider, accountId, isConfigured]);

  // Subscribe to stream
  useEffect(() => {
    if (!isConfigured) return;
    listen<LlmStreamPayload>('llm-stream-chunk', (event) => {
      const payload = event.payload;
      const msgs = messagesRef.current;
      const convId = currentConvIdRef.current;
      const accId = accountIdRef.current;

      if (payload.error) {
        setIsSending(false);
        setMessages((prev) => {
          if (prev.length === 0) return prev;
          const lastIdx = prev.length - 1;
          if (prev[lastIdx].role !== 'assistant') return prev;
          const updated = [...prev];
          updated[lastIdx] = {
            ...updated[lastIdx],
            content: `${t('settings:ai_chat_error_prefix')}: ${payload.error}`,
          };
          return updated;
        });
        return;
      }
      if (payload.isDone) {
        setIsSending(false);
        if (convId && accId) {
          const finalMsgs = msgs.map((m, i) =>
            i === msgs.length - 1 && m.role === 'assistant'
              ? { ...m, content: streamBufferRef.current }
              : m,
          );
          const firstUser = finalMsgs.find((m) => m.role === 'user');
          const finalConv = {
            id: convId,
            name: firstUser ? firstUser.content.slice(0, 30) : '',
            isTemporary: false,
            messages: finalMsgs,
            updatedAt: nowISO(),
          };
          invoke('llm_save_conversation', { accountId: accId, conversation: finalConv })
            .then(() => {
              if (quickChatStorageKey) localStorage.setItem(quickChatStorageKey, convId);
              invoke<ConversationSummary[]>('llm_list_conversations', { accountId: accId })
                .then((list) => setConversations(list))
                .catch(() => {});
            })
            .catch(() => {});
        }
        return;
      }
      setStreamBuffer((prev) => prev + payload.chunk);
      setMessages((prev) => {
        if (prev.length === 0) return prev;
        const lastIdx = prev.length - 1;
        if (prev[lastIdx].role !== 'assistant') return prev;
        const updated = [...prev];
        updated[lastIdx] = {
          ...updated[lastIdx],
          content: streamBufferRef.current + payload.chunk,
        };
        return updated;
      });
    })
      .then((fn) => {
        unlistenRef.current = fn;
      })
      .catch(() => {});
    return () => {
      unlistenRef.current?.();
    };
  }, [isConfigured, t, quickChatStorageKey]);

  // Close history dropdown on outside click within card
  useEffect(() => {
    if (!showHistory) return;
    const handler = (e: MouseEvent) => {
      if (historyRef.current && !historyRef.current.contains(e.target as Node)) {
        setShowHistory(false);
      }
    };
    document.addEventListener('mousedown', handler);
    return () => document.removeEventListener('mousedown', handler);
  }, [showHistory]);

  // Close on outside click (ignore AI sidebar button clicks)
  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (cardRef.current && !cardRef.current.contains(e.target as Node)) {
        if ((e.target as HTMLElement).closest('[data-ai-button]')) return;
        onClose();
      }
    };
    outsideClickTimeoutRef.current = setTimeout(
      () => document.addEventListener('mousedown', handler),
      1
    );
    return () => {
      if (outsideClickTimeoutRef.current) {
        clearTimeout(outsideClickTimeoutRef.current);
      }
      document.removeEventListener('mousedown', handler);
    };
  }, [onClose]);

  // Close on Escape
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    document.addEventListener('keydown', handler);
    return () => document.removeEventListener('keydown', handler);
  }, [onClose]);

  const sendMessage = async () => {
    const text = input.trim();
    if (!text || !activeProvider || !accountId || isOnline === false) return;

    const ts = nowISO();
    const userMsg: ChatMsg = { role: 'user', content: text, createdAt: ts };
    const updatedMessages = [...messages, userMsg];
    setMessages(updatedMessages);
    setInput('');
    setIsSending(true);
    setStreamBuffer('');

    const convId = currentConvId || generateId();
    setCurrentConvId(convId);
    if (quickChatStorageKey) localStorage.setItem(quickChatStorageKey, convId);

    const firstUser = updatedMessages.find((m) => m.role === 'user');
    const convName = firstUser ? firstUser.content.slice(0, 30) : '';
    const partialConv = {
      id: convId,
      name: convName,
      isTemporary: false,
      messages: updatedMessages,
      updatedAt: nowISO(),
    };
    invoke('llm_save_conversation', { accountId, conversation: partialConv }).catch(() => {});

    const assistantMsg: ChatMsg = { role: 'assistant', content: '', createdAt: nowISO() };
    setMessages((prev) => [...prev, assistantMsg]);

    try {
      const apiKey = await invoke<string>('llm_get_api_key', {
        accountId,
        providerId: activeProvider.id,
      });

      const systemPrompt = buildSystemPrompt();
      const chunks = await searchGuideChunks(text, i18n.language || 'zh-CN');
      const docPrompt = formatChunksAsSystemMessage(chunks);
      const allMessages = buildMessagesWithSystemPromptAndChunks(
        text,
        updatedMessages,
        systemPrompt,
        docPrompt,
      );

      markConversationPending(convId);
      invoke('llm_send_message_stream', {
        accountId,
        conversationId: convId,
        baseUrl: activeProvider.baseUrl,
        apiKey,
        model: activeProvider.model,
        apiType: activeProvider.apiType,
        messages: allMessages,
      }).catch((err) => {
        setIsSending(false);
        setMessages((prev) => {
          if (prev.length === 0) return prev;
          const lastIdx = prev.length - 1;
          if (prev[lastIdx].role !== 'assistant') return prev;
          const updated = [...prev];
          updated[lastIdx] = {
            ...updated[lastIdx],
            content: `${t('settings:ai_chat_error_prefix')}: ${String(err)}`,
          };
          return updated;
        });
      });
    } catch (e) {
      const errMsg = typeof e === 'string' ? e : e instanceof Error ? e.message : String(e);
      setIsSending(false);
      setMessages((prev) => {
        if (prev.length === 0) return prev;
        const lastIdx = prev.length - 1;
        if (prev[lastIdx].role !== 'assistant') return prev;
        const updated = [...prev];
        updated[lastIdx] = {
          ...updated[lastIdx],
          content: `${t('settings:ai_chat_error_prefix')}: ${errMsg}`,
        };
        return updated;
      });
    }
  };

  const handleNewConversation = () => {
    setMessages([]);
    setInput('');
    setIsSending(false);
    setStreamBuffer('');
    setCurrentConvId(null);
    if (quickChatStorageKey) localStorage.removeItem(quickChatStorageKey);
  };

  const loadConversation = async (convId: string) => {
    if (!accountId) return;
    try {
      const conv = await invoke<Conversation>('llm_get_conversation', {
        accountId,
        conversationId: convId,
      });
      setCurrentConvId(conv.id);
      setMessages(conv.messages);
      if (quickChatStorageKey) localStorage.setItem(quickChatStorageKey, conv.id);
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
      /* ignore */
    }
  };

  const isLocal = activeProvider ? isOllama(activeProvider.baseUrl) : false;

  const isFloating = placement === 'bottom' || placement === 'top';
  const isRight = placement === 'right';

  return (
    <div
      ref={cardRef}
      data-ai-quick-chat="open"
      style={{
        position: 'fixed',
        ...(isFloating
          ? { right: 12, left: 'auto' }
          : isRight
            ? { right: 52, left: 'auto' }
            : { left: 52, right: 'auto' }),
        top: position?.top ?? 100,
        width: 380,
        height: 520,
        zIndex: 200,
        background: 'var(--bg-elevated)',
        borderRadius: 14,
        boxShadow: 'var(--shadow-lg), 0 0 0 1px var(--border-subtle)',
        border: '1px solid var(--border-subtle)',
        display: 'flex',
        flexDirection: 'column',
        overflow: 'hidden',
        animation: 'quickChatSlideIn 0.18s cubic-bezier(0.34, 1.56, 0.64, 1) both',
      }}
    >
      {/* Header */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          padding: '10px 12px',
          borderBottom: '1px solid var(--border-subtle)',
          flexShrink: 0,
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
          <MessageSquare size={16} style={{ color: 'var(--accent-primary)' }} />
          <span style={{ fontSize: 13, fontWeight: 600, color: 'var(--text-primary)' }}>
            {t('settings:ai_quick_chat_title')}
          </span>
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
          {isConfigured && isAiEnabled && (
            <>
              <button
                onClick={handleNewConversation}
                title={t('settings:ai_new_conv')}
                style={{
                  padding: 4,
                  borderRadius: 6,
                  border: 'none',
                  background: 'transparent',
                  cursor: 'pointer',
                  color: 'var(--text-secondary)',
                }}
              >
                <Plus size={14} />
              </button>
              <button
                onClick={() => setShowHistory((prev) => !prev)}
                title={t('settings:ai_history')}
                style={{
                  padding: 4,
                  borderRadius: 6,
                  border: 'none',
                  background: 'transparent',
                  cursor: 'pointer',
                  color: showHistory ? 'var(--accent-primary)' : 'var(--text-secondary)',
                }}
              >
                <History size={14} />
              </button>
            </>
          )}
          <button
            onClick={() => {
              onClose();
              navigate('/llm-chat');
            }}
            title={t('settings:ai_quick_chat_go_full')}
            style={{
              padding: 4,
              borderRadius: 6,
              border: 'none',
              background: 'transparent',
              cursor: 'pointer',
              color: 'var(--text-secondary)',
            }}
          >
            <ArrowUpRight size={14} />
          </button>
          <button
            onClick={onClose}
            title={t('common:close')}
            style={{
              padding: 4,
              borderRadius: 6,
              border: 'none',
              background: 'transparent',
              cursor: 'pointer',
              color: 'var(--text-tertiary)',
            }}
          >
            <X size={14} />
          </button>
        </div>
      </div>

      {/* History dropdown */}
      {showHistory && (
        <div
          ref={historyRef}
          style={{
            position: 'absolute',
            top: 44,
            left: 8,
            right: 8,
            maxHeight: 220,
            background: 'var(--bg-elevated)',
            borderRadius: 10,
            border: '1px solid var(--border-subtle)',
            boxShadow: 'var(--shadow-lg)',
            zIndex: 10,
            overflowY: 'auto',
            padding: '6px 0',
          }}
        >
          <ConversationHistory
            conversations={conversations}
            currentConvId={currentConvId}
            onSelect={(id) => {
              loadConversation(id);
              setShowHistory(false);
            }}
          />
        </div>
      )}

      {/* Body */}
      {loading ? (
        <LoadingPlaceholder variant="elevated" />
      ) : !isAiEnabled || !isConfigured ? (
        <UnconfiguredHint onClose={onClose} />
      ) : (
        <>
          <ChatMessageList
            messages={messages}
            isSending={isSending}
            copiedIndex={copiedIndex}
            onCopy={handleCopy}
            errorPrefix={t('settings:ai_chat_error_prefix')}
            activeProviderName={activeProvider?.name ?? ''}
            scrollContainerRef={scrollContainerRef}
            chatEndRef={chatEndRef}
          />
          <ChatInputBar
            input={input}
            onInputChange={setInput}
            isSending={isSending}
            onSend={sendMessage}
            activeProvider={activeProvider ? { name: activeProvider.name, model: activeProvider.model, baseUrl: activeProvider.baseUrl } : null}
            checkingOnline={checkingOnline}
            isOnline={isOnline}
            isLocal={isLocal}
          />
        </>
      )}

      <style>{`
        @keyframes quickChatSlideIn {
          from { opacity: 0; transform: translateX(-8px) scale(0.97); }
          to { opacity: 1; transform: translateX(0) scale(1); }
        }
        .quick-chat-markdown pre {
          background: var(--bg-toolbar); border: 1px solid var(--border-subtle); border-radius: 6px;
          padding: 8px 10px; overflow-x: auto; font-size: 12px; line-height: 1.45; margin: 6px 0;
        }
        .quick-chat-markdown code { font-family: 'Menlo', 'Monaco', 'Courier New', monospace; font-size: 12px; }
        .quick-chat-markdown p > code, .quick-chat-markdown li > code { background: rgba(128,128,128,0.1); padding: 1px 3px; border-radius: 3px; }
        .quick-chat-markdown p { margin: 0 0 4px; }
        .quick-chat-markdown p:last-child { margin-bottom: 0; }
        .quick-chat-markdown ul, .quick-chat-markdown ol { margin: 3px 0; padding-left: 16px; }
        .quick-chat-markdown blockquote { border-left: 2px solid var(--accent-primary); margin: 4px 0; padding-left: 8px; color: var(--text-secondary); }
        .typing-animation .dot:nth-child(1) { animation: blink 1.4s infinite 0s; }
        .typing-animation .dot:nth-child(2) { animation: blink 1.4s infinite 0.2s; }
        .typing-animation .dot:nth-child(3) { animation: blink 1.4s infinite 0.4s; }
        @keyframes blink { 0%, 80%, 100% { opacity: 0.3; } 40% { opacity: 1; } }
      `}</style>
    </div>
  );
}
