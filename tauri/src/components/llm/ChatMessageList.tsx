import { memo, useState } from 'react';
import { useRef, useEffect, type RefObject } from 'react';
import { useTranslation } from 'react-i18next';
import { Copy, Check, MessageSquare } from 'lucide-react';
import { SafeMarkdown } from '@/components/ui/SafeMarkdown';
import rehypeHighlight from 'rehype-highlight';
import { formatTimestamp } from '@/lib/time';
import { ICON_SIZE } from '@/lib/constants';
import { type ChatMsg } from '@/types/llmChat';

/** URL 协议白名单：仅允许安全的 HTTP(S)/mailto 链接 */
function allowedUrl(url: string): string {
  try {
    const parsed = new URL(url);
    if (['http:', 'https:', 'mailto:'].includes(parsed.protocol)) return url;
  } catch {
    /* ignore invalid URLs */
  }
  return '';
}

interface ChatMessageItemProps {
  msg: ChatMsg;
  index: number;
  copiedIndex: number | null;
  onCopy: (content: string, index: number) => void;
  errorPrefix: string;
}

/**
 * 单条消息气泡。memo 化后流式期间只有最后一条 assistant 消息的
 * content 变化，其余消息（对象引用稳定）跳过重渲染，避免每次 token
 * 都对整段会话重新 Markdown 解析 + 语法高亮。
 */
const ChatMessageItem = memo(function ChatMessageItem({
  msg,
  index,
  copiedIndex,
  onCopy,
  errorPrefix,
}: ChatMessageItemProps) {
  const { t } = useTranslation(['settings', 'common']);
  return (
    <div style={{ marginBottom: 6 }}>
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
            <SafeMarkdown
              rehypePlugins={[rehypeHighlight]}
              urlTransform={allowedUrl}
              className="quick-chat-markdown"
            >
              {msg.content}
            </SafeMarkdown>
          )}
        </div>
      </div>
      {msg.role !== 'user' && (
        <div style={{ display: 'flex', justifyContent: 'flex-start', padding: '2px 14px' }}>
          <button
            onClick={() => onCopy(msg.content, index)}
            style={{
              padding: '2px 6px',
              borderRadius: 4,
              border: 'none',
              background: 'transparent',
              cursor: 'pointer',
              fontSize: 'var(--text-badge)',
              color: copiedIndex === index ? '#27ae60' : 'var(--text-tertiary)',
              display: 'flex',
              alignItems: 'center',
              gap: 3,
            }}
          >
            {copiedIndex === index ? (
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
  );
});

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

// P026: 长会话分页——仅挂载末尾窗口，避免数百条含 Markdown 的消息全部进 DOM；
// 「加载更早消息」按钮按步长展开更早的消息。
const INITIAL_MESSAGE_WINDOW = 50;
const LOAD_EARLIER_STEP = 50;

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
  const [visibleCount, setVisibleCount] = useState(INITIAL_MESSAGE_WINDOW);

  // 会话切换（消息条数骤降）时重置窗口；流式增长时保持已展开的窗口不变。
  useEffect(() => {
    if (messages.length < visibleCount) {
      setVisibleCount(INITIAL_MESSAGE_WINDOW);
    }
  }, [messages.length, visibleCount]);

  // 末尾窗口切片；index 保持原始数组下标（copiedIndex 语义不变）。
  const start = Math.max(0, messages.length - visibleCount);
  const visibleMessages = messages.slice(start);

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
      {start > 0 && (
        <div style={{ textAlign: 'center', padding: '10px 0 4px' }}>
          <button
            onClick={() =>
              setVisibleCount((c) => Math.min(c + LOAD_EARLIER_STEP, messages.length))
            }
            style={{
              padding: '4px 12px',
              borderRadius: 999,
              border: '1px solid var(--border-subtle)',
              background: 'transparent',
              color: 'var(--text-secondary)',
              cursor: 'pointer',
              fontSize: 'var(--text-caption)',
            }}
          >
            {t('settings:ai_load_earlier', {
              count: Math.min(LOAD_EARLIER_STEP, start),
            })}
          </button>
        </div>
      )}
      {visibleMessages.map((msg, j) => (
        <ChatMessageItem
          key={msg.id ?? `msg-${msg.createdAt}-${msg.role}-${msg.content.slice(0, 16)}`}
          msg={msg}
          index={start + j}
          copiedIndex={copiedIndex}
          onCopy={onCopy}
          errorPrefix={errorPrefix}
        />
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
