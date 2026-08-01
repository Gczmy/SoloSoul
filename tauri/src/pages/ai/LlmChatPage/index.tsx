import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { AppShell } from '@/components/layout/AppShell';
import { PageContainer } from '@/components/layout/PageContainer';

import { LoadingPlaceholder } from '@/components/ui/LoadingPlaceholder';
import { useMemo } from 'react';
import { MessageSquare, Settings, BarChart3, Info, Bot, MessageCircle, History } from 'lucide-react';
import buttonStyles from '@/components/ui/Button.module.css';
import { ConversationSidebar } from '@/components/llm/ConversationSidebar';
import { MessageArea } from '@/components/llm/MessageArea';
import { TrashConversationCard } from '@/components/llm/TrashConversationCard';
import { useLlmChat } from './useLlmChat';
import { ICON_SIZE } from '@/lib/constants';
import { PageGuideButton } from '@/components/guide/PageGuideButton';

export { useLlmChat } from './useLlmChat';
export { type Conversation, type ConversationSummary } from './useLlmChat';

export function LlmChatPage() {
  const navigate = useNavigate();
  const { t } = useTranslation(['settings', 'common']);
  const chat = useLlmChat();

  const aiChatGuidePages = useMemo(
    () => [
      {
        icon: Info,
        title: t('common:guide_ai_chat_title') ?? 'AI Chat Guide',
        steps: [
          {
            icon: Bot,
            title: t('common:guide_ai_chat_step1_title') ?? 'Select Provider',
            description:
              t('common:guide_ai_chat_step1_desc') ??
              'Choose a configured AI provider or add a new one in settings. Local and remote providers are supported.',
          },
          {
            icon: MessageCircle,
            title: t('common:guide_ai_chat_step2_title') ?? 'Start Conversation',
            description:
              t('common:guide_ai_chat_step2_desc') ??
              'Create a new conversation or continue an existing one. Ask questions based on your vault data.',
          },
          {
            icon: History,
            title: t('common:guide_ai_chat_step3_title') ?? 'Manage Conversations',
            description:
              t('common:guide_ai_chat_step3_desc') ??
              'Rename, delete, or archive conversations. Review token usage and model statistics.',
          },
        ],
        helpLinks: [
          {
            title: t('common:guide_help_ai_chat') ?? 'AI Chat',
            description:
              t('common:guide_help_ai_chat_desc') ??
              'Chat with AI using your local vault data',
            href: '/help?id=ai_chat',
          },
        ],
      },
    ],
    [t],
  );

  if (chat.loading) {
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

  if (!chat.isAiEnabled || !chat.isConfigured) {
    return (
      <AppShell
        title={t('settings:ai_chat')}
        onBack={() => navigate('/home')}
        actions={
        <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
          <PageGuideButton pages={aiChatGuidePages} />
          <button
            type="button"
            className={`interactive-toolbar ${buttonStyles.hideLabelOnMobile}`}
            onClick={() => navigate('/settings/llm', { state: { from: '/llm-chat' } })}
            style={{
              fontSize: 'var(--text-caption)',
              padding: '6px 12px',
              borderRadius: 6,
              borderWidth: 1,
              borderStyle: 'solid',
              cursor: 'pointer',
              fontFamily: 'inherit',
              fontWeight: 500,
              display: 'inline-flex',
              alignItems: 'center',
              gap: 4,
            }}
          >
            <Settings size={ICON_SIZE.sm} />{' '}
            <span className={buttonStyles.label}>{t('settings:ai_chat_configure')}</span>
          </button>
        </div>
      }
      >
        <PageContainer variant="small" gap="default">
          <div style={{ textAlign: 'center', paddingTop: 48, paddingBottom: 48 }}>
            <MessageSquare
              size={ICON_SIZE['5xl']}
              style={{ marginBottom: 16, opacity: 0.3, color: 'var(--text-tertiary)' }}
            />
            <h2 style={{ fontSize: 'var(--text-md)', fontWeight: 600, margin: '0 0 8px' }}>
              {t('settings:ai_chat')}
            </h2>
            <p
              style={{
                fontSize: 'var(--text-sm)',
                color: 'var(--text-secondary)',
                marginBottom: 16,
              }}
            >
              {t('settings:ai_chat_disabled')}
            </p>
            <button
              type="button"
              onClick={() => navigate('/settings/llm', { state: { from: '/llm-chat' } })}
              className="interactive-toolbar"
              style={{
                fontSize: 'var(--text-caption)',
                padding: '6px 12px',
                borderRadius: 6,
                borderWidth: 1,
                borderStyle: 'solid',
                cursor: 'pointer',
                fontFamily: 'inherit',
                fontWeight: 500,
              }}
            >
              {t('settings:ai_chat_configure')}
            </button>
          </div>
        </PageContainer>
      </AppShell>
    );
  }

  return (
    <AppShell
      title={t('settings:ai_chat')}
      onBack={() => navigate('/home')}
      actions={
        <div style={{ display: 'flex', gap: 8 }}>
          <PageGuideButton pages={aiChatGuidePages} />
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
              <BarChart3 size={ICON_SIZE.md} />
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
              <Settings size={ICON_SIZE.md} />
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
          conversations={chat.conversations}
          trashList={chat.trashList}
          currentConvId={chat.currentConvId}
          showTrash={chat.showTrash}
          onNewConversation={chat.handleNewConversation}
          onLoadConversation={chat.loadConversation}
          onSoftDelete={chat.handleSoftDelete}
          onRename={chat.handleRename}
          onToggleTrash={() => chat.setShowTrash(!chat.showTrash)}
          onRestore={chat.handleRestore}
          confirmPermanentDeleteId={chat.confirmPermanentDelete}
          onRequestPermanentDelete={(id) => {
            if (chat.confirmPermanentDelete === id) {
              chat.handlePermanentDelete(id);
            } else {
              chat.setConfirmPermanentDelete(id);
            }
          }}
          onViewTrashConv={chat.handleViewTrashConv}
        />

        <MessageArea
          messages={chat.messages}
          input={chat.input}
          isSending={chat.isSending}
          isOnline={chat.isOnline}
          checkingOnline={chat.checkingOnline}
          activeProvider={chat.activeProvider}
          isLocal={chat.isLocal}
          copiedIndex={chat.copiedIndex}
          onInputChange={chat.setInput}
          onSend={chat.sendMessage}
          onCopy={chat.handleCopy}
          onCheckOnline={chat.checkOnline}
        />

        <TrashConversationCard
          floatingConv={chat.floatingConv}
          copiedIndex={chat.copiedIndex}
          onClose={() => chat.setFloatingConv(null)}
          onCopy={chat.handleCopy}
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
        @media (hover: hover) and (pointer: fine) {
          .tooltip-btn:hover::after { opacity: 1; }
        }
        .conv-item {
          border: 1px solid transparent;
          border-radius: 8px;
          margin: 2px 8px;
          transition: transform 0.15s ease, border-color 0.15s ease, box-shadow 0.15s ease;
        }
        @media (hover: hover) and (pointer: fine) {
          .conv-item:hover {
            transform: translateY(-2px);
            border-color: var(--accent-primary);
            box-shadow: 0 6px 16px rgba(0,0,0,0.08);
          }
        }
        @keyframes blink { 0%, 80%, 100% { opacity: 0.3; } 40% { opacity: 1; } }
      `}</style>
    </AppShell>
  );
}
