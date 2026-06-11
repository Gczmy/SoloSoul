import React, { useState, useRef, useCallback, useEffect } from 'react';
import { createPortal } from 'react-dom';
import { useLocation, useNavigate } from 'react-router-dom';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { Plus, MessageSquare, Settings, Send, X, ArrowUpRight, History, Copy, Check } from 'lucide-react';
import type { LucideIcon } from 'lucide-react';
import ReactMarkdown from 'react-markdown';
import rehypeHighlight from 'rehype-highlight';
import { markConversationPending } from '@/lib/notification';
import { buildSystemPrompt, buildMessagesWithSystemPromptAndChunks } from '@/lib/llm/systemPromptBuilder';
import { searchGuideChunks, formatChunksAsSystemMessage } from '@/lib/llm/guideService';
import i18n from '@/lib/i18n';
import styles from './SideNavigation.module.css';
import { useSettingsStore } from '@/stores/settingsStore';
import { useAuthStore } from '@/stores/authStore';
import { useTranslation } from 'react-i18next';
import type { CustomPage } from '@/stores/settingsStore';
import { SearchPopover } from './SearchPopover';
import { NavButton } from './NavButton';
import {
  useActiveCustomPages,
  useBoundNavActions,
  useAiQuickChat,
  SYSTEM_PAGE_KEYS,
  primaryItems,
} from './useNavigationItems';
import {
  PAGE_ICON_MAP,
  CUSTOM_ICON_MAP,
  resolveCustomIcon,
  DEFAULT_CUSTOM_ICON,
  type CustomIconId,
} from '@/lib/pageIcons';

// ── AI Quick Chat types & helpers ───────────────────────────────────────────
interface ChatMsg { role: string; content: string; createdAt: string; }
interface ConversationSummary { id: string; name: string; updatedAt: string; messageCount: number; deletedAt?: string; }
interface Conversation { id: string; name: string; isTemporary: boolean; messages: ChatMsg[]; updatedAt: string; deletedAt?: string; }
interface LlmStreamPayload { conversationId: string; chunk: string; isDone: boolean; error?: string; }

