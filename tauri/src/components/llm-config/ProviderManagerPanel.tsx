import { useState, useRef, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { Settings, Plus } from 'lucide-react';
import { Card } from '@/components/ui/Card';
import { Input } from '@/components/ui/Input';
import { Button } from '@/components/ui/Button';
import { LoadingPlaceholder } from '@/components/ui/LoadingPlaceholder';
import { ICON_SIZE } from '@/lib/constants';

import type { ProviderConfig } from '@/types/llmProvider';
export type { ProviderConfig };
import { isOllama } from '@/types/llmChat';
interface ProviderManagerPanelProps {
  providers: ProviderConfig[];
  activeId: string;
  loading: boolean;
  accountId: string | undefined;
  onSetActive: (id: string) => void;
  onSaveProvider: (provider: ProviderConfig) => Promise<void>;
  onDeleteProvider: (id: string) => void;
  onTestConnection: (provider: ProviderConfig, accountId: string) => Promise<string>;
}

export function ProviderManagerPanel({
  providers,
  activeId,
  loading,
  accountId,
  onSetActive,
  onSaveProvider,
  onDeleteProvider,
  onTestConnection,
}: ProviderManagerPanelProps) {
  const { t } = useTranslation(['settings', 'common']);
  const [editingProvider, setEditingProvider] = useState<ProviderConfig | null>(null);
  const [testResult, setTestResult] = useState<string | null>(null);
  const [testing, setTesting] = useState(false);
  const [savingProvider, setSavingProvider] = useState(false);
  const providerFormRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (editingProvider && providerFormRef.current) {
      providerFormRef.current.scrollIntoView({ behavior: 'smooth', block: 'start' });
    }
  }, [editingProvider]);

  const handleTestConnection = async () => {
    if (!editingProvider || !accountId) return;
    setTesting(true);
    setTestResult(null);
    try {
      const result = await onTestConnection(editingProvider, accountId);
      setTestResult(result);
    } catch (e) {
      setTestResult(t('settings:llm_test_fail') + ' ' + String(e).slice(0, 120));
    } finally {
      setTesting(false);
    }
  };

  const handleSaveProvider = async () => {
    if (!editingProvider) return;
    setSavingProvider(true);
    try {
      await onSaveProvider(editingProvider);
      setEditingProvider(null);
      setTestResult(null);
    } catch {
      /* handled by parent toast */
    } finally {
      setSavingProvider(false);
    }
  };

  const handleAddCustom = () => {
    setEditingProvider({
      id: 'custom_' + Date.now(),
      name: '',
      baseUrl: '',
      model: '',
      isEnabled: false,
      isBuiltIn: false,
      apiKey: '',
      apiType: 'openAI',
    });
  };

  return (
    <>
      <Card>
        <h3 style={{ fontSize: 'var(--text-body)', fontWeight: 600, marginBottom: 12 }}>
          {t('settings:ai_service_providers')}
        </h3>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
          {loading ? (
            <LoadingPlaceholder variant="elevated" minHeight={60} />
          ) : (
            providers.map((p) => (
              <div
                key={p.id}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 8,
                  padding: '8px 10px',
                  borderRadius: 8,
                  background: activeId === p.id ? 'rgba(91,124,153,0.08)' : 'var(--bg-toolbar)',
                  border:
                    activeId === p.id
                      ? '1px solid var(--accent-primary)'
                      : '1px solid var(--border-subtle)',
                  cursor: 'pointer',
                  fontSize: 'var(--text-body-sm)',
                }}
                onClick={() => onSetActive(p.id)}
              >
                <input
                  type="radio"
                  checked={activeId === p.id}
                  onChange={() => onSetActive(p.id)}
                  style={{ accentColor: 'var(--accent-primary)' }}
                />
                <div style={{ flex: 1 }}>
                  <span style={{ fontWeight: 500 }}>{p.name}</span>
                  <span
                    style={{
                      marginLeft: 6,
                      fontSize: 'var(--text-badge)',
                      color: 'var(--text-tertiary)',
                    }}
                  >
                    {p.model}
                  </span>
                  {p.isBuiltIn && (
                    <span
                      style={{
                        marginLeft: 4,
                        fontSize: 'var(--text-badge)',
                        padding: '1px 4px',
                        borderRadius: 3,
                        background: 'var(--bg-elevated)',
                        color: 'var(--text-tertiary)',
                      }}
                    >
                      {t('settings:llm_builtin_badge')}
                    </span>
                  )}
                </div>
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    setEditingProvider({ ...p });
                  }}
                  style={{
                    padding: 4,
                    borderRadius: 4,
                    border: 'none',
                    background: 'transparent',
                    cursor: 'pointer',
                    color: 'var(--text-tertiary)',
                  }}
                >
                  <Settings size={ICON_SIZE.sm} />
                </button>
                {!p.isBuiltIn && (
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      onDeleteProvider(p.id);
                    }}
                    style={{
                      padding: 4,
                      borderRadius: 4,
                      border: 'none',
                      background: 'transparent',
                      cursor: 'pointer',
                      color: '#e74c3c',
                      fontSize: 'var(--text-body)',
                    }}
                  >
                    ×
                  </button>
                )}
              </div>
            ))
          )}
        </div>
        <Button variant="secondary" size="sm" onClick={handleAddCustom} style={{ marginTop: 10 }}>
          <Plus size={ICON_SIZE.sm} style={{ marginRight: 4 }} /> {t('settings:llm_add_custom')}
        </Button>
      </Card>

      {editingProvider && (
        <div ref={providerFormRef}>
          <Card>
            <h3 style={{ fontSize: 'var(--text-body)', fontWeight: 600, marginBottom: 12 }}>
              {editingProvider.isBuiltIn
                ? t('settings:llm_configure') + ' ' + editingProvider.name
                : t('settings:llm_custom_provider')}
            </h3>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
              <Input
                label={t('settings:llm_provider_name')}
                value={editingProvider.name}
                onChange={(e) =>
                  setEditingProvider((p) => (p ? { ...p, name: e.target.value } : null))
                }
                disabled={editingProvider.isBuiltIn}
              />
              <Input
                label={t('settings:llm_base_url')}
                value={editingProvider.baseUrl}
                onChange={(e) =>
                  setEditingProvider((p) => (p ? { ...p, baseUrl: e.target.value } : null))
                }
              />
              {/* P035: 云端 provider 隐私标注——对话内容将发送至该第三方服务 */}
              {editingProvider.baseUrl && !isOllama(editingProvider.baseUrl) && (
                <div
                  style={{
                    fontSize: 'var(--text-caption)',
                    color: '#e67e22',
                    lineHeight: 1.5,
                    marginTop: -6,
                  }}
                >
                  {t('settings:llm_cloud_privacy_hint', { name: editingProvider.name })}
                </div>
              )}
              <Input
                label={t('settings:llm_model')}
                value={editingProvider.model}
                onChange={(e) =>
                  setEditingProvider((p) => (p ? { ...p, model: e.target.value } : null))
                }
              />
              <div>
                <label
                  style={{
                    fontSize: 'var(--text-body-sm)',
                    fontWeight: 500,
                    color: 'var(--text-secondary)',
                    marginBottom: 4,
                    display: 'block',
                  }}
                >
                  {t('settings:llm_api_type')}
                </label>
                <select
                  value={editingProvider.apiType}
                  onChange={(e) =>
                    setEditingProvider((p) =>
                      p ? { ...p, apiType: e.target.value as 'openAI' | 'anthropic' } : null,
                    )
                  }
                  style={{
                    width: '100%',
                    padding: '10px 14px',
                    fontSize: 'var(--text-body)',
                    border: '1px solid var(--border-subtle)',
                    borderRadius: 8,
                    background: 'var(--bg-elevated)',
                    color: 'var(--text-primary)',
                    fontFamily: 'inherit',
                    outline: 'none',
                  }}
                >
                  <option value="openAI">OpenAI Compatible</option>
                  <option value="anthropic">Anthropic</option>
                </select>
              </div>
              <Input
                label={t('settings:llm_api_key')}
                type="password"
                value={editingProvider.apiKey}
                onChange={(e) =>
                  setEditingProvider((p) => (p ? { ...p, apiKey: e.target.value } : null))
                }
                placeholder={
                  editingProvider.apiKey === '••••••••'
                    ? t('settings:llm_api_key_unchanged')
                    : t('settings:llm_api_key_enter')
                }
              />
              {testResult && (
                <div
                  style={{
                    fontSize: 'var(--text-caption)',
                    padding: '6px 10px',
                    borderRadius: 6,
                    background: 'rgba(128,128,128,0.08)',
                    color: testResult.startsWith(t('settings:llm_test_ok')) ? '#27ae60' : '#e74c3c',
                  }}
                >
                  {testResult}
                </div>
              )}
              <div style={{ display: 'flex', gap: 8 }}>
                <Button variant="secondary" onClick={handleTestConnection} loading={testing}>
                  {t('settings:llm_test_connection')}
                </Button>
                <Button onClick={handleSaveProvider} loading={savingProvider}>
                  {t('common:save')}
                </Button>
                <Button
                  variant="secondary"
                  onClick={() => {
                    setEditingProvider(null);
                    setTestResult(null);
                  }}
                >
                  {t('common:cancel')}
                </Button>
              </div>
            </div>
          </Card>
        </div>
      )}
    </>
  );
}
