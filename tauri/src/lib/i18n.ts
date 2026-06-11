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

// zh → zh-CN alias
resources['zh'] = resources['zh-CN'];

const LANG_KEY = 'i18nextLng';

/**
 * Synchronous navigator-based detection.
 */
export function detectSystemLanguage(): SupportedLang {
  const lang = typeof navigator !== 'undefined' ? navigator.language : 'en';
  return lang.startsWith('zh') ? 'zh-CN' : 'en-US';
}

/**
 * Initialize i18next.
 * 1. Reads from localStorage (set by index.html inline script)
 * 2. Initializes i18next synchronously for fast render
 * 3. Queries Rust backend via IPC for the real system locale (most reliable on Windows)
 * 4. If IPC returns a different locale, corrects it BEFORE render completes
 */
export async function initI18n(): Promise<typeof i18next> {
  const stored = typeof localStorage !== 'undefined' ? localStorage.getItem(LANG_KEY) : null;
  const lng: SupportedLang = stored === 'zh-CN' || stored === 'en-US'
    ? stored
    : detectSystemLanguage();

  await i18next.use(initReactI18next).init({
    resources,
    lng,
    fallbackLng: 'en-US',
    defaultNS: 'common',
    ns: ['common', 'navigation', 'settings', 'auth', 'sensitivity', 'editor'],
    interpolation: { escapeValue: false },
  });

  // Query Rust backend for the real system locale (most reliable on Windows).
  // This IPC call runs AFTER i18next init but BEFORE ReactDOM.render (main.tsx awaits us).
  try {
    const locale = await invoke<string>('get_system_locale');
    const realLang: SupportedLang = locale.startsWith('zh') ? 'zh-CN' : 'en-US';
    if (realLang !== i18next.language) {
      await i18next.changeLanguage(realLang);
      if (typeof localStorage !== 'undefined') {
        localStorage.setItem(LANG_KEY, realLang);
      }
    }
  } catch {
    // IPC not available — the initial navigator-based detection is the best we can do
  }

  return i18next;
}

export default i18next;