function nowISO(): string {
  return new Date().toISOString();
}
function formatRelative(iso: string): string {
  const diff = Date.now() - new Date(iso).getTime();
  const mins = Math.floor(diff / 60000);
  if (mins < 1) return '刚刚';
  if (mins < 60) return mins + ' 分钟前';
  const hours = Math.floor(mins / 60);
  if (hours < 24) return hours + ' 小时前';
  const days = Math.floor(hours / 24);
  if (days < 30) return days + ' 天前';
  return formatTimestamp(iso).slice(0, 10);
}
function formatTimestamp(iso: string): string {
  const d = new Date(iso);
  return d.getFullYear() + '-' +
    String(d.getMonth() + 1).padStart(2, '0') + '-' +
    String(d.getDate()).padStart(2, '0') + ' ' +
    String(d.getHours()).padStart(2, '0') + ':' +
    String(d.getMinutes()).padStart(2, '0') + ':' +
    String(d.getSeconds()).padStart(2, '0');
}
function isOllama(baseUrl: string): boolean {
  return baseUrl.toLowerCase().includes('localhost') || baseUrl.toLowerCase().includes('127.0.0.1');
}
function generateId(): string {
  return 'conv_' + Date.now() + '_' + Math.random().toString(36).slice(2, 8);
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
  placement?: 'left' | 'bottom' | 'top';
}) {
  const { t } = useTranslation(['settings', 'common']);
  const navigate = useNavigate();
  const accountId = useAuthStore((s) => s.currentAccount?.id);

  const [loading, setLoading] = useState(true);
  const [isConfigured, setIsConfigured] = useState(false);
  const [isAiEnabled, setIsAiEnabled] = useState(false);
  const [activeProvider, setActiveProvider] = useState<{ id: string; name: string; model: string; baseUrl: string; apiType: string } | null>(null);
  const [isOnline, setIsOnline] = useState<boolean | null>(null);
  const [checkingOnline, setCheckingOnline] = useState(false);

  const [messages, setMessages] = useState<ChatMsg[]>([]);
  const [input, setInput] = useState('');
  const [isSending, setIsSending] = useState(false);
  const [streamBuffer, setStreamBuffer] = useState('');
  const [currentConvId, setCurrentConvId] = useState<string | null>(null);
  const [conversations, setConversations] = useState<ConversationSummary[]>([]);
  const [showHistory, setShowHistory] = useState(false);
  const [copiedIndex, setCopiedIndex] = useState<number | null>(null);

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
  accountIdRef.current = accountId;

  const quickChatStorageKey = accountId ? `solosoul_quick_chat_conv_${accountId}` : null;

  // Load config & restore previous conversation
  useEffect(() => {
    if (!accountId) { setLoading(false); return; }
    (async () => {
      try {
        const cfg = await invoke<{ activeProviderId?: string; aiFeaturesEnabled?: { chat: boolean } }>('llm_get_config', { accountId });
        setIsAiEnabled(cfg.aiFeaturesEnabled?.chat ?? false);
        if (!cfg.activeProviderId) { setIsConfigured(false); setLoading(false); return; }
        const providers = await invoke<Array<{ id: string; name: string; model: string; baseUrl: string; apiType: string }>>('llm_get_providers', { accountId });
        const active = providers.find((p) => p.id === cfg.activeProviderId);
        if (active) {
          setActiveProvider({ id: active.id, name: active.name, model: active.model, baseUrl: active.baseUrl, apiType: active.apiType });
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
            const conv = await invoke<Conversation>('llm_get_conversation', { accountId, conversationId: savedConvId });
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
  }, [accountId]);

  // Check online
  useEffect(() => {
    if (!activeProvider || !accountId || !isConfigured) return;
    (async () => {
      setCheckingOnline(true);
      try {
        let key = '';
        try { key = await invoke<string>('llm_get_api_key', { accountId, providerId: activeProvider.id }); } catch { /* ignore */ }
        const online = await invoke<boolean>('llm_check_connection', {
          baseUrl: activeProvider.baseUrl, apiKey: key, model: activeProvider.model, apiType: activeProvider.apiType,
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
          updated[lastIdx] = { ...updated[lastIdx], content: `${t('settings:ai_chat_error_prefix')}: ${payload.error}` };
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
              : m
          );
          const firstUser = finalMsgs.find((m) => m.role === 'user');
          const finalConv = {
            id: convId,
            name: firstUser ? firstUser.content.slice(0, 30) : '',
            isTemporary: false,
            messages: finalMsgs,
            updatedAt: nowISO(),
          };
          invoke('llm_save_conversation', { accountId: accId, conversation: finalConv }).then(() => {
            if (quickChatStorageKey) localStorage.setItem(quickChatStorageKey, convId);
            // Refresh conversation list
            invoke<ConversationSummary[]>('llm_list_conversations', { accountId: accId }).then((list) => {
              setConversations(list);
            }).catch(() => {});
          }).catch(() => {});
        }
        return;
      }
      setStreamBuffer((prev) => prev + payload.chunk);
      setMessages((prev) => {
        if (prev.length === 0) return prev;
        const lastIdx = prev.length - 1;
        if (prev[lastIdx].role !== 'assistant') return prev;
        const updated = [...prev];
        updated[lastIdx] = { ...updated[lastIdx], content: streamBufferRef.current + payload.chunk };
        return updated;
      });
    }).then((fn) => { unlistenRef.current = fn; }).catch(() => {});
    return () => { unlistenRef.current?.(); };
  }, [isConfigured, t]);

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
    setTimeout(() => document.addEventListener('mousedown', handler), 0);
    return () => document.removeEventListener('mousedown', handler);
  }, [onClose]);

  // Close on Escape
  useEffect(() => {
    const handler = (e: KeyboardEvent) => { if (e.key === 'Escape') onClose(); };
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
    const partialConv = { id: convId, name: convName, isTemporary: false, messages: updatedMessages, updatedAt: nowISO() };
    invoke('llm_save_conversation', { accountId, conversation: partialConv }).catch(() => {});

    const assistantMsg: ChatMsg = { role: 'assistant', content: '', createdAt: nowISO() };
    setMessages((prev) => [...prev, assistantMsg]);

    try {
      const apiKey = await invoke<string>('llm_get_api_key', { accountId, providerId: activeProvider.id });

      // Build system prompt and help doc chunks (RAG vector search)
      const systemPrompt = buildSystemPrompt();
      const chunks = await searchGuideChunks(text, i18n.language || 'zh-CN');
      const docPrompt = formatChunksAsSystemMessage(chunks);
      const allMessages = buildMessagesWithSystemPromptAndChunks(text, updatedMessages, systemPrompt, docPrompt);

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
          updated[lastIdx] = { ...updated[lastIdx], content: `${t('settings:ai_chat_error_prefix')}: ${String(err)}` };
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
        updated[lastIdx] = { ...updated[lastIdx], content: `${t('settings:ai_chat_error_prefix')}: ${errMsg}` };
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
      const conv = await invoke<Conversation>('llm_get_conversation', { accountId, conversationId: convId });
      setCurrentConvId(conv.id);
      setMessages(conv.messages);
      if (quickChatStorageKey) localStorage.setItem(quickChatStorageKey, conv.id);
    } catch { /* ignore */ }
  };

  const handleCopy = async (content: string, index: number) => {
    try {
      await navigator.clipboard.writeText(content);
      setCopiedIndex(index);
      setTimeout(() => setCopiedIndex(null), 1500);
    } catch { /* ignore */ }
  };

  const isLocal = activeProvider ? isOllama(activeProvider.baseUrl) : false;

  const cardContent = (() => {
    if (loading) {
      return (
        <div style={{ flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center', color: 'var(--text-secondary)', fontSize: 13 }}>
          {t('common:loading')}
        </div>
      );
    }

    if (!isAiEnabled || !isConfigured) {
      return (
        <div style={{ flex: 1, display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', padding: 24, gap: 12 }}>
          <MessageSquare size={36} style={{ opacity: 0.3, color: 'var(--text-tertiary)' }} />
          <p style={{ fontSize: 13, color: 'var(--text-secondary)', textAlign: 'center', margin: 0 }}>
            {t('settings:ai_quick_chat_configure_hint')}
          </p>
          <button
            onClick={() => { onClose(); navigate('/settings/llm'); }}
            style={{
              padding: '8px 16px', borderRadius: 8, border: 'none', background: 'var(--accent-primary)', color: 'white',
              fontSize: 13, cursor: 'pointer', display: 'flex', alignItems: 'center', gap: 6,
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
        <div ref={scrollContainerRef} style={{ flex: 1, overflowY: 'auto', padding: '8px 0', minHeight: 0 }}>
          {messages.length === 0 && (
            <div style={{ textAlign: 'center', padding: '32px 16px' }}>
              <MessageSquare size={28} style={{ marginBottom: 8, opacity: 0.25, color: 'var(--text-tertiary)' }} />
              <p style={{ fontSize: 12, color: 'var(--text-tertiary)', margin: 0 }}>
                {t('settings:ai_chat_start')} · {activeProvider?.name}
              </p>
            </div>
          )}
          {messages.map((msg, i) => (
            <div key={i} style={{ marginBottom: 6 }}>
              <div style={{ textAlign: 'center', fontSize: 10, color: 'var(--text-tertiary)', padding: '4px 0 1px' }}>
                {formatTimestamp(msg.createdAt)}
              </div>
              <div style={{ display: 'flex', justifyContent: msg.role === 'user' ? 'flex-end' : 'flex-start', padding: '0 10px' }}>
                <div style={{
                  maxWidth: msg.role === 'user' ? '75%' : '90%',
                  padding: '8px 10px',
                  borderRadius: msg.role === 'user' ? '12px 12px 2px 12px' : '12px 12px 12px 2px',
                  background: msg.role === 'user' ? 'var(--accent-primary)' : msg.content.startsWith(t('settings:ai_chat_error_prefix')) ? 'rgba(231,76,60,0.12)' : 'var(--bg-toolbar)',
                  color: msg.role === 'user' ? 'white' : 'var(--text-primary)',
                  fontSize: 13, lineHeight: 1.55,
                }}>
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
                      padding: '2px 6px', borderRadius: 4, border: 'none',
                      background: 'transparent', cursor: 'pointer', fontSize: 11,
                      color: copiedIndex === i ? '#27ae60' : 'var(--text-tertiary)',
                      display: 'flex', alignItems: 'center', gap: 3,
                    }}
                  >
                    {copiedIndex === i ? <><Check size={11} /> {t('settings:ai_copied')}</> : <><Copy size={11} /> {t('settings:ai_copy')}</>}
                  </button>
                </div>
              )}
            </div>
          ))}
          {isSending && (
            <div style={{ display: 'flex', justifyContent: 'flex-start', padding: '0 10px', marginTop: 4 }}>
              <div style={{ padding: '8px 10px', borderRadius: '12px 12px 12px 2px', background: 'var(--bg-toolbar)', fontSize: 13 }}>
                <span className="typing-animation"><span className="dot">·</span><span className="dot">·</span><span className="dot">·</span></span>
              </div>
            </div>
          )}
          <div ref={chatEndRef} />
        </div>

        {/* Input */}
        <div style={{ borderTop: '1px solid var(--border-subtle)', padding: '6px 10px 8px', flexShrink: 0 }}>
          <div style={{ fontSize: 10, color: 'var(--text-tertiary)', marginBottom: 4, display: 'flex', alignItems: 'center', gap: 4 }}>
            {activeProvider && (
              <>
                <span style={{ color: 'var(--text-secondary)', fontWeight: 500 }}>{activeProvider.name}</span>
                <span>·</span>
                <span>{activeProvider.model}</span>
                <span>·</span>
                <span style={{
                  padding: '0 4px', borderRadius: 3, fontSize: 9,
                  background: isLocal ? 'rgba(39,174,96,0.12)' : 'rgba(41,128,185,0.12)',
                  color: isLocal ? '#27ae60' : '#2980b9',
                }}>{isLocal ? t('settings:ai_local') : t('settings:ai_cloud')}</span>
                <span>·</span>
                {checkingOnline ? (
                  <span style={{ color: 'var(--text-tertiary)' }}>{t('settings:ai_checking')}</span>
                ) : isOnline === true ? (
                  <span style={{ color: '#27ae60', display: 'flex', alignItems: 'center', gap: 2 }}>
                    <span style={{ width: 5, height: 5, borderRadius: '50%', background: '#27ae60', display: 'inline-block' }} />
                    {t('settings:ai_online')}
                  </span>
                ) : isOnline === false ? (
                  <span style={{ color: '#e74c3c', display: 'flex', alignItems: 'center', gap: 2 }}>
                    <span style={{ width: 5, height: 5, borderRadius: '50%', background: '#e74c3c', display: 'inline-block' }} />
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
              onKeyDown={(e) => { if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); sendMessage(); } }}
              placeholder={t('settings:ai_chat_input_placeholder')}
              disabled={isSending}
              rows={2}
              style={{
                flex: 1, padding: '6px 10px', fontSize: 13, lineHeight: 1.5, fontFamily: 'inherit',
                border: '1px solid var(--border-subtle)', borderRadius: 8,
                background: 'var(--bg-elevated)', color: 'var(--text-primary)', resize: 'none', outline: 'none',
              }}
            />
            <button
              onClick={sendMessage}
              disabled={isSending || !input.trim() || isOnline === false}
              style={{
                padding: '6px 12px', borderRadius: 8, border: 'none', height: 36,
                background: isSending || !input.trim() || isOnline === false ? 'var(--border-subtle)' : 'var(--accent-primary)',
                color: isSending || !input.trim() || isOnline === false ? 'var(--text-tertiary)' : 'white',
                cursor: 'pointer',
              }}
            >
              {isSending ? <span className="typing-animation"><span className="dot">·</span><span className="dot">·</span><span className="dot">·</span></span> : <Send size={14} />}
            </button>
          </div>
        </div>
      </>
    );
  })();

  const isFloating = placement === 'bottom' || placement === 'top';

  return (
    <div
      ref={cardRef}
      data-ai-quick-chat="open"
      style={{
        position: 'fixed',
        ...(isFloating
          ? { right: 12, left: 'auto' }
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
      <div style={{
        display: 'flex', alignItems: 'center', justifyContent: 'space-between',
        padding: '10px 12px', borderBottom: '1px solid var(--border-subtle)', flexShrink: 0,
      }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
          <MessageSquare size={16} style={{ color: 'var(--accent-primary)' }} />
          <span style={{ fontSize: 13, fontWeight: 600, color: 'var(--text-primary)' }}>{t('settings:ai_quick_chat_title')}</span>
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
          {(isConfigured && isAiEnabled) && (
            <>
              <button
                onClick={handleNewConversation}
                title={t('settings:ai_new_conv')}
                style={{ padding: 4, borderRadius: 6, border: 'none', background: 'transparent', cursor: 'pointer', color: 'var(--text-secondary)' }}
              >
                <Plus size={14} />
              </button>
              <button
                onClick={() => setShowHistory((prev) => !prev)}
                title={t('settings:ai_history')}
                style={{ padding: 4, borderRadius: 6, border: 'none', background: 'transparent', cursor: 'pointer', color: showHistory ? 'var(--accent-primary)' : 'var(--text-secondary)' }}
              >
                <History size={14} />
              </button>
            </>
          )}
          <button
            onClick={() => { onClose(); navigate('/llm-chat'); }}
            title={t('settings:ai_quick_chat_go_full')}
            style={{ padding: 4, borderRadius: 6, border: 'none', background: 'transparent', cursor: 'pointer', color: 'var(--text-secondary)' }}
          >
            <ArrowUpRight size={14} />
          </button>
          <button
            onClick={onClose}
            style={{ padding: 4, borderRadius: 6, border: 'none', background: 'transparent', cursor: 'pointer', color: 'var(--text-tertiary)' }}
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
            position: 'absolute', top: 44, left: 8, right: 8, maxHeight: 220,
            background: 'var(--bg-elevated)', borderRadius: 10, border: '1px solid var(--border-subtle)',
            boxShadow: 'var(--shadow-lg)', zIndex: 10, overflowY: 'auto', padding: '6px 0',
          }}
        >
          {conversations.length === 0 ? (
            <p style={{ fontSize: 12, color: 'var(--text-tertiary)', textAlign: 'center', padding: '16px 12px', margin: 0 }}>
              {t('settings:ai_no_convs')}
            </p>
          ) : (
            conversations.map((conv) => (
              <div
                key={conv.id}
                onClick={() => { loadConversation(conv.id); setShowHistory(false); }}
                style={{
                  display: 'flex', alignItems: 'center', gap: 8,
                  padding: '7px 12px', cursor: 'pointer', fontSize: 12,
                  background: currentConvId === conv.id ? 'rgba(91,124,153,0.08)' : 'transparent',
                }}
              >
                <MessageSquare size={12} style={{ color: 'var(--text-tertiary)', flexShrink: 0 }} />
                <div style={{ flex: 1, overflow: 'hidden' }}>
                  <div style={{ whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis', color: 'var(--text-primary)', fontWeight: currentConvId === conv.id ? 500 : 400 }}>
                    {conv.name || t('settings:ai_untitled')}
                  </div>
                  <div style={{ fontSize: 10, color: 'var(--text-tertiary)', marginTop: 1 }}>
                    {formatRelative(conv.updatedAt)} · {conv.messageCount} {t('settings:ai_messages')}
                  </div>
                </div>
                {currentConvId === conv.id && (
                  <span style={{ width: 6, height: 6, borderRadius: '50%', background: 'var(--accent-primary)', flexShrink: 0 }} />
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

// =============================================================================
// RenameableNavButton — custom page button with double-click rename
// =============================================================================

export function RenameableNavButton({
  page,
  isActive,
  onClick,
  position = 'left',
}: {
  page: CustomPage;
  isActive: boolean;
  onClick: () => void;
  position?: import('./NavButton').NavPosition;
}) {
  const isHorizontal = position === 'top' || position === 'bottom';
  const isBottom = position === 'bottom';
  const { t } = useTranslation(['navigation', 'common']);
  const [isRenaming, setIsRenaming] = useState(false);
  const [renameValue, setRenameValue] = useState(page.name);
  const [renameError, setRenameError] = useState(false);
  const [selectedIconId, setSelectedIconId] = useState<CustomIconId>(page.iconId as CustomIconId);
  const [showIconPicker, setShowIconPicker] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const wrapperRef = useRef<HTMLDivElement>(null);

  const handleDoubleClick = (e: React.MouseEvent) => {
    e.stopPropagation();
    setRenameValue(page.name);
    setSelectedIconId(page.iconId as CustomIconId);
    setShowIconPicker(false);
    setIsRenaming(true);
    setTimeout(() => inputRef.current?.focus(), 50);
  };

  const handleConfirmRename = async () => {
    const trimmed = renameValue.trim();
    if (!trimmed) {
      setIsRenaming(false);
      return;
    }
    const nameChanged = trimmed !== page.name;
    const iconChanged = selectedIconId !== page.iconId;
    if (!nameChanged && !iconChanged) {
      setIsRenaming(false);
      return;
    }
    // Check for duplicate page names (only if name changed)
    if (nameChanged) {
      const store = useSettingsStore.getState();
      const existingNames = [
        ...SYSTEM_PAGE_KEYS.map((k) => t(k)),
        ...store.settings.customPages.filter((p) => p.id !== page.id && !p.deletedAt).map((p) => p.name),
      ];
      if (existingNames.some((n) => n.toLowerCase() === trimmed.toLowerCase())) {
        setRenameError(true);
        return;
      }
    }
    // Update the object in the objects table
    try {
      await invoke('object_update', {
        objectId: page.id,
        input: { name: trimmed, properties: {} },
      });
    } catch { /* silent */ }
    // Update Zustand state so sidebar reflects the change
    const store = useSettingsStore.getState();
    store.updateSetting('', 'customPages',
      store.settings.customPages.map((p) =>
        p.id === page.id ? { ...p, name: trimmed, iconId: selectedIconId } : p
      )
    );
    setIsRenaming(false);
  };

  const handleCancelRename = () => {
    setIsRenaming(false);
    setRenameValue(page.name);
    setSelectedIconId(page.iconId as CustomIconId);
    setShowIconPicker(false);
  };

  // Use ref to always call the latest handleConfirmRename (avoids stale closure)
  const handleConfirmRenameRef = useRef(handleConfirmRename);
  handleConfirmRenameRef.current = handleConfirmRename;

  // Close on outside click
  useEffect(() => {
    if (!isRenaming) return;
    const handler = (e: MouseEvent) => {
      if (inputRef.current && !inputRef.current.contains(e.target as Node) &&
          wrapperRef.current && !wrapperRef.current.contains(e.target as Node)) {
        handleConfirmRenameRef.current();
      }
    };
    setTimeout(() => document.addEventListener('mousedown', handler), 0);
    return () => document.removeEventListener('mousedown', handler);
  }, [isRenaming]);

  return (
    <div ref={wrapperRef} style={{ position: 'relative' }} onDoubleClick={handleDoubleClick}>
      <NavButton
        path={`/workspace/custom/${page.id}`}
        Icon={resolveCustomIcon(page.iconId)}
        label={page.name}
        isActive={isActive}
        onClick={onClick}
        position={position}
      />
      {isRenaming && (
        <div
          style={{
            position: 'fixed',
            left: wrapperRef.current
              ? (isHorizontal
                  ? wrapperRef.current.getBoundingClientRect().left
                  : wrapperRef.current.getBoundingClientRect().right + 8)
              : 56,
            ...(wrapperRef.current && isBottom
              ? {
                  bottom: window.innerHeight - wrapperRef.current.getBoundingClientRect().top + 8,
                  top: 'auto',
                }
              : {
                  top: wrapperRef.current
                    ? (isHorizontal
                        ? wrapperRef.current.getBoundingClientRect().bottom + 8
                        : wrapperRef.current.getBoundingClientRect().top)
                    : '50%',
                }),
            display: 'flex',
            flexDirection: 'column',
            gap: 8,
            padding: '6px 10px',
            background: 'var(--bg-elevated)',
            borderRadius: 8,
            boxShadow: 'var(--shadow-lg)',
            zIndex: 300,
            border: '1px solid var(--border-subtle)',
          }}
        >
          <div style={{ display: 'flex', alignItems: 'flex-start', gap: 6 }}>
            {/* Icon picker trigger */}
            <button
              onClick={() => setShowIconPicker(!showIconPicker)}
              style={{
                width: 32,
                height: 32,
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                borderRadius: 6,
                border: '1px solid var(--border-subtle)',
                background: 'transparent',
                cursor: 'pointer',
                flexShrink: 0,
              }}
              title={t('navigation:add_page_placeholder') ?? 'Choose icon'}
            >
              {React.createElement(CUSTOM_ICON_MAP[selectedIconId], { size: 18, style: { color: 'var(--accent-primary)' } })}
            </button>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
              <input
                ref={inputRef}
                value={renameValue}
                onChange={(e) => {
                  setRenameValue(e.target.value.slice(0, 30));
                  setRenameError(false);
                }}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') handleConfirmRename();
                  if (e.key === 'Escape') handleCancelRename();
                }}
                maxLength={30}
                autoFocus
                style={{
                  padding: '6px 10px',
                  fontSize: 14,
                  border: renameError ? '1px solid #e74c3c' : '1px solid var(--accent-primary)',
                  borderRadius: 6,
                  background: 'transparent',
                  color: 'var(--text-primary)',
                  fontFamily: 'inherit',
                  outline: 'none',
                  width: 140,
                  animation: renameError ? 'shake 0.4s ease' : 'none',
                }}
              />
              {renameError && (
                <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: 8 }}>
                  <span style={{ fontSize: 11, color: '#e74c3c', whiteSpace: 'nowrap' }}>
                    {t('page_name_exists')}
                  </span>
                  <button
                    onClick={handleCancelRename}
                    onMouseEnter={(e) => { e.currentTarget.style.color = 'var(--accent-primary)'; }}
                    onMouseLeave={(e) => { e.currentTarget.style.color = 'var(--text-tertiary)'; }}
                    style={{ fontSize: 11, color: 'var(--text-tertiary)', background: 'none', border: 'none', cursor: 'pointer', padding: 0, transition: 'color 0.15s ease' }}
                  >
                    {t('common:cancel')}
                  </button>
                </div>
              )}
            </div>
          </div>

          {/* Icon picker grid */}
          {showIconPicker && (
            <div
              style={{
                display: 'grid',
                gridTemplateColumns: 'repeat(5, 1fr)',
                gap: 4,
                padding: '4px 0',
              }}
            >
              {(Object.entries(CUSTOM_ICON_MAP) as [CustomIconId, LucideIcon][]).map(([id, IconComp]) => (
                <button
                  key={id}
                  onClick={() => {
                    setSelectedIconId(id);
                    setShowIconPicker(false);
                  }}
                  style={{
                    width: 32,
                    height: 32,
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    borderRadius: 6,
                    border: selectedIconId === id ? '1px solid var(--accent-primary)' : '1px solid transparent',
                    background: selectedIconId === id ? 'rgba(91,124,153,0.08)' : 'transparent',
                    cursor: 'pointer',
                  }}
                >
                  <IconComp size={18} style={{ color: selectedIconId === id ? 'var(--accent-primary)' : 'var(--text-secondary)' }} />
                </button>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

// =============================================================================
// AddPageButton — "+" button with popover for name + icon selection
// =============================================================================

export function AddPageButton({
  onCreate,
  position = 'left',
}: {
  onCreate: (page: CustomPage) => void;
  position?: import('./NavButton').NavPosition;
}) {
  const isHorizontal = position === 'top' || position === 'bottom';
  const isBottom = position === 'bottom';
  const [isCreating, setIsCreating] = useState(false);
  const [name, setName] = useState('');
  const [nameError, setNameError] = useState(false);
  const [selectedIconId, setSelectedIconId] = useState<CustomIconId>(DEFAULT_CUSTOM_ICON);
  const [showIconPicker, setShowIconPicker] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const buttonRef = useRef<HTMLButtonElement>(null);
  const popoverRef = useRef<HTMLDivElement>(null);
  const { t } = useTranslation(['navigation', 'common']);
  const currentAccount = useAuthStore((s) => s.currentAccount);
  const addCustomPage = useSettingsStore((s) => s.addCustomPage);

  const handleCancel = useCallback(() => {
    setIsCreating(false);
    setName('');
    setNameError(false);
    setSelectedIconId(DEFAULT_CUSTOM_ICON);
    setShowIconPicker(false);
  }, []);

  const handleConfirm = useCallback(() => {
    const trimmed = name.trim();
    if (!trimmed || !currentAccount) {
      handleCancel();
      return;
    }
    // Check for duplicate page names
    const store = useSettingsStore.getState();
    const existingNames = [
      ...SYSTEM_PAGE_KEYS.map((k) => t(k)),
      ...store.settings.customPages.filter((p) => !p.deletedAt).map((p) => p.name),
    ];
    if (existingNames.some((n) => n.toLowerCase() === trimmed.toLowerCase())) {
      setNameError(true);
      return;
    }
    addCustomPage(currentAccount.id, trimmed, selectedIconId).then((page) => {
      onCreate(page);
    });
    handleCancel();
  }, [name, selectedIconId, currentAccount, addCustomPage, onCreate, t, handleCancel]);

  // Close popover on outside click
  useEffect(() => {
    if (!isCreating) return;
    const handler = (e: MouseEvent) => {
      if (
        popoverRef.current &&
        !popoverRef.current.contains(e.target as Node) &&
        buttonRef.current &&
        !buttonRef.current.contains(e.target as Node)
      ) {
        // If input has text → create page; if empty → cancel
        handleConfirm();
      }
    };
    // Small delay to avoid conflicting with the button click
    setTimeout(() => document.addEventListener('mousedown', handler), 0);
    return () => document.removeEventListener('mousedown', handler);
  }, [isCreating, handleConfirm]);

  const SelectedIcon = CUSTOM_ICON_MAP[selectedIconId];

  // Hover name card (same portal pattern as NavButton)
  const wrapperRef = useRef<HTMLDivElement>(null);
  const [cardStyle, setCardStyle] = useState<React.CSSProperties | null>(null);
  const [isHovered, setIsHovered] = useState(false);

  const updateCardPosition = useCallback(() => {
    if (wrapperRef.current) {
      const rect = wrapperRef.current.getBoundingClientRect();
      if (isHorizontal) {
        if (isBottom) {
          setCardStyle({
            top: 'auto',
            bottom: window.innerHeight - rect.top + 8,
            left: rect.left + rect.width / 2,
            transform: 'translateX(-50%)',
          });
        } else {
          setCardStyle({
            top: rect.bottom + 8,
            bottom: 'auto',
            left: rect.left + rect.width / 2,
            transform: 'translateX(-50%)',
          });
        }
      } else {
        setCardStyle({
          top: rect.top + rect.height / 2,
          bottom: 'auto',
          left: rect.right + 8,
          transform: 'translateY(-50%)',
        });
      }
    }
  }, [isHorizontal, isBottom]);

  const handleMouseEnter = useCallback(() => {
    setIsHovered(true);
    updateCardPosition();
  }, [updateCardPosition]);

  const handleMouseLeave = useCallback(() => {
    setIsHovered(false);
  }, []);

  useEffect(() => {
    if (!isHovered) return;
    window.addEventListener('scroll', updateCardPosition, true);
    window.addEventListener('resize', updateCardPosition);
    return () => {
      window.removeEventListener('scroll', updateCardPosition, true);
      window.removeEventListener('resize', updateCardPosition);
    };
  }, [isHovered, updateCardPosition]);

  const nameCard = isHovered && !isCreating ? (
    <div
      className={isHorizontal ? styles.nameCardPortalHorizontal : styles.nameCardPortal}
      style={{
        position: 'fixed',
        ...cardStyle,
        zIndex: 200,
      }}
      role="tooltip"
      aria-hidden="true"
    >
      {t('add_page')}
    </div>
  ) : null;

  return (
    <div className={styles.addPageRow} style={isHorizontal ? { flexDirection: 'row' } : {}}>
      {/* + button */}
      <div
        ref={wrapperRef}
        onMouseEnter={handleMouseEnter}
        onMouseLeave={handleMouseLeave}
        style={isHorizontal ? { width: 40, height: 40 } : undefined}
      >
        <button
          ref={buttonRef}
          className={styles.addPageButton}
          style={isHorizontal ? { width: 40, height: 40, borderRadius: 10 } : {}}
          onClick={() => {
            setIsCreating(true);
            setSelectedIconId(DEFAULT_CUSTOM_ICON);
            setShowIconPicker(false);
            setTimeout(() => inputRef.current?.focus(), 100);
          }}
          aria-label={t('add_page')}
          data-tauri-drag-region="false"
        >
          <Plus size={20} />
        </button>
        {createPortal(nameCard, document.body)}
      </div>

      {/* Popover create row — rendered outside sidebar flow */}
      {isCreating && (
        <div
          ref={popoverRef}
          style={{
            position: 'fixed',
            left: buttonRef.current
              ? (isHorizontal
                  ? buttonRef.current.getBoundingClientRect().left
                  : buttonRef.current.getBoundingClientRect().right + 8)
              : 56,
            ...(buttonRef.current && isBottom
              ? {
                  bottom: window.innerHeight - buttonRef.current.getBoundingClientRect().top + 8,
                  top: 'auto',
                }
              : {
                  top: buttonRef.current
                    ? (isHorizontal
                        ? buttonRef.current.getBoundingClientRect().bottom + 8
                        : buttonRef.current.getBoundingClientRect().top)
                    : '50%',
                }),
            display: 'flex',
            flexDirection: 'column',
            gap: 8,
            padding: '10px 12px',
            background: 'var(--bg-elevated)',
            borderRadius: 8,
            boxShadow: 'var(--shadow-lg)',
            zIndex: 300,
            border: '1px solid var(--border-subtle)',
          }}
        >
          {/* Name input row */}
          <div style={{ display: 'flex', alignItems: 'flex-start', gap: 8 }}>
            {/* Icon picker trigger */}
            <button
              onClick={() => setShowIconPicker(!showIconPicker)}
              style={{
                width: 32,
                height: 32,
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                borderRadius: 6,
                border: '1px solid var(--border-subtle)',
                background: 'transparent',
                cursor: 'pointer',
                flexShrink: 0,
              }}
              title={t('add_page_placeholder') ?? 'Choose icon'}
              aria-label={t("navigation:add_page")}
            >
              <SelectedIcon size={18} style={{ color: 'var(--accent-primary)' }} />
            </button>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
              <input
                ref={inputRef}
                value={name}
                onChange={(e) => {
                  setName(e.target.value.slice(0, 20));
                  setNameError(false);
                }}
                onBlur={(e) => {
                  // Only confirm if the blur is not caused by clicking inside the popover
                  if (popoverRef.current && !popoverRef.current.contains(e.relatedTarget as Node)) {
                    handleConfirm();
                  }
                }}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') handleConfirm();
                  if (e.key === 'Escape') handleCancel();
                }}
                placeholder={t('add_page_placeholder')}
                maxLength={20}
                autoFocus
                aria-label={t('add_page_placeholder')}
                style={{
                  padding: '6px 10px',
                  fontSize: 14,
                  border: nameError ? '1px solid #e74c3c' : '1px solid var(--accent-primary)',
                  borderRadius: 6,
                  background: 'transparent',
                  color: 'var(--text-primary)',
                  fontFamily: 'inherit',
                  outline: 'none',
                  width: 160,
                  animation: nameError ? 'shake 0.4s ease' : 'none',
                }}
              />
              {nameError && (
                <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: 8 }}>
                  <span style={{ fontSize: 11, color: '#e74c3c', whiteSpace: 'nowrap' }}>
                    {t('page_name_exists')}
                  </span>
                  <button
                    onClick={handleCancel}
                    onMouseEnter={(e) => { e.currentTarget.style.color = 'var(--accent-primary)'; }}
                    onMouseLeave={(e) => { e.currentTarget.style.color = 'var(--text-tertiary)'; }}
                    style={{ fontSize: 11, color: 'var(--text-tertiary)', background: 'none', border: 'none', cursor: 'pointer', padding: 0, transition: 'color 0.15s ease' }}
                  >
                    {t('common:cancel')}
                  </button>
                </div>
              )}
            </div>
          </div>

          {/* Icon picker grid */}
          {showIconPicker && (
            <div
              style={{
                display: 'grid',
                gridTemplateColumns: 'repeat(5, 1fr)',
                gap: 4,
                padding: '4px 0',
              }}
            >
              {(Object.entries(CUSTOM_ICON_MAP) as [CustomIconId, LucideIcon][]).map(([id, IconComp]) => (
                <button
                  key={id}
                  onClick={() => {
                    setSelectedIconId(id);
                    setShowIconPicker(false);
                  }}
                  style={{
                    width: 32,
                    height: 32,
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    borderRadius: 6,
                    border: id === selectedIconId
                      ? '2px solid var(--accent-primary)'
                      : '1px solid transparent',
                    background: id === selectedIconId
                      ? 'var(--accent-primary-transparent, rgba(91,124,153,0.1))'
                      : 'transparent',
                    cursor: 'pointer',
                  }}
                  title={id}
                  aria-label={id}
                >
                  <IconComp
                    size={16}
                    style={{
                      color: id === selectedIconId
                        ? 'var(--accent-primary)'
                        : 'var(--text-secondary)',
                    }}
                  />
                </button>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

// =============================================================================
// SideNavigation — main sidebar component
// =============================================================================

export function SideNavigation() {
  const navigate = useNavigate();
  const location = useLocation();
  const activeCustomPages = useActiveCustomPages();
  const sidebarPosition = useSettingsStore((s) => s.settings.sidebarPosition);
  const isHorizontal = sidebarPosition === 'top' || sidebarPosition === 'bottom';
  const { t } = useTranslation('navigation');

  const { items, showSearch, setShowSearch } = useBoundNavActions();
  const aiQuickChatPlacement: import('./useNavigationItems').AiQuickChatPlacement =
    sidebarPosition === 'bottom' ? 'top' : 'left';
  const { showQuickChat, setShowQuickChat, aiButtonRef, quickChatPos } = useAiQuickChat(520, aiQuickChatPlacement);

  const isWorkspaceSectionActive = (sectionPath: string): boolean => {
    // Custom pages are at /workspace/custom/:id — they never match section-based routes
    if (location.pathname.startsWith('/workspace/custom/')) return false;
    if (!location.pathname.startsWith('/workspace')) return false;
    const section = sectionPath.split('section=')[1];
    if (!section) return !location.search.includes('section=');
    return location.search.includes(`section=${section}`);
  };

  const isCustomPageActive = (pageId: string): boolean => {
    return location.pathname === `/workspace/custom/${pageId}`;
  };

  const handleCustomPageNavigate = (page: CustomPage) => {
    navigate(`/workspace/custom/${page.id}`);
  };

  const navStyle: React.CSSProperties = isHorizontal
    ? {
        width: '100%',
        height: 48,
        flexDirection: 'row',
        borderRight: 'none',
        borderLeft: 'none',
        borderBottom: sidebarPosition === 'top' ? '1px solid var(--border-subtle)' : 'none',
        borderTop: sidebarPosition === 'bottom' ? '1px solid var(--border-subtle)' : 'none',
        padding: '0 12px',
        overflow: 'visible',
      }
    : {
        width: 48,
        height: '100vh',
        flexDirection: 'column',
        borderRight: sidebarPosition === 'left' ? '1px solid var(--border-subtle)' : 'none',
        borderLeft: sidebarPosition === 'right' ? '1px solid var(--border-subtle)' : 'none',
        borderBottom: 'none',
        borderTop: 'none',
        padding: '12px 0',
      };

  const zoneStyle: React.CSSProperties = isHorizontal
    ? { flexDirection: 'row', width: 'auto', height: '100%', overflow: 'visible' }
    : { flexDirection: 'column', width: '100%', height: 'auto', overflow: 'visible' };

  return (
    <nav className={styles.sideNav} aria-label={t('home')} style={navStyle}>
      <div className={styles.logo} style={isHorizontal ? { marginBottom: 0, marginRight: 12 } : {}}>S</div>

      <div className={styles.navPrimary} style={{ ...zoneStyle, flex: 1, overflow: isHorizontal ? 'hidden' : 'auto' }}>
        {/* Default pages — icons from PAGE_ICON_MAP (§7.4 SSOT) */}
        {primaryItems.map((item) => {
          const isActive =
            item.path === '/'
              ? location.pathname === '/'
              : isWorkspaceSectionActive(item.path);
          return (
            <NavButton
              key={item.path}
              path={item.path}
              Icon={PAGE_ICON_MAP[item.iconKey]}
              label={t(item.labelKey)}
              isActive={isActive}
              onClick={() => navigate(item.path)}
              position={sidebarPosition}
            />
          );
        })}

        {/* Custom pages — icons from CUSTOM_ICON_MAP via iconId (§9.8) */}
        {activeCustomPages.map((page) => (
          <RenameableNavButton
            key={page.id}
            page={page}
            isActive={isCustomPageActive(page.id)}
            onClick={() => handleCustomPageNavigate(page)}
            position={sidebarPosition}
          />
        ))}

        {/* Add page button */}
        <AddPageButton onCreate={(page) => {
          navigate(`/workspace/custom/${page.id}`);
        }} position={sidebarPosition} />
      </div>

      <div className={styles.navSecondary} style={{ ...zoneStyle, flexShrink: 0 }}>
        {items.map((item, i) => {
          if (item.type === 'action') {
            const isSearch = item.iconKey === 'search';
            return (
              <div key={`action-${i}`} style={{ position: 'relative' }}>
                <NavButton
                  Icon={PAGE_ICON_MAP[item.iconKey]}
                  label={t(item.labelKey)}
                  onClick={item.action}
                  position={sidebarPosition}
                />
                {isSearch && showSearch && createPortal(
                  <SearchPopover onClose={() => setShowSearch(false)} />,
                  document.body
                )}
              </div>
            );
          }
          if (item.path === '/llm-chat') {
            return (
              <div ref={aiButtonRef} key={item.path} data-ai-button="true">
                <NavButton
                  path={item.path}
                  Icon={PAGE_ICON_MAP[item.iconKey]}
                  label={t(item.labelKey)}
                  isActive={showQuickChat || location.pathname.startsWith(item.path)}
                  onClick={() => {
                    if (location.pathname.startsWith('/llm-chat')) return;
                    setShowQuickChat((prev) => !prev);
                  }}
                  position={sidebarPosition}
                />
                {showQuickChat && createPortal(
                  <AiQuickChatPopover
                    position={quickChatPos}
                    onClose={() => setShowQuickChat(false)}
                    placement={sidebarPosition === 'bottom' ? 'top' : 'left'}
                  />,
                  document.body
                )}
              </div>
            );
          }
          const isActive = item.path === '/'
            ? location.pathname === '/'
            : location.pathname.startsWith(item.path);
          return (
            <NavButton
              key={item.path}
              path={item.path}
              Icon={PAGE_ICON_MAP[item.iconKey]}
              label={t(item.labelKey)}
              isActive={isActive}
              onClick={() => navigate(item.path)}
              position={sidebarPosition}
            />
          );
        })}
      </div>
    </nav>
  );
}
