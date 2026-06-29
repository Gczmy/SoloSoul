import { memo } from 'react';
import { useRef, useEffect, type RefObject } from 'react';
import { useTranslation } from 'react-i18next';
import { Copy, Check, MessageSquare } from 'lucide-react';
import ReactMarkdown from 'react-markdown';
import rehypeHighlight from 'rehype-highlight';
import { formatTimestamp } from '@/lib/time';
import { ICON_SIZE } from '@/lib/iconSizes';
import { type ChatMsg } from '@/types/llmChat';

/** URL 协议白名单：仅允许安全的 HTTP(S)/mailto 链接 */
function allowedUrl(url: string): string {
  try {
    const parsed = new URL(url);
    if (['http:', 'https:', 'mailto:'].includes(parsed.protocol)) return url;
  } catch { /* ignore invalid URLs */ }
  return '';
}


interface ChatMessageListProps {
  messages: ChatMsg[];
  isSending: boolean;
  copiedIndex: number | null;
  onCopy: (content: string, index: number) => void;
  errorPrefix: string;
  activeProviderName: string;
  scrollContainerRef: RefObject<HTMLDivElement | null>;
  chatEndRef: RefObject<HTMLDivElement | null>;
}

export const ChatMessageList = memo(function ChatMessageList({
  messages,
  isSending,
  copiedIndex,
  onCopy,
  errorPrefix,
  activeProviderName,
  scrollContainerRef,
  chatEndRef,
}: ChatMessageListProps) {
  const { t } = useTranslation(['settings', 'common']);
  const hasScrolledRef = useRef(false);

  // Scroll to bottom: instant on first mount/load, smooth on subsequent updates
  useEffect(() => {
    const container = scrollContainerRef.current;
    if (!container) return;
    if (!hasScrolledRef.current) {
      container.scrollTop = container.scrollHeight;
      hasScrolledRef.current = true;
    } else {
      chatEndRef.current?.scrollIntoView({ behavior: 'smooth', block: 'end' });
    }
  }, [messages, scrollContainerRef, chatEndRef]);

  return (
    <div
      ref={scrollContainerRef}
      style={{ flex: 1, overflowY: 'auto', padding: '8px 0', minHeight: 0 }}
    >
      {messages.length === 0 && (
        <div style={{ textAlign: 'center', padding: '32px 16px' }}>
          <MessageSquare
            size={ICON_SIZE['3xl']}
            style={{ marginBottom: 8, opacity: 0.25, color: 'var(--text-tertiary)' }}
          />
          <p style={{ fontSize: 'var(--text-caption)', color: 'var(--text-tertiary)', margin: 0 }}>
            {t('settings:ai_chat_start')} · {activeProviderName}
          </p>
        </div>
      )}
      {messages.map((msg, i) => (
        <div
          key={msg.id ?? `msg-${msg.createdAt}-${msg.role}-${msg.content.slice(0, 16)}`}
          style={{ marginBottom: 6 }}
        >
          <div
            style={{
              textAlign: 'center',
              fontSize: 'var(--text-badge)',
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
                    : msg.content.startsWith(errorPrefix)
                      ? 'rgba(231,76,60,0.12)'
                      : 'var(--bg-toolbar)',
                color: msg.role === 'user' ? 'white' : 'var(--text-primary)',
                fontSize: 'var(--text-body-sm)',
                lineHeight: 1.55,
              }}
            >
              {msg.role === 'user' ? (
                <div style={{ whiteSpace: 'pre-wrap' }}>{msg.content}</div>
              ) : msg.content.startsWith(errorPrefix) ? (
                <div style={{ color: '#e74c3c', whiteSpace: 'pre-wrap' }}>{msg.content}</div>
              ) : (
                <div className="quick-chat-markdown">
                  <ReactMarkdown
                    rehypePlugins={[rehypeHighlight]}
                    urlTransform={allowedUrl}
                  >
                    {msg.content}
                  </ReactMarkdown>
                </div>
              )}
            </div>
          </div>
          {msg.role !== 'user' && (
            <div style={{ display: 'flex', justifyContent: 'flex-start', padding: '2px 14px' }}>
              <button
                onClick={() => onCopy(msg.content, i)}
                style={{
                  padding: '2px 6px',
                  borderRadius: 4,
                  border: 'none',
                  background: 'transparent',
                  cursor: 'pointer',
                  fontSize: 'var(--text-badge)',
                  color: copiedIndex === i ? '#27ae60' : 'var(--text-tertiary)',
                  display: 'flex',
                  alignItems: 'center',
                  gap: 3,
                }}
              >
                {copiedIndex === i ? (
                  <>
                    <Check size={ICON_SIZE['2xs']} /> {t('settings:ai_copied')}
                  </>
                ) : (
                  <>
                    <Copy size={ICON_SIZE['2xs']} /> {t('settings:ai_copy')}
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
              fontSize: 'var(--text-body-sm)',
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
  );
});
