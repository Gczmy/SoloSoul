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

async function fetchLocaleFromRust(retries = 5, delay = 50): Promise<string | null> {
  for (let i = 0; i < retries; i++) {
    try {
      const locale = await invoke<string>('get_system_locale');
      console.log(`[i18n] IPC get_system_locale attempt ${i + 1}:`, locale);
      if (locale) return locale;
    } catch (e) {
      console.warn(`[i18n] IPC attempt ${i + 1} failed:`, e);
    }
    if (i < retries - 1) await new Promise((r) => setTimeout(r, delay));
  }
  return null;
}

export async function initI18n(): Promise<typeof i18next> {
  // 1. Check window.__SOLOSOUL_LOCALE__ (set by Rust eval before page load)
  const winLocale = (window as unknown as Record<string, string>).__SOLOSOUL_LOCALE__;
  console.log('[i18n] window.__SOLOSOUL_LOCALE__:', winLocale);

  // 2. Check localStorage
  const stored = typeof localStorage !== 'undefined' ? localStorage.getItem(LANG_KEY) : null;
  console.log('[i18n] localStorage i18nextLng:', stored, 'navigator.language:', navigator.language, 'navigator.userLanguage:', (navigator as unknown as Record<string, string>).userLanguage);

  const lng: SupportedLang = stored === 'zh-CN' || stored === 'en-US' ? stored : detectSystemLanguage();
  console.log('[i18n] initial lng:', lng);

  await i18next.use(initReactI18next).init({
    resources,
    lng,
    fallbackLng: 'en-US',
    defaultNS: 'common',
    ns: ['common', 'navigation', 'settings', 'auth', 'sensitivity', 'editor'],
    interpolation: { escapeValue: false },
  });
  console.log('[i18n] i18next initialized with:', i18next.language);

  // 3. IPC authoritative source (with retries)
  const locale = await fetchLocaleFromRust();
  if (locale) {
    const realLang: SupportedLang = locale.startsWith('zh') ? 'zh-CN' : 'en-US';
    console.log('[i18n] IPC realLang:', realLang, 'current:', i18next.language);
    if (realLang !== i18next.language) {
      await i18next.changeLanguage(realLang);
      console.log('[i18n] changed to:', realLang);
    }
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem(LANG_KEY, realLang);
    }
    return i18next;
  }

  // 4. Fallback to injected window var
  if (winLocale === 'zh-CN' && i18next.language !== 'zh-CN') {
    console.log('[i18n] using window.__SOLOSOUL_LOCALE__ fallback:', winLocale);
    await i18next.changeLanguage('zh-CN');
    if (typeof localStorage !== 'undefined') localStorage.setItem(LANG_KEY, 'zh-CN');
  }

  console.log('[i18n] final language:', i18next.language);
  return i18next;
}

export default i18next;
