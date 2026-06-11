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

// zh → zh-CN alias (some environments may report just 'zh')
resources['zh'] = resources['zh-CN'];

const LANG_KEY = 'i18nextLng';

/**
 * Synchronous system language detection using navigator.language.
 * Used for default values in settings — not as authoritative as the async version.
 */
export function detectSystemLanguage(): SupportedLang {
  const lang = typeof navigator !== 'undefined' ? navigator.language : 'en';
  return lang.startsWith('zh') ? 'zh-CN' : 'en-US';
}

/**
 * Detect system language via Rust backend (sys-locale), fallback to navigator.language.
 * This is more reliable than navigator.language in Tauri WebView2 on Windows.
 */
async function detectSystemLanguageAsync(): Promise<SupportedLang> {
  try {
    const locale = await invoke<string>('get_system_locale');
    if (locale.startsWith('zh')) return 'zh-CN';
    if (locale.startsWith('en')) return 'en-US';
  } catch {
    // ignore, fall through to navigator fallback
  }
  return detectSystemLanguage();
}

/** Initialize i18next with the detected language. Exported for async init. */
export async function initI18n(): Promise<typeof i18next> {
  const stored = typeof localStorage !== 'undefined' ? localStorage.getItem(LANG_KEY) : null;
  const lng: SupportedLang = stored === 'zh-CN' || stored === 'en-US'
    ? stored
    : await detectSystemLanguageAsync();

  await i18next.use(initReactI18next).init({
    resources,
    lng,
    fallbackLng: 'en-US',
    defaultNS: 'common',
    ns: ['common', 'navigation', 'settings', 'auth', 'sensitivity', 'editor'],
    interpolation: { escapeValue: false },
  });

  // Persist to localStorage so next cold launch skips the async detection call
  if (typeof localStorage !== 'undefined') {
    localStorage.setItem(LANG_KEY, lng);
  }

  return i18next;
}

export default i18next;
