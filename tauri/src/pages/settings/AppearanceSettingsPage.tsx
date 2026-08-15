import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useShallow } from 'zustand/react/shallow';
import { AppShell } from '@/components/layout/AppShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { Card } from '@/components/ui/Card';
import { useSettingsStore } from '@/stores/settingsStore';
import { useAuthStore } from '@/stores/authStore';
import { applyTheme, getSystemTheme } from '@/lib/theme';
import { applyScheme, getSchemeById } from '@/lib/themeSchemes';
import { useTranslation } from 'react-i18next';
import { ThemeSchemePanel } from '@/components/settings/ThemeSchemePanel';
import type { AccentPreset } from '@/types';
import type { SupportedLang } from '@/lib/i18n';
import { isMobilePlatformSync } from '@/lib/platform';
import { Palette, PanelTop, PanelBottom, PanelLeft, PanelRight } from 'lucide-react';
import type { ThemeScheme } from '@/lib/themeSchemes';
import type { AppSettings } from '@/stores/settingsStore';
import { PAGE_ICON_MAP } from '@/lib/pageIcons';
import { ICON_SIZE } from '@/lib/constants';

const ACCENT_OPTIONS: { value: AccentPreset; label: string; color: string }[] = [
  { value: 'ocean', label: 'Ocean', color: '#5B7C99' },
  { value: 'amber', label: 'Amber', color: '#C4925C' },
  { value: 'forest', label: 'Forest', color: '#5B8C6F' },
  { value: 'rose', label: 'Rose', color: '#B06B7A' },
  { value: 'purple', label: 'Purple', color: '#8B7AA8' },
];

const LANG_OPTIONS: { value: SupportedLang; label: string }[] = [
  { value: 'zh-CN', label: '中文（简体）' },
  { value: 'en-US', label: 'English' },
];

const SIDEBAR_OPTIONS: {
  value: AppSettings['sidebarPosition'];
  labelKey: string;
  icon: React.ElementType;
}[] = [
  { value: 'left', labelKey: 'settings:sidebar_left', icon: PanelLeft },
  { value: 'right', labelKey: 'settings:sidebar_right', icon: PanelRight },
  { value: 'top', labelKey: 'settings:sidebar_top', icon: PanelTop },
  { value: 'bottom', labelKey: 'settings:sidebar_bottom', icon: PanelBottom },
];

