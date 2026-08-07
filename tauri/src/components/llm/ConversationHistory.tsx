import { memo } from 'react';
import { useTranslation } from 'react-i18next';
import { MessageSquare } from 'lucide-react';
import { formatRelative } from '@/lib/time';
import { ICON_SIZE } from '@/lib/constants';
import type { ConversationSummary } from '@/types/llmChat';

interface ConversationHistoryProps {
  conversations: ConversationSummary[];
  currentConvId: string | null;
  onSelect: (id: string) => void;
}

export const ConversationHistory = memo(function ConversationHistory({
  conversations,
  currentConvId,
  onSelect,
}: ConversationHistoryProps) {
  const { t } = useTranslation(['settings', 'common']);

  if (conversations.length === 0) {
    return (
      <p
        style={{
          fontSize: 'var(--text-caption)',
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
        <ConversationHistoryItem
          key={conv.id}
          conv={conv}
          isActive={currentConvId === conv.id}
          onSelect={onSelect}
        />
      ))}
    </div>
  );
});

const ConversationHistoryItem = memo(function ConversationHistoryItem({
  conv,
  isActive,
  onSelect,
}: {
  conv: ConversationSummary;
  isActive: boolean;
  onSelect: (id: string) => void;
}) {
  const { t } = useTranslation(['settings', 'common']);
  return (
    <div
      onClick={() => onSelect(conv.id)}
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 8,
        padding: '7px 12px',
        cursor: 'pointer',
        fontSize: 'var(--text-caption)',
        background: isActive ? 'rgba(91,124,153,0.08)' : 'transparent',
      }}
    >
      <MessageSquare size={ICON_SIZE.xs} style={{ color: 'var(--text-tertiary)', flexShrink: 0 }} />
      <div style={{ flex: 1, overflow: 'hidden' }}>
        <div
          style={{
            whiteSpace: 'nowrap',
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            color: 'var(--text-primary)',
            fontWeight: isActive ? 500 : 400,
          }}
        >
          {conv.name || t('settings:ai_untitled')}
        </div>
        <div style={{ fontSize: 'var(--text-badge)', color: 'var(--text-tertiary)', marginTop: 1 }}>
          {formatRelative(conv.updatedAt)} · {conv.messageCount} {t('settings:ai_messages')}
        </div>
      </div>
      {isActive && (
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
  );
});
