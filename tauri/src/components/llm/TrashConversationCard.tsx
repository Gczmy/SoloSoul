import { useTranslation } from 'react-i18next';
import { X } from 'lucide-react';
import { ChatMessageBubble, type ChatMsg } from '@/pages/ai/ChatMessageBubble';
import { ICON_SIZE } from '@/lib/constants';

interface Conversation {
  id: string;
  name: string;
  messages: ChatMsg[];
}

interface TrashConversationCardProps {
  floatingConv: Conversation | null;
  copiedIndex: number | null;
  onClose: () => void;
  onCopy: (content: string, index: number) => void;
}

export function TrashConversationCard({
  floatingConv,
  copiedIndex,
  onClose,
  onCopy,
}: TrashConversationCardProps) {
  const { t } = useTranslation('settings');
  if (!floatingConv) return null;

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        zIndex: 2000,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        background: 'rgba(0,0,0,0.25)',
      }}
      onClick={onClose}
    >
      <div
        onClick={(e) => e.stopPropagation()}
        style={{
          width: 600,
          maxHeight: 400,
          background: 'var(--bg-elevated)',
          borderRadius: 16,
          boxShadow: 'var(--shadow-lg)',
          border: '1px solid var(--border-subtle)',
          display: 'flex',
          flexDirection: 'column',
          overflow: 'hidden',
        }}
      >
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            padding: '12px 16px',
            borderBottom: '1px solid var(--border-subtle)',
          }}
        >
          <span style={{ fontSize: 'var(--text-body)', fontWeight: 600 }}>
            {floatingConv.name || t('settings:ai_deleted_conv')}
          </span>
          <button
            onClick={onClose}
            style={{
              padding: 4,
              borderRadius: 4,
              border: 'none',
              background: 'transparent',
              cursor: 'pointer',
              color: 'var(--text-tertiary)',
            }}
          >
            <X size={ICON_SIZE.md} />
          </button>
        </div>
        <div style={{ flex: 1, overflowY: 'auto', padding: '8px 12px' }}>
          {floatingConv.messages.map((msg, i) => (
            <ChatMessageBubble
              key={i}
              msg={msg}
              variant="compact"
              isCopied={copiedIndex === i}
              onCopy={() => onCopy(msg.content, i)}
              copyLabel={t('settings:ai_copy')}
              copiedLabel={t('settings:ai_copied')}
            />
          ))}
        </div>
      </div>
    </div>
  );
}
