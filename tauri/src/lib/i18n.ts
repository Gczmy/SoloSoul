import i18next from 'i18next';
import { initReactI18next } from 'react-i18next';

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

const resources = {
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
} as Record<string, Record<string, object>>;

// zh → zh-CN alias (Windows WebView2 may report just 'zh')
resources['zh'] = resources['zh-CN'];

/** Detect system language: zh-* → zh-CN, other → en-US */
export function detectSystemLanguage(): SupportedLang {
  const lang = typeof navigator !== 'undefined' ? navigator.language : 'en';
  return lang.startsWith('zh') ? 'zh-CN' : 'en-US';
}

const LANG_KEY = 'i18nextLng';
const stored = typeof localStorage !== 'undefined' ? localStorage.getItem(LANG_KEY) : null;
const initialLng: SupportedLang = stored === 'zh-CN' || stored === 'en-US'
  ? stored
  : detectSystemLanguage();

void i18next
  .use(initReactI18next)
  .init({
    resources,
    lng: initialLng,
    fallbackLng: 'en-US',
    defaultNS: 'common',
    ns: ['common', 'navigation', 'settings', 'auth', 'sensitivity', 'editor'],
    interpolation: { escapeValue: false },
  });

// Persist to localStorage so the setting survives across launches
if (typeof localStorage !== 'undefined') {
  localStorage.setItem(LANG_KEY, initialLng);
}

export default i18next;
