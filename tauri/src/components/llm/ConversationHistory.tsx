import { useTranslation } from 'react-i18next';
import { MessageSquare } from 'lucide-react';
import { formatRelative } from '@/lib/time';

interface ConversationSummary {
  id: string;
  name: string;
  updatedAt: string;
  messageCount: number;
  deletedAt?: string;
}

interface ConversationHistoryProps {
  conversations: ConversationSummary[];
  currentConvId: string | null;
  onSelect: (id: string) => void;
}

export function ConversationHistory({
  conversations,
  currentConvId,
  onSelect,
}: ConversationHistoryProps) {
  const { t } = useTranslation(['settings', 'common']);

  if (conversations.length === 0) {
    return (
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
    );
  }

  return (
    <div style={{ padding: '6px 2px' }}>
      {conversations.map((conv) => (
        <div
          key={conv.id}
          onClick={() => onSelect(conv.id)}
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
              {formatRelative(conv.updatedAt)} · {conv.messageCount} {t('settings:ai_messages')}
            </div>
          </div>
          {currentConvId === conv.id && (
            <span
              style={{
                width: 6,
                height: 6,
                borderRadius: '50%',
                background: 'var(--accent-primary)',
                flexShrink: 1,
              }}
            />
          )}
        </div>
      ))}
    </div>
  );
}
