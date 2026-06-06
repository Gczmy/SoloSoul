import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { Input } from '@/components/ui/Input';
import { Button } from '@/components/ui/Button';
import { useAuthStore } from '@/stores/authStore';
import { useToastError } from '@/hooks/useToastError';

interface LlmConfig {
  provider: string;
  apiKey: string;
  model: string;
  baseUrl?: string;
  enabled: boolean;
}

const PROVIDERS = [
  { id: 'ollama', label: 'Ollama (Local)', desc: 'Runs locally, no API key needed' },
  { id: 'openai', label: 'OpenAI', desc: 'GPT-4o, GPT-4, etc.' },
  { id: 'anthropic', label: 'Anthropic', desc: 'Claude Opus, Sonnet, Haiku' },
];

const DEFAULT_MODELS: Record<string, string> = {
  ollama: 'llama3',
  openai: 'gpt-4o',
  anthropic: 'claude-sonnet-4-6',
};

export function LlmConfigPage() {
  const navigate = useNavigate();
  const { t } = useTranslation(['settings', 'common']);
  const accountId = useAuthStore((s) => s.currentAccount?.id);
  const { onError, onSuccess } = useToastError();

  const [config, setConfig] = useState<LlmConfig>({
    provider: 'ollama',
    apiKey: '',
    model: 'llama3',
    baseUrl: 'http://localhost:11434',
    enabled: false,
  });
  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);

  useEffect(() => {
    if (accountId) {
      invoke<LlmConfig>('llm_get_config', { accountId })
        .then((cfg) => setConfig(cfg))
        .catch(() => { /* use defaults */ })
        .finally(() => setIsLoading(false));
    }
  }, [accountId]);

  const handleSave = async () => {
    if (!accountId) return;
    setIsSaving(true);
    try {
      await invoke('llm_update_config', { accountId, config });
      onSuccess(t('common:success'));
    } catch (e) {
      onError(e, t('common:error'));
    } finally {
      setIsSaving(false);
    }
  };

  const handleProviderChange = (provider: string) => {
    setConfig((c) => ({
      ...c,
      provider,
      model: DEFAULT_MODELS[provider] || c.model,
      baseUrl: provider === 'ollama' ? 'http://localhost:11434' : undefined,
    }));
  };

  if (isLoading) {
    return <AppShell title="LLM Config" onBack={() => navigate('/settings')}><p>Loading...</p></AppShell>;
  }

  return (
    <AppShell title="LLM Config" onBack={() => navigate('/settings')}>
      <div style={{ maxWidth: 520, margin: '0 auto', display: 'flex', flexDirection: 'column', gap: 16 }}>
        {/* Enable toggle */}
        <Card>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <div>
              <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 4 }}>{t('common:enable', { defaultValue: 'Enable LLM' })}</h3>
              <p style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>
                {t('settings:ai_chat_description', { defaultValue: 'Enable AI-powered features.' })}
              </p>
            </div>
            <label style={{ position: 'relative', display: 'inline-block', width: 44, height: 24 }}>
              <input
                type="checkbox"
                checked={config.enabled}
                onChange={(e) => setConfig((c) => ({ ...c, enabled: e.target.checked }))}
                style={{ opacity: 0, width: 0, height: 0 }}
              />
              <span style={{
                position: 'absolute', cursor: 'pointer', inset: 0,
                background: config.enabled ? 'var(--accent-primary)' : 'var(--border-subtle)',
                borderRadius: 12, transition: '0.2s',
              }}>
                <span style={{
                  position: 'absolute', height: 18, width: 18, left: config.enabled ? 23 : 3,
                  bottom: 3, background: 'white', borderRadius: '50%', transition: '0.2s',
                }} />
              </span>
            </label>
          </div>
        </Card>

        {/* Provider selection */}
        <Card>
          <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 12 }}>Provider</h3>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
            {PROVIDERS.map((p) => (
              <label
                key={p.id}
                style={{
                  display: 'flex', alignItems: 'center', gap: 10, cursor: 'pointer',
                  padding: '10px 12px', borderRadius: 8,
                  background: config.provider === p.id ? 'var(--state-selected)' : 'var(--bg-toolbar)',
                  border: config.provider === p.id ? '1px solid var(--accent-primary)' : '1px solid var(--border-subtle)',
                }}
              >
                <input
                  type="radio"
                  name="provider"
                  checked={config.provider === p.id}
                  onChange={() => handleProviderChange(p.id)}
                  style={{ accentColor: 'var(--accent-primary)' }}
                />
                <div>
                  <div style={{ fontSize: 14, fontWeight: 500 }}>{p.label}</div>
                  <div style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>{p.desc}</div>
                </div>
              </label>
            ))}
          </div>
        </Card>

        {/* Model + URL */}
        <Card>
          <Input
            label="Model"
            value={config.model}
            onChange={(e) => setConfig((c) => ({ ...c, model: e.target.value }))}
            placeholder={DEFAULT_MODELS[config.provider]}
          />
          {config.provider === 'ollama' && (
            <div style={{ marginTop: 12 }}>
              <Input
                label="Base URL"
                value={config.baseUrl || ''}
                onChange={(e) => setConfig((c) => ({ ...c, baseUrl: e.target.value }))}
                placeholder="http://localhost:11434"
              />
            </div>
          )}
          {(config.provider === 'openai' || config.provider === 'anthropic') && (
            <div style={{ marginTop: 12 }}>
              <Input
                label="API Key"
                type="password"
                value={config.apiKey}
                onChange={(e) => setConfig((c) => ({ ...c, apiKey: e.target.value }))}
                placeholder={config.provider === 'openai' ? 'sk-...' : 'sk-ant-...'}
              />
            </div>
          )}
        </Card>

        <Button onClick={handleSave} loading={isSaving}>{t('common:save')}</Button>
      </div>
    </AppShell>
  );
}
