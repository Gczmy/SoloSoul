import { useState, useEffect } from 'react';
import { useNavigate, useLocation } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { Input } from '@/components/ui/Input';
import { Button } from '@/components/ui/Button';
import { useAuthStore } from '@/stores/authStore';
import { useToastError } from '@/hooks/useToastError';
import { Settings, Plus, BarChart3 } from 'lucide-react';

interface ProviderConfig { id: string; name: string; baseUrl: string; model: string; isEnabled: boolean; isBuiltIn: boolean; apiKey: string; apiType: 'openAI' | 'anthropic'; }
interface AiFeatures { chat: boolean; smartFill: boolean; commandGen: boolean; naturalLanguageSearch: boolean; }

export function LlmConfigPage() {
  const navigate = useNavigate();
  const location = useLocation();
  const backPath = (location.state as { from?: string } | null)?.from || '/settings';
  const { t } = useTranslation(['settings', 'common']);
  const accountId = useAuthStore((s) => s.currentAccount?.id);
  const { onError, onSuccess } = useToastError();

  const [providers, setProviders] = useState<ProviderConfig[]>([]);
  const [activeId, setActiveId] = useState<string>('');
  const [features, setFeatures] = useState<AiFeatures>({ chat: false, smartFill: false, commandGen: false, naturalLanguageSearch: false });
  const [includeSystemPrompt, setIncludeSystemPrompt] = useState(true);
  const [hasAcceptedRisk, setHasAcceptedRisk] = useState(false);
  const [loading, setLoading] = useState(true);
  const [showRiskDialog, setShowRiskDialog] = useState(false);
  const [riskChecked, setRiskChecked] = useState(false);
  const [editingProvider, setEditingProvider] = useState<ProviderConfig | null>(null);
  const [testResult, setTestResult] = useState<string | null>(null);
  const [testing, setTesting] = useState(false);
  const [savingProvider, setSavingProvider] = useState(false);

  useEffect(() => {
    if (!accountId) return;
    Promise.all([
      invoke<ProviderConfig[]>('llm_get_providers', { accountId }),
      invoke<{ activeProviderId?: string; aiFeaturesEnabled?: AiFeatures; includeSystemPrompt?: boolean; hasAcceptedRisk?: boolean }>('llm_get_config', { accountId }),
    ]).then(([provs, cfg]) => {
      setProviders(provs);
      if (cfg.activeProviderId) setActiveId(cfg.activeProviderId);
      if (cfg.aiFeaturesEnabled) setFeatures(cfg.aiFeaturesEnabled);
      if (cfg.includeSystemPrompt !== undefined) setIncludeSystemPrompt(cfg.includeSystemPrompt);
      if (cfg.hasAcceptedRisk) setHasAcceptedRisk(true);
    }).catch(() => {}).finally(() => setLoading(false));
  }, [accountId]);

  const handleSetActive = async (id: string) => {
    if (!accountId) return;
    setActiveId(id);
    await invoke('llm_set_active_provider', { accountId, providerId: id }).catch(() => {});
  };

  const handleFeatureToggle = async (key: keyof AiFeatures) => {
    const next = { ...features, [key]: !features[key] };
    if (!hasAcceptedRisk && next[key]) { setShowRiskDialog(true); setRiskChecked(false); return; }
    setFeatures(next);
    if (accountId) await invoke('llm_set_ai_features', { accountId, features: next }).catch(() => {});
  };

  const handleAcceptRisk = async () => {
    if (!accountId) return;
    await invoke('llm_accept_risk', { accountId }).catch(() => {});
    setHasAcceptedRisk(true);
    setShowRiskDialog(false);
    const next = { ...features, chat: true };
    setFeatures(next);
    await invoke('llm_set_ai_features', { accountId, features: next }).catch(() => {});
  };

  const handleSystemPromptToggle = async () => {
    const next = !includeSystemPrompt;
    setIncludeSystemPrompt(next);
    if (accountId) await invoke('llm_set_system_prompt_switch', { accountId, enabled: next }).catch(() => {});
  };

  const handleSaveProvider = async () => {
    if (!editingProvider || !accountId) return;
    setSavingProvider(true);
    try {
      await invoke('llm_save_provider', { accountId, provider: editingProvider });
      setProviders((prev) => {
        const idx = prev.findIndex((p) => p.id === editingProvider.id);
        const updated = { ...editingProvider, apiKey: editingProvider.apiKey ? '••••••••' : '' };
        if (idx >= 0) { const n = [...prev]; n[idx] = updated; return n; }
        return [...prev, updated];
      });
      setEditingProvider(null);
      onSuccess(t('common:success'));
    } catch (e) { onError(e, t('common:error')); }
    finally { setSavingProvider(false); }
  };

  const handleTestConnection = async () => {
    if (!editingProvider) return;
    setTesting(true); setTestResult(null);
    try {
      let key = editingProvider.apiKey;
      if (key === '••••••••' && accountId) { key = await invoke<string>('llm_get_api_key', { accountId, providerId: editingProvider.id }); }
      const result = await invoke<string>('llm_test_provider', { baseUrl: editingProvider.baseUrl, apiKey: key, model: editingProvider.model, apiType: editingProvider.apiType });
      setTestResult(t('settings:llm_test_ok') + ' "' + result.slice(0, 80) + '"');
    } catch (e) {
      setTestResult(t('settings:llm_test_fail') + ' ' + String(e).slice(0, 120));
    } finally { setTesting(false); }
  };

  const handleDeleteProvider = async (id: string) => {
    if (!accountId || !confirm(t('common:confirm'))) return;
    await invoke('llm_delete_provider', { accountId, providerId: id }).catch(() => {});
    setProviders((prev) => prev.filter((p) => p.id !== id));
    if (activeId === id) setActiveId('');
  };

  const handleAddCustom = () => {
    setEditingProvider({ id: 'custom_' + Date.now(), name: '', baseUrl: '', model: '', isEnabled: false, isBuiltIn: false, apiKey: '', apiType: 'openAI' });
  };

  if (loading) return <AppShell title={t('settings:llm_config')} onBack={() => navigate(backPath)}><p>{t('common:loading')}</p></AppShell>;

  return (
    <AppShell title={t('settings:llm_config')} onBack={() => navigate(backPath)}>
      <div style={{ maxWidth: 560, margin: '0 auto', display: 'flex', flexDirection: 'column', gap: 16 }}>
        {!hasAcceptedRisk && (
          <Card>
            <p style={{ fontSize: 12, color: 'var(--text-tertiary)', lineHeight: 1.5 }}>
              <span style={{ color: '#e67e22' }}>⚠</span> {t('settings:ai_risk_notice')}
            </p>
          </Card>
        )}

        <Card>
          <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 12 }}>{t('settings:ai_features')}</h3>
          {(['chat', 'smartFill', 'commandGen', 'naturalLanguageSearch'] as const).map((key) => (
            <label key={key} style={{ display: 'flex', alignItems: 'center', gap: 10, padding: '6px 0', cursor: key === 'chat' ? 'pointer' : 'not-allowed', fontSize: 13, opacity: key === 'chat' ? 1 : 0.5 }}>
              <input type="checkbox" checked={features[key]} onChange={() => key === 'chat' && handleFeatureToggle(key)} disabled={key !== 'chat'} style={{ accentColor: 'var(--accent-primary)' }} />
              {t('settings:ai_' + key)}
              {key !== 'chat' && <span style={{ fontSize: 11, color: 'var(--text-tertiary)', marginLeft: 4 }}>({t('settings:ai_in_development')})</span>}
            </label>
          ))}
        </Card>

        <Card>
          <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 12 }}>{t('settings:ai_system_prompt_title')}</h3>
          <label style={{ display: 'flex', alignItems: 'center', gap: 10, padding: '6px 0', cursor: 'pointer', fontSize: 13 }}>
            <input type="checkbox" checked={includeSystemPrompt} onChange={handleSystemPromptToggle} style={{ accentColor: 'var(--accent-primary)' }} />
            {t('settings:ai_system_prompt_software')}
          </label>
          <p style={{ fontSize: 11, color: 'var(--text-tertiary)', marginTop: 4, paddingLeft: 26 }}>
            {t('settings:ai_system_prompt_desc')}
          </p>
        </Card>

        <Card>
          <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 12 }}>{t('settings:ai_service_providers')}</h3>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
            {providers.map((p) => (
              <div key={p.id} style={{ display: 'flex', alignItems: 'center', gap: 8, padding: '8px 10px', borderRadius: 8, background: activeId === p.id ? 'rgba(91,124,153,0.08)' : 'var(--bg-toolbar)', border: activeId === p.id ? '1px solid var(--accent-primary)' : '1px solid var(--border-subtle)', cursor: 'pointer', fontSize: 13 }}
                onClick={() => handleSetActive(p.id)}>
                <input type="radio" checked={activeId === p.id} onChange={() => handleSetActive(p.id)} style={{ accentColor: 'var(--accent-primary)' }} />
                <div style={{ flex: 1 }}>
                  <span style={{ fontWeight: 500 }}>{p.name}</span>
                  <span style={{ marginLeft: 6, fontSize: 11, color: 'var(--text-tertiary)' }}>{p.model}</span>
                  {p.isBuiltIn && <span style={{ marginLeft: 4, fontSize: 10, padding: '1px 4px', borderRadius: 3, background: 'var(--bg-elevated)', color: 'var(--text-tertiary)' }}>{t('settings:llm_builtin_badge')}</span>}
                </div>
                <button onClick={(e) => { e.stopPropagation(); setEditingProvider({ ...p }); }} style={{ padding: 4, borderRadius: 4, border: 'none', background: 'transparent', cursor: 'pointer', color: 'var(--text-tertiary)' }}><Settings size={14} /></button>
                {!p.isBuiltIn && <button onClick={(e) => { e.stopPropagation(); handleDeleteProvider(p.id); }} style={{ padding: 4, borderRadius: 4, border: 'none', background: 'transparent', cursor: 'pointer', color: '#e74c3c', fontSize: 14 }}>×</button>}
              </div>
            ))}
          </div>
          <Button variant="secondary" size="sm" onClick={handleAddCustom} style={{ marginTop: 10 }}>
            <Plus size={14} style={{ marginRight: 4 }} /> {t('settings:llm_add_custom')}
          </Button>
        </Card>

        <Card interactive onClick={() => navigate('/settings/llm/stats', { state: { from: '/settings/llm' } })}>
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
              <BarChart3 size={20} color="var(--accent-primary)" />
              <div>
                <span style={{ fontSize: 14, fontWeight: 500 }}>使用统计</span>
                <div style={{ fontSize: 11, color: 'var(--text-tertiary)', marginTop: 1 }}>查看 LLM Token 消耗和对话统计</div>
              </div>
            </div>
            <span style={{ color: 'var(--text-tertiary)', fontSize: 18 }}>›</span>
          </div>
        </Card>

        {editingProvider && (
          <Card>
            <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 12 }}>{editingProvider.isBuiltIn ? t('settings:llm_configure') + ' ' + editingProvider.name : t('settings:llm_custom_provider')}</h3>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
              <Input label={t('settings:llm_provider_name')} value={editingProvider.name} onChange={(e) => setEditingProvider((p) => p ? { ...p, name: e.target.value } : null)} disabled={editingProvider.isBuiltIn} />
              <Input label={t('settings:llm_base_url')} value={editingProvider.baseUrl} onChange={(e) => setEditingProvider((p) => p ? { ...p, baseUrl: e.target.value } : null)} />
              <Input label={t('settings:llm_model')} value={editingProvider.model} onChange={(e) => setEditingProvider((p) => p ? { ...p, model: e.target.value } : null)} />
              <div>
                <label style={{ fontSize: 13, fontWeight: 500, color: 'var(--text-secondary)', marginBottom: 4, display: 'block' }}>{t('settings:llm_api_type')}</label>
                <select value={editingProvider.apiType} onChange={(e) => setEditingProvider((p) => p ? { ...p, apiType: e.target.value as 'openAI' | 'anthropic' } : null)}
                  style={{ width: '100%', padding: '10px 14px', fontSize: 14, border: '1px solid var(--border-subtle)', borderRadius: 8, background: 'var(--bg-elevated)', color: 'var(--text-primary)', fontFamily: 'inherit', outline: 'none' }}>
                  <option value="openAI">OpenAI Compatible</option>
                  <option value="anthropic">Anthropic</option>
                </select>
              </div>
              <Input label={t('settings:llm_api_key')} type="password" value={editingProvider.apiKey} onChange={(e) => setEditingProvider((p) => p ? { ...p, apiKey: e.target.value } : null)}
                placeholder={editingProvider.apiKey === '••••••••' ? t('settings:llm_api_key_unchanged') : t('settings:llm_api_key_enter')} />
              {testResult && <div style={{ fontSize: 12, padding: '6px 10px', borderRadius: 6, background: 'rgba(128,128,128,0.08)', color: testResult.startsWith(t('settings:llm_test_ok')) ? '#27ae60' : '#e74c3c' }}>{testResult}</div>}
              <div style={{ display: 'flex', gap: 8 }}>
                <Button variant="secondary" onClick={handleTestConnection} loading={testing}>{t('settings:llm_test_connection')}</Button>
                <Button onClick={handleSaveProvider} loading={savingProvider}>{t('common:save')}</Button>
                <Button variant="secondary" onClick={() => { setEditingProvider(null); setTestResult(null); }}>{t('common:cancel')}</Button>
              </div>
            </div>
          </Card>
        )}

        {showRiskDialog && (
          <div style={{ position: 'fixed', inset: 0, zIndex: 3000, display: 'flex', alignItems: 'center', justifyContent: 'center', background: 'rgba(0,0,0,0.45)', backdropFilter: 'blur(6px)' }}>
            <div style={{ background: 'var(--bg-elevated)', borderRadius: 16, padding: '28px 32px', maxWidth: 400, width: '90%', boxShadow: 'var(--shadow-lg)', border: '1px solid var(--border-subtle)' }}>
              <h3 style={{ fontSize: 17, fontWeight: 600, marginBottom: 12, display: 'flex', alignItems: 'center', gap: 8 }}><span style={{ fontSize: 20 }}>⚠</span> {t('settings:ai_risk_title')}</h3>
              <p style={{ fontSize: 13, color: 'var(--text-secondary)', lineHeight: 1.6, marginBottom: 16 }}>{t('settings:ai_risk_desc')}</p>
              <ul style={{ fontSize: 12, color: 'var(--text-secondary)', lineHeight: 1.8, paddingLeft: 16, marginBottom: 16 }}>
                <li>{t('settings:ai_risk_li1')}</li>
                <li>{t('settings:ai_risk_li2')}</li>
                <li>{t('settings:ai_risk_li3')}</li>
                <li>{t('settings:ai_risk_li4')}</li>
              </ul>
              <label style={{ display: 'flex', alignItems: 'center', gap: 8, cursor: 'pointer', marginBottom: 16, fontSize: 13 }}>
                <input type="checkbox" checked={riskChecked} onChange={() => setRiskChecked(!riskChecked)} style={{ accentColor: 'var(--accent-primary)' }} />
                {t('settings:ai_risk_agree')}
              </label>
              <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end' }}>
                <Button variant="secondary" onClick={() => setShowRiskDialog(false)}>{t('common:cancel')}</Button>
                <Button onClick={handleAcceptRisk} disabled={!riskChecked}>{t('settings:ai_enable')}</Button>
              </div>
            </div>
          </div>
        )}
      </div>
    </AppShell>
  );
}
