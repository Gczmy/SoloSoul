import { useState, useRef, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { Input } from '@/components/ui/Input';
import { Button } from '@/components/ui/Button';
import { useAuthStore } from '@/stores/authStore';
import { MessageSquare, Settings, Send } from 'lucide-react';

interface LlmConfig {
  provider: string;
  apiKey: string;
  model: string;
  baseUrl?: string;
  enabled: boolean;
}

interface ChatMessage {
  role: 'user' | 'assistant' | 'system';
  content: string;
}

export function LlmChatPage() {
  const navigate = useNavigate();
  const { t } = useTranslation(['settings', 'common']);
  const accountId = useAuthStore((s) => s.currentAccount?.id);
  const [config, setConfig] = useState<LlmConfig | null>(null);
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [input, setInput] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const chatEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (accountId) {
      invoke<LlmConfig>('llm_get_config', { accountId })
        .then((cfg) => setConfig(cfg))
        .catch(() => setConfig(null));
    }
  }, [accountId]);

  useEffect(() => {
    chatEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  const sendMessage = async () => {
    const text = input.trim();
    if (!text || !config || !config.enabled) return;

    const userMsg: ChatMessage = { role: 'user', content: text };
    setMessages((prev) => [...prev, userMsg]);
    setInput('');
    setIsLoading(true);

    try {
      const allMessages = [...messages, userMsg];
      const response = await callProvider(config, allMessages);
      setMessages((prev) => [...prev, { role: 'assistant', content: response }]);
    } catch (e) {
      setMessages((prev) => [...prev, { role: 'assistant', content: `Error: ${String(e)}` }]);
    } finally {
      setIsLoading(false);
    }
  };

  if (!config) {
    return (
      <AppShell title={t('settings:items.ai_chat', { defaultValue: 'AI Chat' })}>
        <div style={{ maxWidth: 600, margin: '0 auto', textAlign: 'center', padding: '48px 24px' }}>
          <MessageSquare size={48} style={{ marginBottom: 16, opacity: 0.3 }} />
          <p style={{ color: 'var(--text-secondary)' }}>{t('common:loading')}</p>
        </div>
      </AppShell>
    );
  }

  if (!config.enabled) {
    return (
      <AppShell
        title={t('settings:items.ai_chat', { defaultValue: 'AI Chat' })}
        actions={
          <button onClick={() => navigate('/settings/llm')} style={{ padding: '8px 14px', borderRadius: 8, border: '1px solid var(--border-subtle)', background: 'transparent', cursor: 'pointer', fontSize: 13, color: 'var(--text-secondary)' }}>
            <Settings size={14} style={{ verticalAlign: 'middle', marginRight: 4 }} />
            Configure
          </button>
        }
      >
        <div style={{ maxWidth: 600, margin: '0 auto', textAlign: 'center', padding: '48px 24px' }}>
          <MessageSquare size={48} style={{ marginBottom: 16, opacity: 0.3, color: 'var(--text-tertiary)' }} />
          <h2 style={{ fontSize: 18, fontWeight: 600, margin: '0 0 8px' }}>AI Chat</h2>
          <p style={{ fontSize: 14, color: 'var(--text-secondary)', marginBottom: 16 }}>
            LLM features are disabled. Configure a provider to get started.
          </p>
          <button
            onClick={() => navigate('/settings/llm')}
            style={{ padding: '10px 24px', borderRadius: 8, border: 'none', background: 'var(--accent-primary)', color: 'white', fontSize: 14, cursor: 'pointer' }}
          >
            Configure LLM
          </button>
        </div>
      </AppShell>
    );
  }

  return (
    <AppShell
      title={`AI Chat · ${config.model}`}
      actions={
        <button
          onClick={() => navigate('/settings/llm')}
          title="LLM Settings"
          style={{ padding: 8, borderRadius: 8, border: '1px solid var(--border-subtle)', background: 'transparent', cursor: 'pointer', color: 'var(--text-secondary)' }}
        >
          <Settings size={16} />
        </button>
      }
    >
      <div style={{ maxWidth: 680, margin: '0 auto', display: 'flex', flexDirection: 'column', height: 'calc(100vh - 100px)' }}>
        {/* Messages */}
        <div style={{ flex: 1, overflowY: 'auto', padding: '0 4px' }}>
          {messages.length === 0 && (
            <div style={{ textAlign: 'center', padding: '48px 24px' }}>
              <MessageSquare size={40} style={{ marginBottom: 12, opacity: 0.25, color: 'var(--text-tertiary)' }} />
              <p style={{ fontSize: 14, color: 'var(--text-tertiary)' }}>
                Start a conversation · {config.provider} · {config.model}
              </p>
            </div>
          )}
          {messages.map((msg, i) => (
            <div
              key={i}
              style={{
                display: 'flex', justifyContent: msg.role === 'user' ? 'flex-end' : 'flex-start',
                marginBottom: 12,
              }}
            >
              <div
                style={{
                  maxWidth: '80%', padding: '10px 14px', borderRadius: 12,
                  background: msg.role === 'user' ? 'var(--accent-primary)' : 'var(--bg-toolbar)',
                  color: msg.role === 'user' ? 'white' : 'var(--text-primary)',
                  fontSize: 14, lineHeight: 1.5, whiteSpace: 'pre-wrap',
                  borderBottomRightRadius: msg.role === 'user' ? 4 : 12,
                  borderBottomLeftRadius: msg.role === 'user' ? 12 : 4,
                }}
              >
                {msg.content}
              </div>
            </div>
          ))}
          {isLoading && (
            <div style={{ display: 'flex', justifyContent: 'flex-start', marginBottom: 12 }}>
              <div style={{ padding: '10px 14px', borderRadius: 12, background: 'var(--bg-toolbar)', fontSize: 14 }}>
                <span style={{ opacity: 0.5 }}>Thinking...</span>
              </div>
            </div>
          )}
          <div ref={chatEndRef} />
        </div>

        {/* Input */}
        <div style={{ display: 'flex', gap: 8, padding: '12px 0', borderTop: '1px solid var(--border-subtle)' }}>
          <Input
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={(e) => { if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); sendMessage(); } }}
            placeholder="Type a message... (Enter to send)"
            style={{ flex: 1 }}
          />
          <button
            onClick={sendMessage}
            disabled={isLoading || !input.trim()}
            style={{ padding: '8px 16px', borderRadius: 8, border: 'none', background: 'var(--accent-primary)', color: 'white', cursor: isLoading ? 'not-allowed' : 'pointer', opacity: isLoading ? 0.6 : 1 }}
          >
            <Send size={16} />
          </button>
        </div>
      </div>
    </AppShell>
  );
}

