import { useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { Sun, Moon, Monitor, Check } from 'lucide-react';
import { useShallow } from 'zustand/react/shallow';
import { PageShell } from '@/components/layout/PageShell';
import { useSettingsStore } from '@/stores/settingsStore';
import { useAuthStore } from '@/stores/authStore';
import { applyTheme, getSystemTheme } from '@/lib/theme';
import { ANDROID_PALETTES, androidMaterialTokens } from '@/lib/androidMaterial';

export function AndroidAppearance() {
  const { t } = useTranslation(['common', 'settings']);
  const navigate = useNavigate();
  const accountId = useAuthStore((s) => s.currentAccount?.id) ?? '';
  const { theme, accentColor, reduceMotion, language } = useSettingsStore(
    useShallow((s) => ({
      theme: s.settings.theme,
      accentColor: s.settings.accentColor,
      reduceMotion: s.settings.reduceMotion,
      language: s.settings.language,
    })),
  );
  const update = useSettingsStore((s) => s.updateSetting);
  useEffect(() => {
    let active = true;
    void (async () => {
      const resolvedSystemTheme = theme === 'system' ? await getSystemTheme() : undefined;
      if (!active) return;
      await applyTheme({
        preset:
          theme === 'system' ? 'system' : theme === 'dark' ? 'warm-stone-dark' : 'warm-stone-light',
        accentColor,
        backgroundType: 'solid',
        backgroundValue: '',
        resolvedSystemTheme,
      });
    })();
    return () => {
      active = false;
    };
  }, [theme, accentColor]);
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
          <h2>{t('material.palette')}</h2>
          <div className="android-theme-options">
            {ANDROID_PALETTES.map((id) => (
              <button
                type="button"
                key={id}
                className="android-theme-option"
                aria-pressed={accentColor === id}
                onClick={() => void update(accountId, 'accentColor', id)}
              >
                <span
                  className="android-theme-swatch"
                  style={{ background: androidMaterialTokens(false, id)['--accent-primary'] }}
                >
                  {accentColor === id && <Check size={20} />}
                </span>
                <span>{t(`material.palette_${id}`)}</span>
              </button>
            ))}
          </div>
        </section>
        <section className="android-theme-section">
          <label className="android-switch-row">
            <span>
              <strong>{t('material.reduce_motion')}</strong>
              <small>{t('material.reduce_motion_desc')}</small>
            </span>
            <span className="android-switch-target">
              <input
                type="checkbox"
                checked={reduceMotion}
                onChange={(event) => void update(accountId, 'reduceMotion', event.target.checked)}
              />
            </span>
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
