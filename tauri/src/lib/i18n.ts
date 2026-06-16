import i18next from 'i18next';
import { initReactI18next } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';

import enCommon from '@/locales/en-US/common.json';
import enNav from '@/locales/en-US/navigation.json';
import enSettings from '@/locales/en-US/settings.json';
import enAuth from '@/locales/en-US/auth.json';
import enSensitivity from '@/locales/en-US/sensitivity.json';
import enEditor from '@/locales/en-US/editor.json';
import enPlugin from '@/locales/en-US/plugin.json';

import zhCommon from '@/locales/zh-CN/common.json';
import zhNav from '@/locales/zh-CN/navigation.json';
import zhSettings from '@/locales/zh-CN/settings.json';
import zhAuth from '@/locales/zh-CN/auth.json';
import zhSensitivity from '@/locales/zh-CN/sensitivity.json';
import zhEditor from '@/locales/zh-CN/editor.json';
import zhPlugin from '@/locales/zh-CN/plugin.json';

export const SUPPORTED_LANGS = ['zh-CN', 'en-US'] as const;
export type SupportedLang = (typeof SUPPORTED_LANGS)[number];

declare global {
  interface Window {
    __SOLOSOUL_LOCALE__?: string;
  }
}

const resources: Record<string, Record<string, object>> = {
  'en-US': {
    common: enCommon,
    navigation: enNav,
    settings: enSettings,
    auth: enAuth,
    sensitivity: enSensitivity,
    editor: enEditor,
    plugin: enPlugin,
  },
  'zh-CN': {
    common: zhCommon,
    navigation: zhNav,
    settings: zhSettings,
    auth: zhAuth,
    sensitivity: zhSensitivity,
    editor: zhEditor,
    plugin: zhPlugin,
  },
};

resources['zh'] = resources['zh-CN'];

const LANG_KEY = 'i18nextLng';

export function detectSystemLanguage(): SupportedLang {
  const lang = typeof navigator !== 'undefined' ? navigator.language : 'en';
  return lang.startsWith('zh') ? 'zh-CN' : 'en-US';
}

/**
 * Initialize i18next.
 *
 * Detection priority:
 *  1. window.__SOLOSOUL_LOCALE__ — set by Rust setup eval before page loads
 *  2. localStorage — written by Rust setup eval or previous sessions
 *  3. IPC get_system_locale — Rust backend via OS API (authoritative on desktop)
 *  4. navigator.language — last resort
 */
export async function initI18n(): Promise<typeof i18next> {
  let detectedLng: SupportedLang | null = null;

  // Layer 1: Rust setup eval (injects before page loads)
  const winLocale = window.__SOLOSOUL_LOCALE__;
  if (winLocale === 'zh-CN' || winLocale === 'en-US') detectedLng = winLocale;

  // Layer 2: localStorage
  if (!detectedLng) {
    const stored = localStorage.getItem(LANG_KEY);
    if (stored === 'zh-CN' || stored === 'en-US') detectedLng = stored;
  }

  // Layer 3: Rust IPC
  if (!detectedLng) {
    const locale = await invoke<string>('get_system_locale');
    if (locale) {
      detectedLng = locale.startsWith('zh') ? 'zh-CN' : 'en-US';
    }
  }

  // Layer 4: navigator.language
  if (!detectedLng) {
    detectedLng = detectSystemLanguage();
  }

  await i18next.use(initReactI18next).init({
    resources,
    lng: detectedLng,
    fallbackLng: 'en-US',
    defaultNS: 'common',
    ns: ['common', 'navigation', 'settings', 'auth', 'sensitivity', 'editor', 'plugin'],
    interpolation: { escapeValue: false },
  });

  if (typeof localStorage !== 'undefined') {
    localStorage.setItem(LANG_KEY, detectedLng);
  }

  return i18next;
}

export default i18next;
