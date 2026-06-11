import i18next from 'i18next';
import { initReactI18next } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';

import enCommon from '@/locales/en-US/common.json';
import enNav from '@/locales/en-US/navigation.json';
import enSettings from '@/locales/en-US/settings.json';
import enAuth from '@/locales/en-US/auth.json';
import enSensitivity from '@/locales/en-US/sensitivity.json';
import enEditor from '@/locales/en-US/editor.json';

import zhCommon from '@/locales/zh-CN/common.json';
import zhNav from '@/locales/zh-CN/navigation.json';
import zhSettings from '@/locales/zh-CN/settings.json';
import zhAuth from '@/locales/zh-CN/auth.json';
import zhSensitivity from '@/locales/zh-CN/sensitivity.json';
import zhEditor from '@/locales/zh-CN/editor.json';

export const SUPPORTED_LANGS = ['zh-CN', 'en-US'] as const;
export type SupportedLang = (typeof SUPPORTED_LANGS)[number];

const resources: Record<string, Record<string, object>> = {
  'en-US': {
    common: enCommon,
    navigation: enNav,
    settings: enSettings,
    auth: enAuth,
    sensitivity: enSensitivity,
    editor: enEditor,
  },
  'zh-CN': {
    common: zhCommon,
    navigation: zhNav,
    settings: zhSettings,
    auth: zhAuth,
    sensitivity: zhSensitivity,
    editor: zhEditor,
  },
};

resources['zh'] = resources['zh-CN'];

const LANG_KEY = 'i18nextLng';

export function detectSystemLanguage(): SupportedLang {
  const lang = typeof navigator !== 'undefined' ? navigator.language : 'en';
  return lang.startsWith('zh') ? 'zh-CN' : 'en-US';
}

/**
 * Initialize i18next with three layers of detection:
 * 1. window.__SOLOSOUL_LOCALE__ (set by Rust setup eval BEFORE page loads)
 * 2. localStorage (set by Rust eval and/or previous sessions)
 * 3. Rust IPC get_system_locale (authoritative, with retries)
 * 4. navigator.language (last resort)
 */
export async function initI18n(): Promise<typeof i18next> {
  let detectedLng: SupportedLang | null = null;

  // Layer 1: window.__SOLOSOUL_LOCALE__ (set by Rust eval, confirmed working = zh-CN)
  const winLocale = (window as unknown as Record<string, string>).__SOLOSOUL_LOCALE__;
  console.log('[i18n] __SOLOSOUL_LOCALE__:', winLocale);
  if (winLocale === 'zh-CN' || winLocale === 'en-US') detectedLng = winLocale;

  // Layer 2: localStorage (set by Rust eval via localStorage.setItem)
  if (!detectedLng) {
    const stored = localStorage.getItem(LANG_KEY);
    console.log('[i18n] localStorage:', stored);
    if (stored === 'zh-CN' || stored === 'en-US') detectedLng = stored;
  }

  // Layer 3: Rust IPC (with retries for early startup race)
  if (!detectedLng) {
    for (let i = 0; i < 10; i++) {
      try {
        const locale = await invoke<string>('get_system_locale');
        console.log('[i18n] IPC attempt', i + 1, ':', locale);
        if (locale) { detectedLng = locale.startsWith('zh') ? 'zh-CN' : 'en-US'; break; }
      } catch (e) {
        console.warn('[i18n] IPC attempt', i + 1, 'failed:', e);
      }
      await new Promise((r) => setTimeout(r, 100));
    }
  }

  // Layer 4: navigator.language
  if (!detectedLng) {
    detectedLng = detectSystemLanguage();
    console.log('[i18n] navigator fallback:', navigator.language, '->', detectedLng);
  }

  // Layer 5: window.__SOLOSOUL_LOCALE__ as authoritative override (if Rust eval succeeded)
  const winLocale2 = (window as unknown as Record<string, string>).__SOLOSOUL_LOCALE__;
  if (winLocale2 === 'zh-CN' && detectedLng === 'en-US') {
    console.log('[i18n] overriding with __SOLOSOUL_LOCALE__:', winLocale2);
    detectedLng = 'zh-CN';
  }

  console.log('[i18n] FINAL language:', detectedLng);

  await i18next.use(initReactI18next).init({
    resources,
    lng: detectedLng,
    fallbackLng: 'en-US',
    defaultNS: 'common',
    ns: ['common', 'navigation', 'settings', 'auth', 'sensitivity', 'editor'],
    interpolation: { escapeValue: false },
  });

  console.log('[i18n] i18next.language after init:', i18next.language);

  if (typeof localStorage !== 'undefined') {
    localStorage.setItem(LANG_KEY, detectedLng);
  }

  return i18next;
}

export default i18next;
