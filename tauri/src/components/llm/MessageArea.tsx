import { useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { Send, RotateCw, RefreshCw, MessageSquare } from 'lucide-react';
import { ChatMessageBubble, type ChatMsg } from '@/pages/ai/ChatMessageBubble';
import { ICON_SIZE } from '@/lib/constants';

interface ActiveProvider {
  name: string;
  model: string;
  baseUrl: string;
}

interface MessageAreaProps {
  messages: ChatMsg[];
  input: string;
  isSending: boolean;
  isOnline: boolean | null;
  checkingOnline: boolean;
  activeProvider: ActiveProvider | null;
  isLocal: boolean;
  copiedIndex: number | null;
  onInputChange: (value: string) => void;
  onSend: () => void;
  onCopy: (content: string, index: number) => void;
  onCheckOnline: () => void;
  t_prefix?: string;
}

export function MessageArea({
  messages,
  input,
  isSending,
  isOnline,
  checkingOnline,
  activeProvider,
  isLocal,
  copiedIndex,
  onInputChange,
  onSend,
  onCopy,
  onCheckOnline,
}: MessageAreaProps) {
  const { t } = useTranslation('settings');
  const chatEndRef = useRef<HTMLDivElement>(null);

  return (
    <div
      style={{
        flex: 1,
        display: 'flex',
        flexDirection: 'column',
        overflow: 'hidden',
        minWidth: 0,
      }}
    >
      <div style={{ flex: 1, overflowY: 'auto', padding: '4px 0', minHeight: 0 }}>
        {messages.length === 0 && (
          <div style={{ textAlign: 'center', padding: '64px 24px' }}>
            <MessageSquare
              size={ICON_SIZE['4xl']}
              style={{ marginBottom: 12, opacity: 0.25, color: 'var(--text-tertiary)' }}
            />
            <p style={{ fontSize: 'var(--text-body)', color: 'var(--text-tertiary)' }}>
              {t('settings:ai_chat_start')} · {activeProvider?.name} · {activeProvider?.model}
            </p>
          </div>
        )}
        {messages.map((msg, i) => (
          <ChatMessageBubble
            key={i}
            msg={msg}
            isCopied={copiedIndex === i}
            onCopy={() => onCopy(msg.content, i)}
            copyLabel={t('settings:ai_copy')}
            copiedLabel={t('settings:ai_copied')}
          />
        ))}
        {isSending && (
          <div
            style={{
              display: 'flex',
              justifyContent: 'flex-start',
              padding: '0 16px',
              marginTop: 4,
            }}
          >
            <div
              style={{
                padding: '10px 14px',
                borderRadius: '16px 16px 16px 4px',
                background: 'var(--bg-elevated)',
                fontSize: 'var(--text-body)',
                lineHeight: 1.6,
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
        <div ref={chatEndRef} data-chat-end />
      </div>

      {/* Input Area */}
      <div
        style={{
          borderTop: '1px solid var(--border-subtle)',
          padding: '6px 12px 10px',
          flexShrink: 0,
        }}
      >
        <div
          style={{
            fontSize: 'var(--text-badge)',
            color: 'var(--text-tertiary)',
            marginBottom: 4,
            display: 'flex',
            alignItems: 'center',
            gap: 4,
          }}
        >
          {activeProvider && (
            <>
              <span style={{ color: 'var(--text-secondary)', fontWeight: 500 }}>
                {activeProvider.name}
              </span>
              <span>·</span>
              <span>{activeProvider.model}</span>
              <span>·</span>
              <span
                style={{
                  padding: '1px 5px',
                  borderRadius: 3,
                  fontSize: 'var(--text-badge)',
                  background: isLocal ? 'rgba(39,174,96,0.12)' : 'rgba(41,128,185,0.12)',
                  color: isLocal ? '#27ae60' : '#2980b9',
                }}
              >
                {isLocal ? t('settings:ai_local') : t('settings:ai_cloud')}
              </span>
              <span>·</span>
              {checkingOnline ? (
                <span style={{ color: 'var(--text-tertiary)' }}>
                  <RefreshCw size={ICON_SIZE['2xs']} style={{ verticalAlign: 'middle' }} />{' '}
                  {t('settings:ai_checking')}
                </span>
              ) : isOnline === true ? (
                <span style={{ color: '#27ae60', display: 'flex', alignItems: 'center', gap: 2 }}>
                  <span
                    style={{
                      width: 6,
                      height: 6,
                      borderRadius: '50%',
                      background: '#27ae60',
                      display: 'inline-block',
                    }}
                  />
                  {t('settings:ai_online')}
                </span>
              ) : isOnline === false ? (
                <span style={{ color: '#e74c3c', display: 'flex', alignItems: 'center', gap: 2 }}>
                  <span
                    style={{
                      width: 6,
                      height: 6,
                      borderRadius: '50%',
                      background: '#e74c3c',
                      display: 'inline-block',
                    }}
                  />
                  {t('settings:ai_offline')}
                  <button
                    onClick={onCheckOnline}
                    style={{
                      padding: 0,
                      border: 'none',
                      background: 'transparent',
                      cursor: 'pointer',
                      color: '#e74c3c',
                    }}
                  >
                    <RotateCw size={ICON_SIZE['2xs']} />
                  </button>
                </span>
              ) : null}
            </>
          )}
        </div>
        <div style={{ display: 'flex', gap: 8, alignItems: 'flex-end' }}>
          <textarea
            value={input}
            onChange={(e) => onInputChange(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter' && !e.shiftKey) {
                e.preventDefault();
                onSend();
              }
            }}
            placeholder={String(t('settings:ai_chat_input_placeholder'))}
            disabled={isSending}
            rows={2}
            style={{
              flex: 1,
              padding: '8px 12px',
              fontSize: 'var(--text-body)',
              lineHeight: 1.5,
              fontFamily: 'inherit',
              border: '1px solid var(--border-subtle)',
              borderRadius: 10,
              background: 'var(--bg-elevated)',
              color: 'var(--text-primary)',
              resize: 'none',
              outline: 'none',
            }}
          />
          <button
            onClick={onSend}
            disabled={isSending || !input.trim() || isOnline === false}
            title={isOnline === false ? String(t('settings:ai_model_offline')) : ''}
            style={{
              padding: '8px 16px',
              borderRadius: 10,
              border: 'none',
              height: 40,
              background:
                isSending || !input.trim() || isOnline === false
                  ? 'var(--border-subtle)'
                  : 'var(--accent-primary)',
              color:
                isSending || !input.trim() || isOnline === false ? 'var(--text-tertiary)' : 'white',
              cursor: 'pointer',
            }}
          >
            {isSending ? (
              <span style={{ display: 'flex', gap: 2 }} />
            ) : (
              <Send size={ICON_SIZE.md} />
            )}
          </button>
        </div>
      </div>
    </div>
  );
}
