import { useNavigate } from 'react-router-dom';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { useSettingsStore } from '@/stores/settingsStore';
import { useAuthStore } from '@/stores/authStore';
import { applyTheme } from '@/lib/theme';
import { useTranslation } from 'react-i18next';
import type { AccentPreset } from '@/types';
import type { SupportedLang } from '@/lib/i18n';

const ACCENT_OPTIONS: { value: AccentPreset; label: string; color: string }[] = [
  { value: 'ocean', label: 'Ocean', color: '#5B7C99' },
  { value: 'amber', label: 'Amber', color: '#C4925C' },
  { value: 'forest', label: 'Forest', color: '#5B8C6F' },
  { value: 'rose', label: 'Rose', color: '#B06B7A' },
];

const LANG_OPTIONS: { value: SupportedLang; label: string }[] = [
  { value: 'zh-CN', label: '中文（简体）' },
  { value: 'en-US', label: 'English' },
];

export function AppearanceSettingsPage() {
  const navigate = useNavigate();
  const currentAccount = useAuthStore((s) => s.currentAccount);
  const { settings, updateSetting } = useSettingsStore();
  const accountId = currentAccount?.id || '';
  const { t } = useTranslation();

  const handlePresetChange = (preset: 'light' | 'dark' | 'system') => {
    updateSetting(accountId, 'theme', preset);
    applyTheme({
      preset: preset === 'dark' ? 'warm-stone-dark' :
              preset === 'light' ? 'warm-stone-light' : 'system',
      accentColor: settings.accentColor as AccentPreset,
      backgroundType: 'solid',
      backgroundValue: '',
    });
  };

  const handleAccentChange = (accent: AccentPreset) => {
    updateSetting(accountId, 'accentColor', accent);
    applyTheme({
      preset: settings.theme === 'dark' ? 'warm-stone-dark' :
              settings.theme === 'light' ? 'warm-stone-light' : 'system',
      accentColor: accent,
      backgroundType: 'solid',
      backgroundValue: '',
    });
  };

  return (
    <AppShell title="Appearance" onBack={() => navigate('/settings')}>
      <div style={{ maxWidth: 480, margin: '0 auto', display: 'flex', flexDirection: 'column', gap: 16 }}>
        {/* Theme preset */}
        <Card>
          <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 12 }}>Theme</h3>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
            {(['light', 'dark', 'system'] as const).map((preset) => (
              <label
                key={preset}
                style={{
                  display: 'flex', alignItems: 'center', gap: 10, cursor: 'pointer',
                  padding: '8px 10px', borderRadius: 8,
                  background: settings.theme === preset ? 'var(--state-selected)' : 'transparent',
                  fontSize: 14,
                }}
              >
                <input
                  type="radio"
                  name="theme"
                  checked={settings.theme === preset}
                  onChange={() => handlePresetChange(preset)}
                  style={{ accentColor: 'var(--accent-primary)' }}
                />
                {preset === 'light' ? 'Light (Warm Stone)' :
                 preset === 'dark' ? 'Dark (Warm Stone)' : 'System'}
              </label>
            ))}
          </div>
        </Card>

        {/* Accent color */}
        <Card>
          <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 12 }}>Accent Color</h3>
          <div style={{ display: 'flex', gap: 12 }}>
            {ACCENT_OPTIONS.map((opt) => (
              <button
                key={opt.value}
                onClick={() => handleAccentChange(opt.value)}
                title={opt.label}
                style={{
                  width: 36, height: 36, borderRadius: 10, border: '2px solid',
                  borderColor: settings.accentColor === opt.value ? 'var(--accent-primary)' : 'var(--border-subtle)',
                  background: opt.color, cursor: 'pointer',
                  transition: 'border-color 0.15s, transform 0.15s',
                  transform: settings.accentColor === opt.value ? 'scale(1.1)' : 'scale(1)',
                }}
              />
            ))}
          </div>
        </Card>

        {/* Language selector (15.7) */}
        <Card>
          <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 12 }}>{t('settings:items.language')}</h3>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
            {LANG_OPTIONS.map((opt) => (
              <label
                key={opt.value}
                style={{
                  display: 'flex', alignItems: 'center', gap: 10, cursor: 'pointer',
                  padding: '8px 10px', borderRadius: 8,
                  background: settings.language === opt.value ? 'var(--state-selected)' : 'transparent',
                  fontSize: 14,
                }}
              >
                <input
                  type="radio"
                  name="language"
                  checked={settings.language === opt.value}
                  onChange={() => {
                    updateSetting(accountId, 'language', opt.value);
                  }}
                  style={{ accentColor: 'var(--accent-primary)' }}
                />
                {opt.label}
              </label>
            ))}
          </div>
        </Card>
      </div>
    </AppShell>
  );
}
