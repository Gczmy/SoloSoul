import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { invoke } from '@tauri-apps/api/core';
import { useSettingsStore } from '@/stores/settingsStore';
import { useAuthStore } from '@/stores/authStore';
import { applyTheme } from '@/lib/theme';
import { applyScheme } from '@/lib/themeSchemes';
import { useTranslation } from 'react-i18next';
import { ThemeSchemePanel } from '@/components/settings/ThemeSchemePanel';
import type { AccentPreset } from '@/types';
import type { SupportedLang } from '@/lib/i18n';
import { Palette } from 'lucide-react';
import type { ThemeScheme } from '@/lib/themeSchemes';

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
  const { t } = useTranslation(['settings', 'common']);
  const [isPanelOpen, setIsPanelOpen] = useState(false);

  const syncUiCache = () => {
    const s = useSettingsStore.getState().settings;
    try {
      localStorage.setItem('solosoul_ui_prefs', JSON.stringify({
        theme: s.theme,
        accentColor: s.accentColor,
        defaultLightTheme: s.defaultLightTheme,
        defaultDarkTheme: s.defaultDarkTheme,
      }));
    } catch {}
  };

  const handlePresetChange = (preset: 'light' | 'dark' | 'system') => {
    updateSetting(accountId, 'theme', preset);
    invoke('ui_update_preference', { key: 'theme', value: preset }).catch(() => {});
    applyTheme({
      preset: preset === 'dark' ? 'warm-stone-dark' :
              preset === 'light' ? 'warm-stone-light' : 'system',
      accentColor: settings.accentColor as AccentPreset,
      backgroundType: 'solid',
      backgroundValue: '',
      defaultLightTheme: settings.defaultLightTheme,
      defaultDarkTheme: settings.defaultDarkTheme,
    });
    syncUiCache();
  };

  const handleAccentChange = (accent: AccentPreset) => {
    updateSetting(accountId, 'accentColor', accent);
    invoke('ui_update_preference', { key: 'accentColor', value: accent }).catch(() => {});
    applyTheme({
      preset: settings.theme === 'dark' ? 'warm-stone-dark' :
              settings.theme === 'light' ? 'warm-stone-light' : 'system',
      accentColor: accent,
      backgroundType: 'solid',
      backgroundValue: '',
      defaultLightTheme: settings.defaultLightTheme,
      defaultDarkTheme: settings.defaultDarkTheme,
    });
    syncUiCache();
  };

  const handleSelectScheme = (scheme: ThemeScheme) => {
    // Apply scheme immediately
    applyScheme(scheme.id);
    // Persist as default for the scheme's mode
    const key = scheme.mode === 'light' ? 'defaultLightTheme' : 'defaultDarkTheme';
    updateSetting(accountId, key, scheme.id);
    invoke('ui_update_preference', { key, value: scheme.id }).catch(() => {});
    syncUiCache();
  };

  return (
    <AppShell title={t('settings:items.theme_appearance')} onBack={() => navigate('/settings')}>
      <div
        style={{
          display: 'flex',
          height: 'calc(100vh - 56px - 32px)',
          margin: '-16px',
        }}
      >
        <ThemeSchemePanel
          isOpen={isPanelOpen}
          onClose={() => setIsPanelOpen(false)}
          defaultLightTheme={settings.defaultLightTheme}
          defaultDarkTheme={settings.defaultDarkTheme}
          currentThemeMode={settings.theme}
          onSelectScheme={handleSelectScheme}
        />

        <div
          style={{
            flex: 1,
            overflowY: 'auto',
            padding: 16,
          }}
        >
          <div style={{ maxWidth: 480, margin: '0 auto', display: 'flex', flexDirection: 'column', gap: 16 }}>
            {/* Theme preset */}
            <Card>
              <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 12 }}>{t('settings:groups.appearance')}</h3>
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
                    {t(`common:theme.${preset}`)}
                  </label>
                ))}
              </div>

              {/* More appearances button */}
              <button
                onClick={() => setIsPanelOpen(true)}
                style={{
                  marginTop: 16,
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  gap: 8,
                  width: '100%',
                  padding: '10px 0',
                  borderRadius: 8,
                  border: '1px dashed var(--border-strong)',
                  background: 'transparent',
                  color: 'var(--text-secondary)',
                  fontSize: 13,
                  fontWeight: 500,
                  cursor: 'pointer',
                  transition: 'all 0.15s',
                }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.borderColor = 'var(--accent-primary)';
                  e.currentTarget.style.color = 'var(--accent-primary)';
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.borderColor = 'var(--border-strong)';
                  e.currentTarget.style.color = 'var(--text-secondary)';
                }}
              >
                <Palette size={16} />
                {t('settings:more_appearances')}
              </button>
            </Card>

            {/* Accent color */}
            <Card>
              <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 12 }}>{t("settings:accent_color")}</h3>
              <div style={{ display: 'flex', gap: 12 }}>
                {ACCENT_OPTIONS.map((opt) => (
                  <button
                    key={opt.value}
                    onClick={() => handleAccentChange(opt.value)}
                    title={t(`settings:accent_${opt.value}`)}
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
        </div>
      </div>
    </AppShell>
  );
}
