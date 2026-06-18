import { useTranslation } from 'react-i18next';
import { Send } from 'lucide-react';

interface ChatInputBarProps {
  input: string;
  onInputChange: (v: string) => void;
  isSending: boolean;
  onSend: () => void;
  activeProvider: { name: string; model: string; baseUrl: string } | null;
  checkingOnline: boolean;
  isOnline: boolean | null;
  isLocal: boolean;
}

export function ChatInputBar({
  input,
  onInputChange,
  isSending,
  onSend,
  activeProvider,
  checkingOnline,
  isOnline,
  isLocal,
}: ChatInputBarProps) {
  const { t } = useTranslation(['settings', 'common']);

  return (
    <div
      style={{
        borderTop: '1px solid var(--border-subtle)',
        padding: '6px 10px 8px',
        flexShrink: 0,
      }}
    >
      <div
        style={{
          fontSize: 10,
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
                padding: '0 4px',
                borderRadius: 3,
                fontSize: 9,
                background: isLocal ? 'rgba(39,174,96,0.12)' : 'rgba(41,128,185,0.12)',
                color: isLocal ? '#27ae60' : '#2980b9',
              }}
            >
              {isLocal ? t('settings:ai_local') : t('settings:ai_cloud')}
            </span>
            <span>·</span>
            {checkingOnline ? (
              <span style={{ color: 'var(--text-tertiary)' }}>{t('settings:ai_checking')}</span>
            ) : isOnline === true ? (
              <span style={{ color: '#27ae60', display: 'flex', alignItems: 'center', gap: 2 }}>
                <span
                  style={{
                    width: 5,
                    height: 5,
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
                    width: 5,
                    height: 5,
                    borderRadius: '50%',
                    background: '#e74c3c',
                    display: 'inline-block',
                  }}
                />
                {t('settings:ai_offline')}
              </span>
            ) : null}
          </>
        )}
      </div>
      <div style={{ display: 'flex', gap: 6, alignItems: 'flex-end' }}>
        <textarea
          value={input}
          onChange={(e) => onInputChange(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter' && !e.shiftKey) {
              e.preventDefault();
              onSend();
            }
          }}
          placeholder={t('settings:ai_chat_input_placeholder')}
          disabled={isSending}
          rows={2}
          style={{
            flex: 1,
            padding: '6px 10px',
            fontSize: 13,
            lineHeight: 1.5,
            fontFamily: 'inherit',
            border: '1px solid var(--border-subtle)',
            borderRadius: 8,
            background: 'var(--bg-elevated)',
            color: 'var(--text-primary)',
            resize: 'none',
            outline: 'none',
          }}
        />
        <button
          onClick={onSend}
          disabled={isSending || !input.trim() || isOnline === false}
          style={{
            padding: '6px 12px',
            borderRadius: 8,
            border: 'none',
            height: 36,
            background:
              isSending || !input.trim() || isOnline === false
                ? 'var(--border-subtle)'
                : 'var(--accent-primary)',
            color:
              isSending || !input.trim() || isOnline === false
                ? 'var(--text-tertiary)'
                : 'white',
            cursor: 'pointer',
          }}
        >
          {isSending ? (
            <span className="typing-animation">
              <span className="dot">·</span>
              <span className="dot">·</span>
              <span className="dot">·</span>
            </span>
          ) : (
            <Send size={14} />
          )}
        </button>
      </div>
    </div>
  );
}