async function callProvider(config: LlmConfig, messages: ChatMessage[]): Promise<string> {
  const apiMessages = messages.map((m) => ({ role: m.role, content: m.content }));

  switch (config.provider) {
    case 'ollama': {
      const base = config.baseUrl || 'http://localhost:11434';
      const resp = await fetch(`${base}/api/chat`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ model: config.model, messages: apiMessages, stream: false }),
      });
      const data = await resp.json();
      return data.message?.content || 'No response';
    }
    case 'openai': {
      const resp = await fetch('https://api.openai.com/v1/chat/completions', {
        method: 'POST',
        headers: { 'Authorization': `Bearer ${config.apiKey}`, 'Content-Type': 'application/json' },
        body: JSON.stringify({ model: config.model, messages: apiMessages }),
      });
      const data = await resp.json();
      return data.choices?.[0]?.message?.content || 'No response';
    }
    case 'anthropic': {
      const systemMsg = apiMessages.find((m) => m.role === 'system');
      const chatMsgs = apiMessages.filter((m) => m.role !== 'system');
      const resp = await fetch('https://api.anthropic.com/v1/messages', {
        method: 'POST',
        headers: { 'x-api-key': config.apiKey, 'anthropic-version': '2023-06-01', 'Content-Type': 'application/json' },
        body: JSON.stringify({ model: config.model, max_tokens: 4096, system: systemMsg?.content, messages: chatMsgs }),
      });
      const data = await resp.json();
      return data.content?.[0]?.text || 'No response';
    }
    default:
      throw new Error(`Unknown provider: ${config.provider}`);
  }
}
