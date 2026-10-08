import { useNavigate } from 'react-router-dom';
import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Sun, Moon, Monitor, Check } from 'lucide-react';
import { useShallow } from 'zustand/react/shallow';
import { PageShell } from '@/components/layout/PageShell';
import { SelectCheckbox } from '@/components/ui/SelectCheckbox';
import { useSettingsStore } from '@/stores/settingsStore';
import { useSettingAction } from '@/hooks/useSettingAction';
import { useAuthStore } from '@/stores/authStore';
import { ACCENT_COLORS } from '@/lib/theme';
import { getSchemesByMode } from '@/lib/themeSchemes';
import { accentTextColor } from '@/lib/accentContrast';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { ANDROID_GLASS_MODES } from '@/lib/androidGlass';

export function AndroidAppearance() {
  const { t } = useTranslation(['common', 'settings']);
  const navigate = useNavigate();
  const accountId = useAuthStore((s) => s.currentAccount?.id) ?? '';
  const {
    theme,
    accentColor,
    customAccentHex,
    defaultLightTheme,
    defaultDarkTheme,
    reduceMotion,
    androidGlass,
    language,
  } = useSettingsStore(
    useShallow((s) => ({
      theme: s.settings.theme,
      accentColor: s.settings.accentColor,
      customAccentHex: s.settings.customAccentHex,
      defaultLightTheme: s.settings.defaultLightTheme,
      defaultDarkTheme: s.settings.defaultDarkTheme,
      reduceMotion: s.settings.reduceMotion,
      androidGlass: s.settings.androidGlass,
      language: s.settings.language,
    })),
  );
  const update = useSettingAction();
  const [custom, setCustom] = useState(customAccentHex || '#5b7c99');
  const [saving, setSaving] = useState(false);
  const accentRequest = useRef(0);
  useEffect(() => {
    setCustom(customAccentHex || '#5b7c99');
  }, [customAccentHex, accountId]);
  useEffect(() => {
    setSaving(false);
  }, [accountId]);
  useEffect(
    () => () => {
      accentRequest.current += 1;
    },
    [accountId],
  );
  const selectAccent = (id: keyof typeof ACCENT_COLORS) => {
    accentRequest.current += 1;
    setSaving(false);
    void update(accountId, 'accentColor', id);
  };
  const saveCustom = async () => {
    const hex = custom.trim();
    if (!/^#[\da-f]{6}$/i.test(hex)) return;
    const request = ++accentRequest.current;
    setSaving(true);
    try {
      const result = await update(accountId, 'customAccentHex', hex);
      if (request !== accentRequest.current || result.status !== 'saved' || !result.isCurrent())
        return;
      await update(accountId, 'accentColor', 'custom');
    } finally {
      if (request === accentRequest.current) setSaving(false);
    }
  };
  return (
    <PageShell title={t('settings:items.theme_appearance')} onBack={() => navigate('/settings')}>
      <div className="android-page" style={{ maxWidth: 640 }}>
        <section className="android-theme-section">
          <h2>{t('material.appearance')}</h2>
          <div className="android-theme-options">
            {(
              [
                { id: 'light', Icon: Sun },
                { id: 'dark', Icon: Moon },
                { id: 'system', Icon: Monitor },
              ] as const
            ).map(({ id, Icon }) => (
              <button
                type="button"
                key={id}
                className="android-theme-option"
                aria-pressed={theme === id}
                onClick={() => void update(accountId, 'theme', id)}
              >
                <Icon size={24} />
                <span>{t(`material.theme_${id}`)}</span>
              </button>
            ))}
          </div>
        </section>
        <section className="android-theme-section">
          <h2>{t('settings:theme_schemes')}</h2>
          {(['light', 'dark'] as const).map((mode) => (
            <label key={mode} className="android-scheme-row">
              <span>{t(`settings:${mode}_themes`)}</span>
              <select
                aria-label={t(`settings:${mode}_themes`)}
                value={mode === 'light' ? defaultLightTheme : defaultDarkTheme}
                onChange={(event) =>
                  void update(
                    accountId,
                    mode === 'light' ? 'defaultLightTheme' : 'defaultDarkTheme',
                    event.target.value,
                  )
                }
              >
                {getSchemesByMode(mode).map((scheme) => (
                  <option key={scheme.id} value={scheme.id}>
                    {t(scheme.nameKey)}
                  </option>
                ))}
              </select>
            </label>
          ))}
        </section>
        <section className="android-theme-section">
          <h2>{t('settings:accent_color')}</h2>
          <div className="android-theme-options">
            {(['ocean', 'forest', 'amber', 'rose', 'purple'] as const).map((id) => (
              <button
                type="button"
                key={id}
                className="android-theme-option"
                aria-pressed={accentColor === id}
                onClick={() => selectAccent(id)}
              >
                <span
                  className="android-theme-swatch"
                  style={{
                    background: ACCENT_COLORS[id],
                    color: accentTextColor(ACCENT_COLORS[id])!,
                  }}
                >
                  {accentColor === id && <Check size={20} />}
                </span>
                <span>{t(`settings:accent_${id}`)}</span>
              </button>
            ))}
          </div>
          <form
            className="android-custom-accent"
            onSubmit={(event) => {
              event.preventDefault();
              void saveCustom();
            }}
          >
            <Input
              label={t('settings:accent_custom')}
              aria-label={t('settings:accent_custom')}
              value={custom}
              onChange={(event) => setCustom(event.target.value)}
              maxLength={7}
              pattern="#[0-9a-fA-F]{6}"
              placeholder="#5b7c99"
              autoCapitalize="none"
              spellCheck={false}
            />
            <Button
              type="submit"
              variant="secondary"
              loading={saving}
              disabled={!/^#[\da-f]{6}$/i.test(custom.trim())}
            >
              {t('save')}
            </Button>
          </form>
        </section>
        <section className="android-theme-section">
          <h2>{t('material.glass_title')}</h2>
          <p className="android-material-description">{t('material.glass_desc')}</p>
          <div className="android-theme-options" aria-label={t('material.glass_title')}>
            {ANDROID_GLASS_MODES.map((mode) => (
              <button
                type="button"
                key={mode}
                className="android-theme-option"
                aria-pressed={androidGlass === mode}
                onClick={() => void update(accountId, 'androidGlass', mode)}
              >
                {t(`material.glass_${mode}`)}
              </button>
            ))}
          </div>
          <p className="android-material-description">{t(`material.glass_${androidGlass}_desc`)}</p>
        </section>
        <section className="android-theme-section">
          <label className="android-switch-row">
            <span>
              <strong>{t('material.reduce_motion')}</strong>
              <small>{t('material.reduce_motion_desc')}</small>
            </span>
            <SelectCheckbox
              checked={reduceMotion}
              onChange={(checked) => void update(accountId, 'reduceMotion', checked)}
              aria-label={t('material.reduce_motion')}
            />
          </label>
        </section>
        <section className="android-theme-section">
          <h2>{t('settings:items.language')}</h2>
          <div
            className="android-theme-options"
            style={{ gridTemplateColumns: 'repeat(2,minmax(0,1fr))' }}
          >
            {(
              [
                { id: 'zh-CN', label: '简体中文' },
                { id: 'en-US', label: 'English' },
              ] as const
            ).map(({ id, label }) => (
              <button
                type="button"
                className="android-theme-option"
                key={id}
                aria-pressed={language === id}
                onClick={() => void update(accountId, 'language', id)}
              >
                {label}
              </button>
            ))}
          </div>
        </section>
      </div>
    </PageShell>
  );
}
