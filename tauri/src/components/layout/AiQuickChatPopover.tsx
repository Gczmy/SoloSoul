import React, { useState, useRef, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import {
  MessageSquare,
  Settings,
  Send,
  Plus,
  X,
  ArrowUpRight,
  History,
  Copy,
  Check,
} from 'lucide-react';
import ReactMarkdown from 'react-markdown';
import rehypeHighlight from 'rehype-highlight';
import { markConversationPending, setQuickChatOpen } from '@/lib/notification';
import {
  buildSystemPrompt,
  buildMessagesWithSystemPromptAndChunks,
} from '@/lib/llm/systemPromptBuilder';
import { searchGuideChunks, formatChunksAsSystemMessage } from '@/lib/llm/guideService';
import i18n from '@/lib/i18n';
import { formatRelative, formatTimestamp } from '@/lib/time';
import { COPY_FEEDBACK_DURATION_MS } from '@/lib/constants';
import { useAuthStore } from '@/stores/authStore';
import { LoadingPlaceholder } from '@/components/ui/LoadingPlaceholder';

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
        // Load conversation list
        const list = await invoke<ConversationSummary[]>('llm_list_conversations', { accountId });
        setConversations(list);
        // Restore last open conversation from localStorage
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
              // Refresh conversation list
              invoke<ConversationSummary[]>('llm_list_conversations', { accountId: accId })
                .then((list) => {
                  setConversations(list);
                })
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

  // Scroll to bottom: instant on first mount/load, smooth on subsequent updates
  const hasScrolledRef = useRef(false);
  useEffect(() => {
    const container = scrollContainerRef.current;
    if (!container) return;
    if (!hasScrolledRef.current) {
      // First render (e.g. reopening card) — set scrollTop directly, absolutely no animation
      container.scrollTop = container.scrollHeight;
      hasScrolledRef.current = true;
    } else {
      chatEndRef.current?.scrollIntoView({ behavior: 'smooth', block: 'end' });
    }
  }, [messages]);

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
      0
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

    // Save user message immediately
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

      // Build system prompt and help doc chunks (RAG vector search)
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

  const cardContent = (() => {
    if (loading) {
      return <LoadingPlaceholder variant="elevated" />;
    }

    if (!isAiEnabled || !isConfigured) {
      return (
        <div
          style={{
            flex: 1,
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            justifyContent: 'center',
            padding: 24,
            gap: 12,
          }}
        >
          <MessageSquare size={36} style={{ opacity: 0.3, color: 'var(--text-tertiary)' }} />
          <p
            style={{ fontSize: 13, color: 'var(--text-secondary)', textAlign: 'center', margin: 0 }}
          >
            {t('settings:ai_quick_chat_configure_hint')}
          </p>
          <button
            onClick={() => {
              onClose();
              navigate('/settings/llm');
            }}
            style={{
              padding: '8px 16px',
              borderRadius: 8,
              border: 'none',
              background: 'var(--accent-primary)',
              color: 'white',
              fontSize: 13,
              cursor: 'pointer',
              display: 'flex',
              alignItems: 'center',
              gap: 6,
            }}
          >
            <Settings size={14} /> {t('settings:ai_chat_configure')}
          </button>
        </div>
      );
    }

    return (
      <>
        {/* Messages */}
        <div
          ref={scrollContainerRef}
          style={{ flex: 1, overflowY: 'auto', padding: '8px 0', minHeight: 0 }}
        >
          {messages.length === 0 && (
            <div style={{ textAlign: 'center', padding: '32px 16px' }}>
              <MessageSquare
                size={28}
                style={{ marginBottom: 8, opacity: 0.25, color: 'var(--text-tertiary)' }}
              />
              <p style={{ fontSize: 12, color: 'var(--text-tertiary)', margin: 0 }}>
                {t('settings:ai_chat_start')} · {activeProvider?.name}
              </p>
            </div>
          )}
          {messages.map((msg, i) => (
            <div key={i} style={{ marginBottom: 6 }}>
              <div
                style={{
                  textAlign: 'center',
                  fontSize: 10,
                  color: 'var(--text-tertiary)',
                  padding: '4px 0 1px',
                }}
              >
                {formatTimestamp(msg.createdAt)}
              </div>
              <div
                style={{
                  display: 'flex',
                  justifyContent: msg.role === 'user' ? 'flex-end' : 'flex-start',
                  padding: '0 10px',
                }}
              >
                <div
                  style={{
                    maxWidth: msg.role === 'user' ? '75%' : '90%',
                    padding: '8px 10px',
                    borderRadius: msg.role === 'user' ? '12px 12px 2px 12px' : '12px 12px 12px 2px',
                    background:
                      msg.role === 'user'
                        ? 'var(--accent-primary)'
                        : msg.content.startsWith(t('settings:ai_chat_error_prefix'))
                          ? 'rgba(231,76,60,0.12)'
                          : 'var(--bg-toolbar)',
                    color: msg.role === 'user' ? 'white' : 'var(--text-primary)',
                    fontSize: 13,
                    lineHeight: 1.55,
                  }}
                >
                  {msg.role === 'user' ? (
                    <div style={{ whiteSpace: 'pre-wrap' }}>{msg.content}</div>
                  ) : msg.content.startsWith(t('settings:ai_chat_error_prefix')) ? (
                    <div style={{ color: '#e74c3c', whiteSpace: 'pre-wrap' }}>{msg.content}</div>
                  ) : (
                    <div className="quick-chat-markdown">
                      <ReactMarkdown rehypePlugins={[rehypeHighlight]}>{msg.content}</ReactMarkdown>
                    </div>
                  )}
                </div>
              </div>
              {msg.role !== 'user' && (
                <div style={{ display: 'flex', justifyContent: 'flex-start', padding: '2px 14px' }}>
                  <button
                    onClick={() => handleCopy(msg.content, i)}
                    style={{
                      padding: '2px 6px',
                      borderRadius: 4,
                      border: 'none',
                      background: 'transparent',
                      cursor: 'pointer',
                      fontSize: 11,
                      color: copiedIndex === i ? '#27ae60' : 'var(--text-tertiary)',
                      display: 'flex',
                      alignItems: 'center',
                      gap: 3,
                    }}
                  >
                    {copiedIndex === i ? (
                      <>
                        <Check size={11} /> {t('settings:ai_copied')}
                      </>
                    ) : (
                      <>
                        <Copy size={11} /> {t('settings:ai_copy')}
                      </>
                    )}
                  </button>
                </div>
              )}
            </div>
          ))}
          {isSending && (
            <div
              style={{
                display: 'flex',
                justifyContent: 'flex-start',
                padding: '0 10px',
                marginTop: 4,
              }}
            >
              <div
                style={{
                  padding: '8px 10px',
                  borderRadius: '12px 12px 12px 2px',
                  background: 'var(--bg-toolbar)',
                  fontSize: 13,
                }}
              >
                <span className="typing-animation">
                  <span className="dot">·</span>
                  <span className="dot">·</span>
                  <span className="dot">·</span>
                </span>
              </div>
            </div>
          )}
          <div ref={chatEndRef} />
        </div>

        {/* Input */}
        <div
          style={{
            borderTop: '1px solid var(--border-subtle)',
            padding: '6px 10px 8px',
            flexShrink: 0,
          }}
        >
          <div
            style={{
              fontSize: 10,
              color: 'var(--text-tertiary)',
              marginBottom: 4,
              display: 'flex',
              alignItems: 'center',
              gap: 4,
            }}
          >
            {activeProvider && (
              <>
                <span style={{ color: 'var(--text-secondary)', fontWeight: 500 }}>
                  {activeProvider.name}
                </span>
                <span>·</span>
                <span>{activeProvider.model}</span>
                <span>·</span>
                <span
                  style={{
                    padding: '0 4px',
                    borderRadius: 3,
                    fontSize: 9,
                    background: isLocal ? 'rgba(39,174,96,0.12)' : 'rgba(41,128,185,0.12)',
                    color: isLocal ? '#27ae60' : '#2980b9',
                  }}
                >
                  {isLocal ? t('settings:ai_local') : t('settings:ai_cloud')}
                </span>
                <span>·</span>
                {checkingOnline ? (
                  <span style={{ color: 'var(--text-tertiary)' }}>{t('settings:ai_checking')}</span>
                ) : isOnline === true ? (
                  <span style={{ color: '#27ae60', display: 'flex', alignItems: 'center', gap: 2 }}>
                    <span
                      style={{
                        width: 5,
                        height: 5,
                        borderRadius: '50%',
                        background: '#27ae60',
                        display: 'inline-block',
                      }}
                    />
                    {t('settings:ai_online')}
                  </span>
                ) : isOnline === false ? (
                  <span style={{ color: '#e74c3c', display: 'flex', alignItems: 'center', gap: 2 }}>
                    <span
                      style={{
                        width: 5,
                        height: 5,
                        borderRadius: '50%',
                        background: '#e74c3c',
                        display: 'inline-block',
                      }}
                    />
                    {t('settings:ai_offline')}
                  </span>
                ) : null}
              </>
            )}
          </div>
          <div style={{ display: 'flex', gap: 6, alignItems: 'flex-end' }}>
            <textarea
              value={input}
              onChange={(e) => setInput(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter' && !e.shiftKey) {
                  e.preventDefault();
                  sendMessage();
                }
              }}
              placeholder={t('settings:ai_chat_input_placeholder')}
              disabled={isSending}
              rows={2}
              style={{
                flex: 1,
                padding: '6px 10px',
                fontSize: 13,
                lineHeight: 1.5,
                fontFamily: 'inherit',
                border: '1px solid var(--border-subtle)',
                borderRadius: 8,
                background: 'var(--bg-elevated)',
                color: 'var(--text-primary)',
                resize: 'none',
                outline: 'none',
              }}
            />
            <button
              onClick={sendMessage}
              disabled={isSending || !input.trim() || isOnline === false}
              style={{
                padding: '6px 12px',
                borderRadius: 8,
                border: 'none',
                height: 36,
                background:
                  isSending || !input.trim() || isOnline === false
                    ? 'var(--border-subtle)'
                    : 'var(--accent-primary)',
                color:
                  isSending || !input.trim() || isOnline === false
                    ? 'var(--text-tertiary)'
                    : 'white',
                cursor: 'pointer',
              }}
            >
              {isSending ? (
                <span className="typing-animation">
                  <span className="dot">·</span>
                  <span className="dot">·</span>
                  <span className="dot">·</span>
                </span>
              ) : (
                <Send size={14} />
              )}
            </button>
          </div>
        </div>
      </>
    );
  })();

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
          {conversations.length === 0 ? (
            <p
              style={{
                fontSize: 12,
                color: 'var(--text-tertiary)',
                textAlign: 'center',
                padding: '16px 12px',
                margin: 0,
              }}
            >
              {t('settings:ai_no_convs')}
            </p>
          ) : (
            conversations.map((conv) => (
              <div
                key={conv.id}
                onClick={() => {
                  loadConversation(conv.id);
                  setShowHistory(false);
                }}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 8,
                  padding: '7px 12px',
                  cursor: 'pointer',
                  fontSize: 12,
                  background: currentConvId === conv.id ? 'rgba(91,124,153,0.08)' : 'transparent',
                }}
              >
                <MessageSquare size={12} style={{ color: 'var(--text-tertiary)', flexShrink: 0 }} />
                <div style={{ flex: 1, overflow: 'hidden' }}>
                  <div
                    style={{
                      whiteSpace: 'nowrap',
                      overflow: 'hidden',
                      textOverflow: 'ellipsis',
                      color: 'var(--text-primary)',
                      fontWeight: currentConvId === conv.id ? 500 : 400,
                    }}
                  >
                    {conv.name || t('settings:ai_untitled')}
                  </div>
                  <div style={{ fontSize: 10, color: 'var(--text-tertiary)', marginTop: 1 }}>
                    {formatRelative(conv.updatedAt)} · {conv.messageCount}{' '}
                    {t('settings:ai_messages')}
                  </div>
                </div>
                {currentConvId === conv.id && (
                  <span
                    style={{
                      width: 6,
                      height: 6,
                      borderRadius: '50%',
                      background: 'var(--accent-primary)',
                      flexShrink: 0,
                    }}
                  />
                )}
              </div>
            ))
          )}
        </div>
      )}

      {cardContent}

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
