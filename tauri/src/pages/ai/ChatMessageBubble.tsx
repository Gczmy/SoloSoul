import { memo } from 'react';
import { Copy, Check } from 'lucide-react';
import ReactMarkdown from 'react-markdown';
import rehypeHighlight from 'rehype-highlight';
import { formatTimestamp } from '@/lib/time';

export interface ChatMsg {
  role: string;
  content: string;
  createdAt: string;
  isError?: boolean;
}

interface ChatMessageBubbleProps {
  msg: ChatMsg;
  isCopied: boolean;
  onCopy: () => void;
  copyLabel: string;
  copiedLabel: string;
  /** Compact variant used in the floating quick-chat panel. */
  variant?: 'default' | 'compact';
}

export const ChatMessageBubble = memo(function ChatMessageBubble({
  msg,
  isCopied,
  onCopy,
  copyLabel,
  copiedLabel,
  variant = 'default',
}: ChatMessageBubbleProps) {
  const isUser = msg.role === 'user';
  const isCompact = variant === 'compact';

  return (
    <div style={{ marginBottom: isCompact ? 8 : 4 }}>
      <div
        style={{
          textAlign: 'center',
          fontSize: isCompact ? 10 : 11,
          color: 'var(--text-tertiary)',
          padding: isCompact ? '0 0 2px' : '8px 0 2px',
          marginBottom: isCompact ? 2 : undefined,
        }}
      >
        {formatTimestamp(msg.createdAt)}
      </div>
      <div
        style={{
          display: 'flex',
          justifyContent: isUser ? 'flex-end' : 'flex-start',
          padding: isCompact ? 0 : '0 16px',
        }}
      >
        <div
          style={{
            maxWidth: isUser ? (isCompact ? '80%' : '70%') : isCompact ? '90%' : '85%',
            padding: isCompact ? '8px 12px' : '10px 14px',
            borderRadius: isUser ? '16px 16px 4px 16px' : '16px 16px 16px 4px',
            background: isUser
              ? 'var(--accent-primary)'
              : msg.isError
                ? 'rgba(231,76,60,0.12)'
                : 'var(--bg-elevated)',
            color: isUser ? 'white' : 'var(--text-primary)',
            fontSize: 'var(--text-body)',
            lineHeight: 1.6,
          }}
        >
          {isUser ? (
            <div style={{ whiteSpace: 'pre-wrap' }}>{msg.content}</div>
          ) : msg.isError ? (
            <div style={{ color: '#e74c3c', whiteSpace: 'pre-wrap' }}>{msg.content}</div>
          ) : (
            <div className="markdown-content">
              <ReactMarkdown rehypePlugins={[rehypeHighlight]}>{msg.content}</ReactMarkdown>
            </div>
          )}
        </div>
      </div>
      {!isCompact && (
        <div
          style={{
            display: 'flex',
            justifyContent: isUser ? 'flex-end' : 'flex-start',
            padding: '2px 20px',
          }}
        >
          <button
            onClick={onCopy}
            style={{
              padding: '2px 6px',
              borderRadius: 4,
              border: 'none',
              background: 'transparent',
              cursor: 'pointer',
              fontSize: 'var(--text-badge)',
              color: isCopied ? '#27ae60' : 'var(--text-tertiary)',
              display: 'flex',
              alignItems: 'center',
              gap: 3,
            }}
          >
            {isCopied ? (
              <>
                <Check size={11} /> {copiedLabel}
              </>
            ) : (
              <>
                <Copy size={11} /> {copyLabel}
              </>
            )}
          </button>
        </div>
      )}
    </div>
  );
});
