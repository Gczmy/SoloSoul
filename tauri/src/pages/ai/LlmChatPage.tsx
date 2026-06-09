import { useState, useRef, useEffect, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { AppShell } from '@/components/layout/AppShell';
import { Button } from '@/components/ui/Button';
import { useAuthStore } from '@/stores/authStore';
import { useLlmStore } from '@/stores/llmStore';
import i18n from '@/lib/i18n';
import { buildSystemPrompt, buildMessagesWithSystemPromptAndChunks } from '@/lib/llm/systemPromptBuilder';
import { searchGuideChunks, formatChunksAsSystemMessage } from '@/lib/llm/guideService';
import ReactMarkdown from 'react-markdown';
import rehypeHighlight from 'rehype-highlight';
import { MessageSquare, Settings, Send, Plus, Copy, Check, Trash2, Pencil, RotateCw, RefreshCw, X, Undo2, Delete, BarChart3 } from 'lucide-react';
import { markConversationPending } from '@/lib/notification';

interface ChatMsg {
  role: string;
  content: string;
  createdAt: string;
}

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

function formatTimestamp(iso: string): string {
  const d = new Date(iso);
  return d.getFullYear() + '-' +
    String(d.getMonth() + 1).padStart(2, '0') + '-' +
    String(d.getDate()).padStart(2, '0') + ' ' +
    String(d.getHours()).padStart(2, '0') + ':' +
    String(d.getMinutes()).padStart(2, '0') + ':' +
    String(d.getSeconds()).padStart(2, '0');
}

function formatRelative(iso: string): string {
  const diff = Date.now() - new Date(iso).getTime();
  const mins = Math.floor(diff / 60000);
  if (mins < 1) return '刚刚';
  if (mins < 60) return mins + ' 分钟前';
  const hours = Math.floor(mins / 60);
  if (hours < 24) return hours + ' 小时前';
  return formatTimestamp(iso).slice(0, 10);
}

function generateId(): string {
  return 'conv_' + Date.now() + '_' + Math.random().toString(36).slice(2, 8);
}

const COPY_FEEDBACK_DURATION = 1500;

export function LlmChatPage() {
  const navigate = useNavigate();
  const { t } = useTranslation(['settings', 'common']);
  const accountId = useAuthStore((s) => s.currentAccount?.id);

  // State
  const [activeProvider, setActiveProvider] = useState<{ id: string; name: string; model: string; baseUrl: string; apiType: string } | null>(null);
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

  // Online status
  const [isOnline, setIsOnline] = useState<boolean | null>(null);
  const [checkingOnline, setCheckingOnline] = useState(false);

  // Rename
  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [renameValue, setRenameValue] = useState('');

  // Copy feedback
  const [copiedIndex, setCopiedIndex] = useState<number | null>(null);

  // Floating card (trash viewer)
  const [floatingConv, setFloatingConv] = useState<Conversation | null>(null);
  const [confirmPermanentDelete, setConfirmPermanentDelete] = useState<string | null>(null);

  // Refs
  const chatEndRef = useRef<HTMLDivElement>(null);
  const renameInputRef = useRef<HTMLInputElement>(null);

  // Load providers and config
  useEffect(() => {
    if (!accountId) return;
    (async () => {
      try {
        const cfg = await invoke<{ activeProviderId?: string; aiFeaturesEnabled?: { chat: boolean }; includeSystemPrompt?: boolean }>('llm_get_config', { accountId });
        setIsAiEnabled(cfg.aiFeaturesEnabled?.chat ?? false);
        setIncludeSystemPrompt(cfg.includeSystemPrompt ?? true);
        if (!cfg.activeProviderId) { setIsConfigured(false); setLoading(false); return; }
        const providers = await invoke<any[]>('llm_get_providers', { accountId });
        const active = providers.find((p) => p.id === cfg.activeProviderId);
        if (active) {
          setActiveProvider({ id: active.id, name: active.name, model: active.model, baseUrl: active.baseUrl, apiType: active.apiType });
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
  const loadAllLists = useCallback(async () => {
    if (!accountId || !isAiEnabled || !isConfigured) return;
    try {
      const [list, trash] = await Promise.all([
        invoke<ConversationSummary[]>('llm_list_conversations', { accountId }),
        invoke<ConversationSummary[]>('llm_list_trash', { accountId }),
      ]);
      setConversations(list);
      setTrashList(trash);
    } catch { /* ignore */ }
  }, [accountId, isAiEnabled, isConfigured]);

  useEffect(() => { loadAllLists(); }, [loadAllLists]);

  // Check online status
  const checkOnline = useCallback(async () => {
    if (!activeProvider || !accountId) return;
    setCheckingOnline(true);
    try {
      let key = '';
      try { key = await invoke<string>('llm_get_api_key', { accountId, providerId: activeProvider.id }); } catch { /* may not have key */ }
      const online = await invoke<boolean>('llm_check_connection', {
        baseUrl: activeProvider.baseUrl, apiKey: key, model: activeProvider.model, apiType: activeProvider.apiType,
      });
      setIsOnline(online);
    } catch {
      setIsOnline(false);
    } finally {
      setCheckingOnline(false);
    }
  }, [activeProvider, accountId]);

  useEffect(() => {
    if (activeProvider && accountId) checkOnline();
  }, [activeProvider, accountId, checkOnline]);

  // Periodic online check every 60s
  useEffect(() => {
    if (!activeProvider) return;
    const interval = setInterval(checkOnline, 60000);
    return () => clearInterval(interval);
  }, [activeProvider, checkOnline]);

  // Scroll to bottom
  useEffect(() => {
    chatEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  // Focus rename input
  useEffect(() => {
    if (renamingId) renameInputRef.current?.focus();
  }, [renamingId]);

  const loadConversation = useCallback(async (convId: string) => {
    if (!accountId) return;
    try {
      const conv = await invoke<Conversation>('llm_get_conversation', { accountId, conversationId: convId });
      setCurrentConvId(conv.id);
      setCurrentConv(conv);
      setMessages(conv.messages);
    } catch { /* may be deleted */ }
  }, [accountId]);

  const handleNewConversation = () => {
    const id = generateId();
    setCurrentConvId(id);
    setCurrentConv({ id, name: '', isTemporary: true, messages: [], updatedAt: nowISO() });
    setMessages([]);
  };

  const llmStore = useLlmStore();

  // Listen to LLM stream state: update messages when stream buffer changes
  useEffect(() => {
    if (!llmStore.isStreaming || !llmStore.streamingConvId) return;
    // Update the last assistant message with current stream buffer
    setMessages((prev) => {
      if (prev.length === 0) return prev;
      const lastIdx = prev.length - 1;
      if (prev[lastIdx].role !== 'assistant') return prev;
      const updated = [...prev];
      updated[lastIdx] = { ...updated[lastIdx], content: llmStore.streamBuffer };
      return updated;
    });
  }, [llmStore.streamBuffer, llmStore.isStreaming, llmStore.streamingConvId]);

  // Listen to LLM stream done: save conversation when stream ends
  useEffect(() => {
    if (!llmStore.isStreaming && llmStore.streamingConvId && llmStore.streamBuffer) {
      // Stream finished, save final conversation
      const convId = llmStore.streamingConvId;
      const currentMsgs = messages;
      if (currentMsgs.length > 0 && currentMsgs[currentMsgs.length - 1].role === 'assistant') {
        const finalConv: Conversation = {
          id: convId,
          name: currentConv?.name || '',
          isTemporary: false,
          messages: currentMsgs,
          updatedAt: nowISO(),
        };
        invoke('llm_save_conversation', { accountId, conversation: finalConv }).catch(() => {});
        setCurrentConv(finalConv);
        loadAllLists();
      }
      llmStore.reset();
      setIsSending(false);
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [llmStore.isStreaming, llmStore.streamingConvId]);

  // Listen to LLM stream error
  useEffect(() => {
    if (llmStore.streamError) {
      const errMsg = llmStore.streamError;
      setMessages((prev) => {
        if (prev.length === 0) return prev;
        const lastIdx = prev.length - 1;
        if (prev[lastIdx].role !== 'assistant') return prev;
        const updated = [...prev];
        updated[lastIdx] = { ...updated[lastIdx], content: `${t('settings:ai_chat_error_prefix')}: ${errMsg}` };
        return updated;
      });
      llmStore.reset();
      setIsSending(false);
    }
  }, [llmStore.streamError, llmStore, t]);

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
    const convName = isFirstMsg ? text.slice(0, 30) : (currentConv?.name || '');
    const convId = currentConvId || generateId();

    // Save immediately with user message
    if (isFirstMsg || currentConv?.isTemporary) {
      const partialConv: Conversation = {
        id: convId, name: convName, isTemporary: false, messages: updatedMessages, updatedAt: nowISO(),
      };
      try {
        await invoke('llm_save_conversation', { accountId, conversation: partialConv });
        setCurrentConvId(convId);
        setCurrentConv(partialConv);
        loadAllLists();
      } catch { /* continue */ }
    }

    // Add empty assistant message for streaming
    const assistantMsg: ChatMsg = { role: 'assistant', content: '', createdAt: nowISO() };
    const streamingMessages = [...updatedMessages, assistantMsg];
    setMessages(streamingMessages);

    try {
      const apiKey = await invoke<string>('llm_get_api_key', { accountId, providerId: activeProvider.id });

      // Build system prompt and help doc (merged into single system message)
      let allMessages: Array<{ role: string; content: string }> = [];
      if (includeSystemPrompt) {
        const systemPrompt = buildSystemPrompt();
        const chunks = await searchGuideChunks(text, i18n.language || 'zh-CN');
        const docPrompt = formatChunksAsSystemMessage(chunks);
        allMessages = buildMessagesWithSystemPromptAndChunks(text, updatedMessages, systemPrompt, docPrompt);
      } else {
        allMessages = updatedMessages.map((m) => ({ role: m.role, content: m.content }));
        allMessages.push({ role: 'user', content: text });
      }

      // Convert to serde_json::Value compatible format
      const messagesPayload = allMessages.map((m) => ({ role: m.role, content: m.content }));

      // Start stream
      markConversationPending(convId);
      llmStore.startStream(convId);

      // Call streaming command (fire-and-forget)
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
      const errorAssistantMsg: ChatMsg = { role: 'assistant', content: `${t('settings:ai_chat_error_prefix')}: ${errMsg}`, createdAt: nowISO() };
      const errorMessages = [...updatedMessages, errorAssistantMsg];
      setMessages(errorMessages);

      const errorConv: Conversation = {
        id: convId, name: convName, isTemporary: false, messages: errorMessages, updatedAt: nowISO(),
      };
      try {
        await invoke('llm_save_conversation', { accountId, conversation: errorConv });
        setCurrentConv(errorConv);
        loadAllLists();
      } catch { /* best effort */ }
      setIsSending(false);
    }
  };

  const handleRenameStart = (convId: string, currentName: string) => {
    setRenamingId(convId);
    setRenameValue(currentName);
  };

  const handleRenameConfirm = async () => {
    if (!renamingId || !renameValue.trim() || !accountId) { setRenamingId(null); return; }
    await invoke('llm_rename_conversation', { accountId, conversationId: renamingId, name: renameValue.trim() });
    setConversations((prev) => prev.map((c) => c.id === renamingId ? { ...c, name: renameValue.trim() } : c));
    if (currentConv?.id === renamingId) setCurrentConv((prev) => prev ? { ...prev, name: renameValue.trim() } : prev);
    setRenamingId(null);
  };

  // Soft-delete: move to trash
  const handleSoftDelete = async (convId: string) => {
    if (!accountId) return;
    await invoke('llm_soft_delete_conversation', { accountId, conversationId: convId });
    setConversations((prev) => prev.filter((c) => c.id !== convId));
    if (currentConvId === convId) handleNewConversation();
    loadAllLists();
  };

  // Restore from trash
  const handleRestore = async (convId: string) => {
    if (!accountId) return;
    await invoke('llm_restore_conversation', { accountId, conversationId: convId });
    loadAllLists();
  };

  // Permanent delete
  const handlePermanentDelete = async (convId: string) => {
    if (!accountId) return;
    await invoke('llm_permanent_delete', { accountId, conversationId: convId });
    setTrashList((prev) => prev.filter((c) => c.id !== convId));
    setConfirmPermanentDelete(null);
    setFloatingConv((prev) => prev?.id === convId ? null : prev);
  };

  // View trash conversation as floating card
  const handleViewTrashConv = async (convId: string) => {
    if (!accountId) return;
    try {
      const conv = await invoke<Conversation>('llm_get_conversation', { accountId, conversationId: convId });
      setFloatingConv(floatingConv?.id === convId ? null : conv);
    } catch { /* ignore */ }
  };

  const handleCopy = async (content: string, index: number) => {
    try {
      await navigator.clipboard.writeText(content);
      setCopiedIndex(index);
      setTimeout(() => setCopiedIndex(null), COPY_FEEDBACK_DURATION);
    } catch { /* fallback */ }
  };

  // ── Render logic ──

  if (loading) {
    return (
      <AppShell title={t('settings:ai_chat')} onBack={() => navigate('/home')}>
        <div style={{ maxWidth: 600, margin: '0 auto', textAlign: 'center', padding: '48px 24px' }}>
          <MessageSquare size={48} style={{ marginBottom: 16, opacity: 0.3 }} />
          <p style={{ color: 'var(--text-secondary)' }}>{t('common:loading')}</p>
        </div>
      </AppShell>
    );
  }

  if (!isAiEnabled || !isConfigured) {
    return (
      <AppShell title={t('settings:ai_chat')} onBack={() => navigate('/home')}
        actions={
          <Button variant="secondary" size="sm" onClick={() => navigate('/settings/llm', { state: { from: '/llm-chat' } })}>
            <Settings size={14} style={{ marginRight: 4 }} /> {t('settings:ai_chat_configure')}
          </Button>
        }
      >
        <div style={{ maxWidth: 600, margin: '0 auto', textAlign: 'center', padding: '48px 24px' }}>
          <MessageSquare size={48} style={{ marginBottom: 16, opacity: 0.3, color: 'var(--text-tertiary)' }} />
          <h2 style={{ fontSize: 18, fontWeight: 600, margin: '0 0 8px' }}>{t('settings:ai_chat')}</h2>
          <p style={{ fontSize: 14, color: 'var(--text-secondary)', marginBottom: 16 }}>{t('settings:ai_chat_disabled')}</p>
          <Button onClick={() => navigate('/settings/llm', { state: { from: '/llm-chat' } })}>{t('settings:ai_chat_configure')}</Button>
        </div>
      </AppShell>
    );
  }

  const isLocal = activeProvider ? isOllama(activeProvider.baseUrl) : false;

  return (
    <AppShell title={t('settings:ai_chat')} onBack={() => navigate('/home')}
      actions={
        <div style={{ display: 'flex', gap: 8 }}>
          <button onClick={() => navigate('/settings/llm/stats', { state: { from: '/llm-chat' } })} title="使用统计"
            style={{ padding: 8, borderRadius: 8, border: '1px solid var(--border-subtle)', background: 'transparent', cursor: 'pointer', color: 'var(--text-secondary)' }}>
            <BarChart3 size={16} />
          </button>
          <button onClick={() => navigate('/settings/llm', { state: { from: '/llm-chat' } })} title={t('settings:llm_config')}
            style={{ padding: 8, borderRadius: 8, border: '1px solid var(--border-subtle)', background: 'transparent', cursor: 'pointer', color: 'var(--text-secondary)' }}>
            <Settings size={16} />
          </button>
        </div>
      }
    >
      <div style={{ position: 'fixed', top: 56, left: 48, right: 0, bottom: 0, display: 'flex', overflow: 'hidden' }}>
        {/* ── Conversation Sidebar (fixed layout, only list scrolls) ── */}
        <div style={{ width: 220, minWidth: 180, maxWidth: 360, borderRight: '1px solid var(--border-subtle)', display: 'flex', flexDirection: 'column', background: 'var(--bg-toolbar)', overflow: 'hidden', height: '100%' }}>
          <div style={{ padding: '10px 12px', borderBottom: '1px solid var(--border-subtle)', flexShrink: 0 }}>
            <Button variant="secondary" size="sm" onClick={handleNewConversation} style={{ width: '100%' }}>
              <Plus size={14} style={{ marginRight: 4 }} /> {t('settings:ai_new_conv') }
            </Button>
          </div>
          <div style={{ flex: 1, overflowY: 'auto', padding: '6px 0', minHeight: 0 }}>
            {conversations.length === 0 && (
              <p style={{ fontSize: 12, color: 'var(--text-tertiary)', textAlign: 'center', padding: '24px 12px' }}>
                {t('settings:ai_no_convs') }
              </p>
            )}
            {conversations.map((conv) => (
              <div key={conv.id} onClick={() => loadConversation(conv.id)} style={{
                display: 'flex', alignItems: 'center', gap: 6, padding: '8px 12px', cursor: 'pointer', fontSize: 13,
                background: currentConvId === conv.id ? 'rgba(91,124,153,0.08)' : 'transparent',
                borderLeft: currentConvId === conv.id ? '2px solid var(--accent-primary)' : '2px solid transparent',
              }}>
                <div style={{ flex: 1, overflow: 'hidden' }}>
                  {renamingId === conv.id ? (
                    <input ref={renameInputRef} value={renameValue} onChange={(e) => setRenameValue(e.target.value)}
                      onKeyDown={(e) => { if (e.key === 'Enter') handleRenameConfirm(); if (e.key === 'Escape') setRenamingId(null); }}
                      onBlur={handleRenameConfirm}
                      style={{ width: '100%', padding: '2px 4px', fontSize: 13, border: '1px solid var(--accent-primary)', borderRadius: 4, background: 'var(--bg-elevated)', color: 'var(--text-primary)', outline: 'none' }} autoFocus />
                  ) : (
                    <>
                      <div style={{ fontWeight: 500, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis', color: 'var(--text-primary)' }}>
                        {conv.name || t('settings:ai_untitled') }
                      </div>
                      <div style={{ fontSize: 11, color: 'var(--text-tertiary)', marginTop: 1 }}>{formatRelative(conv.updatedAt)}</div>
                    </>
                  )}
                </div>
                <button onClick={(e) => { e.stopPropagation(); handleRenameStart(conv.id, conv.name); }}
                  style={{ padding: 3, borderRadius: 4, border: 'none', background: 'transparent', cursor: 'pointer', color: 'var(--text-tertiary)', opacity: 0 }} className="sidebar-action-btn">
                  <Pencil size={12} />
                </button>
                <button onClick={(e) => { e.stopPropagation(); handleSoftDelete(conv.id); }}
                  style={{ padding: 3, borderRadius: 4, border: 'none', background: 'transparent', cursor: 'pointer', color: '#e74c3c', opacity: 0 }} className="sidebar-action-btn">
                  <Trash2 size={12} />
                </button>
              </div>
            ))}
          </div>

          {/* ── Trash entry (fixed at bottom) ── */}
          <div style={{ borderTop: '1px solid var(--border-subtle)', flexShrink: 0 }}>
            <button onClick={() => setShowTrash(!showTrash)}
              style={{ width: '100%', display: 'flex', alignItems: 'center', gap: 8, padding: '10px 12px', border: 'none', background: showTrash ? 'rgba(91,124,153,0.08)' : 'transparent', cursor: 'pointer', fontSize: 13, color: 'var(--text-tertiary)' }}>
              <Trash2 size={14} />
              <span>{t("settings:ai_trash")}</span>
              {trashList.length > 0 && <span style={{ marginLeft: 'auto', fontSize: 11, background: 'rgba(231,76,60,0.15)', color: '#e74c3c', padding: '1px 6px', borderRadius: 8 }}>{trashList.length}</span>}
            </button>
            {showTrash && (
              <div style={{ maxHeight: 200, overflowY: 'auto', borderTop: '1px solid var(--border-subtle)' }}>
                {trashList.length === 0 ? (
                  <p style={{ fontSize: 12, color: 'var(--text-tertiary)', textAlign: 'center', padding: '16px 12px' }}>{t('settings:ai_trash_empty')}</p>
                ) : (
                  trashList.map((conv) => (
                    <div key={conv.id} style={{ display: 'flex', alignItems: 'center', gap: 4, padding: '6px 12px', fontSize: 12 }}>
                      <div style={{ flex: 1, overflow: 'hidden', cursor: 'pointer' }} onClick={() => handleViewTrashConv(conv.id)}>
                        <div style={{ whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis', color: 'var(--text-secondary)' }}>
                          {conv.name || t('settings:ai_untitled') }
                        </div>
                        <div style={{ fontSize: 10, color: 'var(--text-tertiary)' }}>{conv.deletedAt ? formatRelative(conv.deletedAt) : ''}</div>
                      </div>
                      <button onClick={() => handleRestore(conv.id)} title="恢复"
                        style={{ padding: 3, borderRadius: 4, border: 'none', background: 'transparent', cursor: 'pointer', color: '#27ae60' }}>
                        <Undo2 size={12} />
                      </button>
                      {confirmPermanentDelete === conv.id ? (
                        <button onClick={() => handlePermanentDelete(conv.id)} title={t("settings:ai_confirm_delete")}
                          style={{ padding: '2px 6px', borderRadius: 4, border: '1px solid #e74c3c', background: '#e74c3c', cursor: 'pointer', color: 'white', fontSize: 10 }}>
                          {t('settings:ai_confirm_btn')}
                        </button>
                      ) : (
                        <button onClick={() => setConfirmPermanentDelete(conv.id)} title="永久删除"
                          style={{ padding: 3, borderRadius: 4, border: 'none', background: 'transparent', cursor: 'pointer', color: '#e74c3c' }}>
                          <Delete size={12} />
                        </button>
                      )}
                    </div>
                  ))
                )}
              </div>
            )}
          </div>
        </div>

        {/* ── Message Area (only messages scroll) ── */}
        <div style={{ flex: 1, display: 'flex', flexDirection: 'column', overflow: 'hidden', minWidth: 0 }}>
          <div style={{ flex: 1, overflowY: 'auto', padding: '4px 0', minHeight: 0 }}>
            {messages.length === 0 && (
              <div style={{ textAlign: 'center', padding: '64px 24px' }}>
                <MessageSquare size={40} style={{ marginBottom: 12, opacity: 0.25, color: 'var(--text-tertiary)' }} />
                <p style={{ fontSize: 14, color: 'var(--text-tertiary)' }}>
                  {t('settings:ai_chat_start')} · {activeProvider?.name} · {activeProvider?.model}
                </p>
              </div>
            )}
            {messages.map((msg, i) => (
              <div key={i} style={{ marginBottom: 4 }}>
                <div style={{ textAlign: 'center', fontSize: 11, color: 'var(--text-tertiary)', padding: '8px 0 2px' }}>
                  {formatTimestamp(msg.createdAt)}
                </div>
                <div style={{ display: 'flex', justifyContent: msg.role === 'user' ? 'flex-end' : 'flex-start', padding: '0 16px' }}>
                  <div style={{
                    maxWidth: msg.role === 'user' ? '70%' : '85%', padding: '10px 14px',
                    borderRadius: msg.role === 'user' ? '16px 16px 4px 16px' : '16px 16px 16px 4px',
                    background: msg.role === 'user' ? 'var(--accent-primary)' : msg.content.startsWith(t('settings:ai_chat_error_prefix')) ? 'rgba(231,76,60,0.12)' : 'var(--bg-elevated)',
                    color: msg.role === 'user' ? 'white' : 'var(--text-primary)',
                    fontSize: 14, lineHeight: 1.6,
                  }}>
                    {msg.role === 'user' ? (
                      <div style={{ whiteSpace: 'pre-wrap' }}>{msg.content}</div>
                    ) : msg.content.startsWith(t('settings:ai_chat_error_prefix')) ? (
                      <div style={{ color: '#e74c3c', whiteSpace: 'pre-wrap' }}>{msg.content}</div>
                    ) : (
                      <div className="markdown-content">
                        <ReactMarkdown rehypePlugins={[rehypeHighlight]}>{msg.content}</ReactMarkdown>
                      </div>
                    )}
                  </div>
                </div>
                <div style={{ display: 'flex', justifyContent: msg.role === 'user' ? 'flex-end' : 'flex-start', padding: '2px 20px' }}>
                  <button onClick={() => handleCopy(msg.content, i)}
                    style={{ padding: '2px 6px', borderRadius: 4, border: 'none', background: 'transparent', cursor: 'pointer', fontSize: 11, color: copiedIndex === i ? '#27ae60' : 'var(--text-tertiary)', display: 'flex', alignItems: 'center', gap: 3 }}>
                    {copiedIndex === i ? <><Check size={11} /> {t('settings:ai_copied')}</> : <><Copy size={11} /> {t('settings:ai_copy')}</>}
                  </button>
                </div>
              </div>
            ))}
            {isSending && (
              <div style={{ display: 'flex', justifyContent: 'flex-start', padding: '0 16px', marginTop: 4 }}>
                <div style={{ padding: '10px 14px', borderRadius: '16px 16px 16px 4px', background: 'var(--bg-elevated)', fontSize: 14, lineHeight: 1.6 }}>
                  <span className="typing-animation"><span className="dot">·</span><span className="dot">·</span><span className="dot">·</span></span>
                </div>
              </div>
            )}
            <div ref={chatEndRef} />
          </div>

          {/* ── Input Area (fixed at bottom) ── */}
          <div style={{ borderTop: '1px solid var(--border-subtle)', padding: '6px 12px 10px', flexShrink: 0 }}>
            <div style={{ fontSize: 11, color: 'var(--text-tertiary)', marginBottom: 4, display: 'flex', alignItems: 'center', gap: 4 }}>
              {activeProvider && (
                <>
                  <span style={{ color: 'var(--text-secondary)', fontWeight: 500 }}>{activeProvider.name}</span>
                  <span>·</span>
                  <span>{activeProvider.model}</span>
                  <span>·</span>
                  <span style={{
                    padding: '1px 5px', borderRadius: 3, fontSize: 10,
                    background: isLocal ? 'rgba(39,174,96,0.12)' : 'rgba(41,128,185,0.12)',
                    color: isLocal ? '#27ae60' : '#2980b9',
                  }}>{isLocal ? t('settings:ai_local') : t('settings:ai_cloud')}</span>
                  <span>·</span>
                  {checkingOnline ? (
                    <span style={{ color: 'var(--text-tertiary)' }}>
                      <RefreshCw size={10} style={{ verticalAlign: 'middle' }} /> {t('settings:ai_checking')}
                    </span>
                  ) : isOnline === true ? (
                    <span style={{ color: '#27ae60', display: 'flex', alignItems: 'center', gap: 2 }}>
                      <span style={{ width: 6, height: 6, borderRadius: '50%', background: '#27ae60', display: 'inline-block' }} />
                      {t('settings:ai_online')}
                    </span>
                  ) : isOnline === false ? (
                    <span style={{ color: '#e74c3c', display: 'flex', alignItems: 'center', gap: 2 }}>
                      <span style={{ width: 6, height: 6, borderRadius: '50%', background: '#e74c3c', display: 'inline-block' }} />
                      {t('settings:ai_offline')}
                      <button onClick={checkOnline} style={{ padding: 0, border: 'none', background: 'transparent', cursor: 'pointer', color: '#e74c3c' }}>
                        <RotateCw size={10} />
                      </button>
                    </span>
                  ) : null}
                </>
              )}
            </div>
            <div style={{ display: 'flex', gap: 8, alignItems: 'flex-end' }}>
              <textarea value={input} onChange={(e) => setInput(e.target.value)}
                onKeyDown={(e) => { if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); sendMessage(); } }}
                placeholder={t('settings:ai_chat_input_placeholder')} disabled={isSending} rows={2}
                style={{
                  flex: 1, padding: '8px 12px', fontSize: 14, lineHeight: 1.5, fontFamily: 'inherit',
                  border: '1px solid var(--border-subtle)', borderRadius: 10,
                  background: 'var(--bg-elevated)', color: 'var(--text-primary)', resize: 'none', outline: 'none',
                }} />
              <button onClick={sendMessage} disabled={isSending || !input.trim() || isOnline === false}
                title={isOnline === false ? t('settings:ai_model_offline') : ''}
                style={{
                  padding: '8px 16px', borderRadius: 10, border: 'none', height: 40,
                  background: isSending || !input.trim() || isOnline === false ? 'var(--border-subtle)' : 'var(--accent-primary)',
                  color: isSending || !input.trim() || isOnline === false ? 'var(--text-tertiary)' : 'white',
                  cursor: 'pointer',
                }}>
                {isSending ? <span style={{ display: 'flex', gap: 2 }}><span className="dot">·</span><span className="dot">·</span><span className="dot">·</span></span> : <Send size={16} />}
              </button>
            </div>
          </div>
        </div>

        {/* ── Floating card for trash conversation ── */}
        {floatingConv && (
          <div style={{ position: 'fixed', inset: 0, zIndex: 2000, display: 'flex', alignItems: 'center', justifyContent: 'center', background: 'rgba(0,0,0,0.25)' }}
            onClick={() => setFloatingConv(null)}>
            <div onClick={(e) => e.stopPropagation()} style={{
              width: 600, maxHeight: 400, background: 'var(--bg-elevated)', borderRadius: 16, boxShadow: 'var(--shadow-lg)',
              border: '1px solid var(--border-subtle)', display: 'flex', flexDirection: 'column', overflow: 'hidden',
            }}>
              <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', padding: '12px 16px', borderBottom: '1px solid var(--border-subtle)' }}>
                <span style={{ fontSize: 14, fontWeight: 600 }}>{floatingConv.name || t('settings:ai_deleted_conv')}</span>
                <button onClick={() => setFloatingConv(null)} style={{ padding: 4, borderRadius: 4, border: 'none', background: 'transparent', cursor: 'pointer', color: 'var(--text-tertiary)' }}>
                  <X size={16} />
                </button>
              </div>
              <div style={{ flex: 1, overflowY: 'auto', padding: '8px 12px' }}>
                {floatingConv.messages.map((msg, i) => (
                  <div key={i} style={{ marginBottom: 8 }}>
                    <div style={{ fontSize: 10, color: 'var(--text-tertiary)', textAlign: 'center', marginBottom: 2 }}>{formatTimestamp(msg.createdAt)}</div>
                    <div style={{ display: 'flex', justifyContent: msg.role === 'user' ? 'flex-end' : 'flex-start' }}>
                      <div style={{
                        maxWidth: '80%', padding: '8px 12px', borderRadius: 12, fontSize: 13,
                        background: msg.role === 'user' ? 'var(--accent-primary)' : 'var(--bg-toolbar)',
                        color: msg.role === 'user' ? 'white' : 'var(--text-primary)',
                      }}>
                        {msg.role === 'user' ? msg.content : (
                          <div className="markdown-content"><ReactMarkdown rehypePlugins={[rehypeHighlight]}>{msg.content}</ReactMarkdown></div>
                        )}
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </div>
        )}
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
        @keyframes blink { 0%, 80%, 100% { opacity: 0.3; } 40% { opacity: 1; } }
      `}</style>
    </AppShell>
  );
}