export function AppearanceSettingsPage() {
  const navigate = useNavigate();
  const currentAccount = useAuthStore((s) => s.currentAccount);
  // P022: useShallow 字段级选择——避免 store 无关字段翻转时整页重渲染
  const { settings, updateSetting } = useSettingsStore(
    useShallow((s) => ({ settings: s.settings, updateSetting: s.updateSetting })),
  );
  const accountId = currentAccount?.id || '';
  const isMobilePlatform = isMobilePlatformSync();
  const { t } = useTranslation(['settings', 'common']);
  const [isPanelOpen, setIsPanelOpen] = useState(false);

  const isHorizontal = settings.sidebarPosition === 'top' || settings.sidebarPosition === 'bottom';
  const contentHeight = isHorizontal
    ? 'calc(100vh - 48px - 56px - 32px)'
    : 'calc(100vh - 56px - 32px)';

  const lightSchemeName = t(
    getSchemeById(settings.defaultLightTheme)?.nameKey.replace('settings:', '') as string,
  );
  const darkSchemeName = t(
    getSchemeById(settings.defaultDarkTheme)?.nameKey.replace('settings:', '') as string,
  );

  // P128: ②③ 副本写入统一由 settingsStore.updateSetting（P129 集中化 helper）负责，
  // 本页不再直接写 localStorage / ui_update_preference，杜绝第 5 个漂移写入点。
  const handlePresetChange = async (preset: 'light' | 'dark' | 'system') => {
    updateSetting(accountId, 'theme', preset);
    const resolvedSystemTheme = preset === 'system' ? await getSystemTheme() : undefined;
    await applyTheme({
      preset:
        preset === 'dark' ? 'warm-stone-dark' : preset === 'light' ? 'warm-stone-light' : 'system',
      accentColor: settings.accentColor as AccentPreset,
      backgroundType: 'solid',
      backgroundValue: '',
      defaultLightTheme: settings.defaultLightTheme,
      defaultDarkTheme: settings.defaultDarkTheme,
      resolvedSystemTheme,
    });
  };

  const handleAccentChange = async (accent: AccentPreset) => {
    updateSetting(accountId, 'accentColor', accent);
    const resolvedSystemTheme = settings.theme === 'system' ? await getSystemTheme() : undefined;
    await applyTheme({
      preset:
        settings.theme === 'dark'
          ? 'warm-stone-dark'
          : settings.theme === 'light'
            ? 'warm-stone-light'
            : 'system',
      accentColor: accent,
      backgroundType: 'solid',
      backgroundValue: '',
      defaultLightTheme: settings.defaultLightTheme,
      defaultDarkTheme: settings.defaultDarkTheme,
      resolvedSystemTheme,
    });
  };

  const handleSelectScheme = async (scheme: ThemeScheme) => {
    const currentMode = settings.theme === 'system' ? await getSystemTheme() : settings.theme;
    // If the selected scheme's mode differs from the current theme setting,
    // automatically switch to match so the UI state reflects the selected theme.
    if (scheme.mode !== currentMode) {
      const newTheme = scheme.mode;
      updateSetting(accountId, 'theme', newTheme);
      await applyTheme({
        preset: newTheme === 'dark' ? 'warm-stone-dark' : 'warm-stone-light',
        accentColor: settings.accentColor as AccentPreset,
        backgroundType: 'solid',
        backgroundValue: '',
        defaultLightTheme: scheme.mode === 'light' ? scheme.id : settings.defaultLightTheme,
        defaultDarkTheme: scheme.mode === 'dark' ? scheme.id : settings.defaultDarkTheme,
      });
    } else {
      // Apply scheme immediately (mode matches current theme)
      applyScheme(scheme.id);
    }
    // Persist as default for the scheme's mode
    const key = scheme.mode === 'light' ? 'defaultLightTheme' : 'defaultDarkTheme';
    updateSetting(accountId, key, scheme.id);
  };

  return (
    <AppShell title={t('settings:items.theme_appearance')} onBack={() => navigate('/settings')}>
      <div
        style={{
          display: 'flex',
          height: contentHeight,
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
            padding: '16px 0',
          }}
        >
          <PageContainer variant="form" gap="default">
            {/* Theme preset */}
            <Card>
              <h3 style={{ fontSize: 'var(--text-sm)', fontWeight: 600, marginBottom: 12 }}>
                {t('settings:groups.appearance')}
              </h3>
              <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
                {(['light', 'dark', 'system'] as const).map((preset) => (
                  <label
                    key={preset}
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      gap: 10,
                      cursor: 'pointer',
                      padding: '8px 10px',
                      borderRadius: 8,
                      background:
                        settings.theme === preset ? 'var(--state-selected)' : 'transparent',
                      fontSize: 'var(--text-sm)',
                    }}
                  >
                    <input
                      type="radio"
                      name="theme"
                      checked={settings.theme === preset}
                      onChange={() => handlePresetChange(preset)}
                      style={{ accentColor: 'var(--accent-primary)' }}
                    />
                    {preset === 'light'
                      ? `${t('common:theme.light')} · ${lightSchemeName}`
                      : preset === 'dark'
                        ? `${t('common:theme.dark')} · ${darkSchemeName}`
                        : t('common:theme.system')}
                  </label>
                ))}
              </div>

              {/* More appearances button */}
              <button
                onClick={() => setIsPanelOpen((open) => !open)}
                className="interactive-dashed"
                style={{
                  marginTop: 16,
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  gap: 8,
                  width: '100%',
                  padding: '10px 0',
                  borderRadius: 8,
                  borderWidth: 1,
                  borderStyle: 'dashed',
                  fontSize: 'var(--text-body-sm)',
                  fontWeight: 500,
                  cursor: 'pointer',
                }}
              >
                <Palette size={ICON_SIZE.md} />
                {t('settings:more_appearances')}
              </button>
            </Card>

            {/* Accent color */}
            <Card>
              <h3 style={{ fontSize: 'var(--text-sm)', fontWeight: 600, marginBottom: 12 }}>
                {t('settings:accent_color')}
              </h3>
              <div style={{ display: 'flex', gap: 12 }}>
                {ACCENT_OPTIONS.map((opt) => (
                  <button
                    key={opt.value}
                    onClick={() => handleAccentChange(opt.value)}
                    title={t(`settings:accent_${opt.value}`)}
                    style={{
                      width: 36,
                      height: 36,
                      borderRadius: 10,
                      border: '2px solid',
                      borderColor:
                        settings.accentColor === opt.value
                          ? 'var(--accent-primary)'
                          : 'var(--border-subtle)',
                      background: opt.color,
                      cursor: 'pointer',
                      transition: 'border-color 0.15s, transform 0.15s',
                      transform: settings.accentColor === opt.value ? 'scale(1.1)' : 'scale(1)',
                    }}
                  />
                ))}
              </div>
            </Card>

            {/* Language selector (15.7) */}
            <Card>
              <h3 style={{ fontSize: 'var(--text-sm)', fontWeight: 600, marginBottom: 12 }}>
                {t('settings:items.language')}
              </h3>
              <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
                {LANG_OPTIONS.map((opt) => (
                  <label
                    key={opt.value}
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      gap: 10,
                      cursor: 'pointer',
                      padding: '8px 10px',
                      borderRadius: 8,
                      background:
                        settings.language === opt.value ? 'var(--state-selected)' : 'transparent',
                      fontSize: 'var(--text-sm)',
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

            {/* Sidebar position (desktop only) */}
            {!isMobilePlatform && (
              <Card>
                <h3 style={{ fontSize: 'var(--text-sm)', fontWeight: 600, marginBottom: 12 }}>
                  {t('settings:sidebar_position')}
                </h3>
                <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: 8 }}>
                  {SIDEBAR_OPTIONS.map((opt) => {
                    const Icon = opt.icon;
                    const isActive = settings.sidebarPosition === opt.value;
                    return (
                      <button
                        key={opt.value}
                        onClick={() => updateSetting(accountId, 'sidebarPosition', opt.value)}
                        style={{
                          display: 'flex',
                          flexDirection: 'column',
                          alignItems: 'center',
                          gap: 6,
                          padding: '12px 4px',
                          borderRadius: 10,
                          border: isActive
                            ? '2px solid var(--accent-primary)'
                            : '1px solid var(--border-subtle)',
                          background: isActive ? 'rgba(91,124,153,0.08)' : 'transparent',
                          cursor: 'pointer',
                          transition: 'all 0.15s ease',
                          color: isActive ? 'var(--accent-primary)' : 'var(--text-secondary)',
                        }}
                      >
                        <Icon size={ICON_SIZE['2xl']} />
                        <span
                          style={{
                            fontSize: 'var(--text-caption)',
                            fontWeight: isActive ? 500 : 400,
                          }}
                        >
                          {t(opt.labelKey)}
                        </span>
                      </button>
                    );
                  })}
                </div>
              </Card>
            )}

            {/* Sidebar button mode: card vs page (desktop only) */}
            {!isMobilePlatform && (
              <Card>
                <h3 style={{ fontSize: 'var(--text-sm)', fontWeight: 600, marginBottom: 4 }}>
                  {t('settings:sidebar_button_mode')}
                </h3>
                <p
                  style={{
                    fontSize: 'var(--text-caption)',
                    color: 'var(--text-secondary)',
                    marginBottom: 12,
                  }}
                >
                  {t('settings:sidebar_button_mode_desc')}
                </p>
                <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
                  {(['ocr', 'plugins', 'ai_chat', 'search'] as const).map((id) => {
                    const currentMode = settings.sidebarButtonModes[id] || 'card';
                    const Icon = PAGE_ICON_MAP[id];
                    const label = t(`navigation:${id}`);
                    return (
                      <label
                        key={id}
                        className="interactive-neutral"
                        style={{
                          display: 'flex',
                          alignItems: 'center',
                          justifyContent: 'space-between',
                          padding: '8px 10px',
                          borderRadius: 8,
                          cursor: 'pointer',
                        }}
                      >
                        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                          <Icon size={ICON_SIZE.md} style={{ color: 'var(--text-secondary)' }} />
                          <span style={{ fontSize: 'var(--text-body-sm)', fontWeight: 500 }}>
                            {label}
                          </span>
                        </div>
                        <select
                          value={currentMode}
                          onChange={(e) => {
                            const newModes: Record<string, 'card' | 'page'> = {
                              ...settings.sidebarButtonModes,
                              [id]: e.target.value as 'card' | 'page',
                            };
                            updateSetting(accountId, 'sidebarButtonModes', newModes);
                          }}
                          style={{
                            padding: '4px 8px',
                            fontSize: 'var(--text-caption)',
                            borderRadius: 6,
                            border: '1px solid var(--border-subtle)',
                            background: 'var(--bg-elevated)',
                            color: 'var(--text-primary)',
                            cursor: 'pointer',
                            fontFamily: 'inherit',
                          }}
                        >
                          <option value="card">{t('settings:button_mode_card')}</option>
                          <option value="page">{t('settings:button_mode_page')}</option>
                        </select>
                      </label>
                    );
                  })}
                </div>
              </Card>
            )}
          </PageContainer>
        </div>
      </div>
    </AppShell>
  );
}
