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
 * Try to get the system locale from Rust backend, with retries.
 * Retries up to `retries` times with `delay` ms between attempts.
 */
async function fetchLocaleFromRust(retries = 5, delay = 50): Promise<string | null> {
  for (let i = 0; i < retries; i++) {
    try {
      const locale = await invoke<string>('get_system_locale');
      if (locale) return locale;
    } catch {
      // IPC not ready yet — wait and retry
    }
    if (i < retries - 1) await new Promise((r) => setTimeout(r, delay));
  }
  return null;
}

export async function initI18n(): Promise<typeof i18next> {
  // 1. Fast init from localStorage (set by index.html inline script or Rust setup eval)
  const stored = typeof localStorage !== 'undefined' ? localStorage.getItem(LANG_KEY) : null;
  const lng: SupportedLang = stored === 'zh-CN' || stored === 'en-US' ? stored : detectSystemLanguage();

  await i18next.use(initReactI18next).init({
    resources,
    lng,
    fallbackLng: 'en-US',
    defaultNS: 'common',
    ns: ['common', 'navigation', 'settings', 'auth', 'sensitivity', 'editor'],
    interpolation: { escapeValue: false },
  });

  // 2. Authoritative source: Rust backend via IPC (with retries)
  const locale = await fetchLocaleFromRust();
  if (locale) {
    const realLang: SupportedLang = locale.startsWith('zh') ? 'zh-CN' : 'en-US';
    if (realLang !== i18next.language) {
      await i18next.changeLanguage(realLang);
    }
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem(LANG_KEY, realLang);
    }
    return i18next;
  }

  // 3. Fallback: use window.__SOLOSOUL_LOCALE__ (set by Rust eval before page load)
  const winLocale = (window as unknown as Record<string, string>).__SOLOSOUL_LOCALE__;
  if (winLocale === 'zh-CN' && i18next.language !== 'zh-CN') {
    await i18next.changeLanguage('zh-CN');
    if (typeof localStorage !== 'undefined') localStorage.setItem(LANG_KEY, 'zh-CN');
  }

  return i18next;
}

export default i18next;
