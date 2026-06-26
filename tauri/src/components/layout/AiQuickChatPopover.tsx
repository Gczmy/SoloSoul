import React, { useState, useRef, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { motion, AnimatePresence } from 'framer-motion';
import { MessageSquare, Plus, History, ArrowUpRight, X } from 'lucide-react';
import { setQuickChatOpen } from '@/lib/notification';
import { useAuthStore } from '@/stores/authStore';
import { LoadingPlaceholder } from '@/components/ui/LoadingPlaceholder';
import { ChatMessageList } from '@/components/llm/ChatMessageList';
import { ChatInputBar } from '@/components/llm/ChatInputBar';
import { ConversationHistory } from '@/components/llm/ConversationHistory';
import { UnconfiguredHint } from '@/components/llm/UnconfiguredHint';
import { useLlmChatCore } from '@/hooks/useLlmChatCore';
import { ICON_SIZE } from '@/lib/iconSizes';


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

  const [showHistory, setShowHistory] = useState(false);
  const outsideClickTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const scrollContainerRef = useRef<HTMLDivElement>(null);
  const chatEndRef = useRef<HTMLDivElement>(null);
  const cardRef = useRef<HTMLDivElement>(null);
  const historyRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    setQuickChatOpen(true);
    return () => setQuickChatOpen(false);
  }, []);

  const quickChatStorageKey = accountId ? `solosoul_quick_chat_conv_${accountId}` : null;

  const core = useLlmChatCore({
    includeSystemPrompt: true,
  });

  // Restore previous conversation from localStorage
  useEffect(() => {
    if (!core.loading && quickChatStorageKey) {
      const savedConvId = localStorage.getItem(quickChatStorageKey);
      if (savedConvId) {
        core.loadConversation(savedConvId).catch(() => {
          localStorage.removeItem(quickChatStorageKey!);
        });
      }
    }
    // Only run on mount / when loading completes
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [core.loading]);

  // Save conv ID to localStorage when it changes
  const prevConvIdRef = useRef<string | null>(null);
  useEffect(() => {
    if (core.currentConvId && core.currentConvId !== prevConvIdRef.current && quickChatStorageKey) {
      prevConvIdRef.current = core.currentConvId;
      localStorage.setItem(quickChatStorageKey, core.currentConvId);
    }
  }, [core.currentConvId, quickChatStorageKey]);

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
      1,
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

  // Scroll to bottom when messages change
  useEffect(() => {
    chatEndRef.current?.scrollIntoView({ behavior: 'auto' });
  }, [core.messages.length]);

  const handleNewConversation = () => {
    core.setMessages([]);
    core.setInput('');
    core.setCurrentConvId(null);
    if (quickChatStorageKey) localStorage.removeItem(quickChatStorageKey);
  };

  const loadConversation = async (convId: string) => {
    await core.loadConversation(convId);
    if (quickChatStorageKey) localStorage.setItem(quickChatStorageKey, convId);
  };

  const hoverBtnEnter = (e: React.MouseEvent<HTMLButtonElement>) => {
    e.currentTarget.style.background =
      'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
    e.currentTarget.style.color = 'var(--accent-primary)';
  };
  const hoverBtnLeave = (e: React.MouseEvent<HTMLButtonElement>) => {
    e.currentTarget.style.background = 'transparent';
    e.currentTarget.style.color = 'var(--text-secondary)';
  };

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
          <MessageSquare size={ICON_SIZE.md} style={{ color: 'var(--accent-primary)' }} />
          <span style={{ fontSize: 'var(--text-body-sm)', fontWeight: 600, color: 'var(--text-primary)' }}>
            {t('settings:ai_quick_chat_title')}
          </span>
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
          {core.isConfigured && core.isAiEnabled && (
            <>
              <button
                onClick={handleNewConversation}
                title={t('settings:ai_new_conv')}
                onMouseEnter={hoverBtnEnter}
                onMouseLeave={hoverBtnLeave}
                style={{
                  padding: 4,
                  borderRadius: 6,
                  border: 'none',
                  background: 'transparent',
                  cursor: 'pointer',
                  color: 'var(--text-secondary)',
                  transition: 'all 0.15s ease',
                }}
              >
                <Plus size={ICON_SIZE.sm} />
              </button>
              <button
                onClick={() => setShowHistory((prev) => !prev)}
                title={t('settings:ai_history')}
                onMouseEnter={(e) => {
                  e.currentTarget.style.background =
                    'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
                  if (!showHistory) e.currentTarget.style.color = 'var(--accent-primary)';
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.background = 'transparent';
                  if (!showHistory) e.currentTarget.style.color = 'var(--text-secondary)';
                }}
                style={{
                  padding: 4,
                  borderRadius: 6,
                  border: 'none',
                  background: 'transparent',
                  cursor: 'pointer',
                  color: showHistory ? 'var(--accent-primary)' : 'var(--text-secondary)',
                  transition: 'all 0.15s ease',
                }}
              >
                <History size={ICON_SIZE.sm} />
              </button>
            </>
          )}
          <button
            onClick={() => {
              onClose();
              navigate('/llm-chat');
            }}
            title={t('settings:ai_quick_chat_go_full')}
            onMouseEnter={hoverBtnEnter}
            onMouseLeave={hoverBtnLeave}
            style={{
              padding: 4,
              borderRadius: 6,
              border: 'none',
              background: 'transparent',
              cursor: 'pointer',
              color: 'var(--text-secondary)',
              transition: 'all 0.15s ease',
            }}
          >
            <ArrowUpRight size={ICON_SIZE.sm} />
          </button>
          <button
            onClick={onClose}
            title={t('common:close')}
            onMouseEnter={(e) => {
              e.currentTarget.style.background =
                'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
              e.currentTarget.style.color = 'var(--accent-primary)';
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.background = 'transparent';
              e.currentTarget.style.color = 'var(--text-tertiary)';
            }}
            style={{
              padding: 4,
              borderRadius: 6,
              border: 'none',
              background: 'transparent',
              cursor: 'pointer',
              color: 'var(--text-tertiary)',
              transition: 'all 0.15s ease',
            }}
          >
            <X size={ICON_SIZE.sm} />
          </button>
        </div>
      </div>

      {/* History dropdown */}
      <AnimatePresence>
        {showHistory && (
          <motion.div
            ref={historyRef}
            initial={{ opacity: 0, y: -6, scale: 0.96 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={{ opacity: 0, y: -6, scale: 0.96 }}
            transition={{ duration: 0.15, ease: 'easeOut' }}
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
              transformOrigin: 'top',
            }}
          >
            <ConversationHistory
              conversations={core.conversations}
              currentConvId={core.currentConvId}
              onSelect={(id) => {
                loadConversation(id);
                setShowHistory(false);
              }}
            />
          </motion.div>
        )}
      </AnimatePresence>

      {/* Body */}
      {core.loading ? (
        <LoadingPlaceholder variant="elevated" />
      ) : !core.isAiEnabled || !core.isConfigured ? (
        <UnconfiguredHint onClose={onClose} />
      ) : (
        <>
          <ChatMessageList
            messages={core.messages}
            isSending={core.isSending}
            copiedIndex={core.copiedIndex}
            onCopy={core.handleCopy}
            errorPrefix={t('settings:ai_chat_error_prefix')}
            activeProviderName={core.activeProvider?.name ?? ''}
            scrollContainerRef={scrollContainerRef}
            chatEndRef={chatEndRef}
          />
          <ChatInputBar
            input={core.input}
            onInputChange={core.setInput}
            isSending={core.isSending}
            onSend={core.sendMessage}
            activeProvider={
              core.activeProvider
                ? {
                    name: core.activeProvider.name,
                    model: core.activeProvider.model,
                    baseUrl: core.activeProvider.baseUrl,
                  }
                : null
            }
            checkingOnline={core.checkingOnline}
            isOnline={core.isOnline}
            isLocal={core.isLocal}
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
