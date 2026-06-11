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

export function detectSystemLanguage(): SupportedLang {
  const lang = typeof navigator !== 'undefined' ? navigator.language : 'en';
  return lang.startsWith('zh') ? 'zh-CN' : 'en-US';
}

/**
 * Initialize i18next.
 * IPC is the single authoritative source — confirmed returning zh-CN on Chinese Windows.
 * Falls back to navigator.language if IPC unavailable.
 */
export async function initI18n(): Promise<typeof i18next> {
  let detectedLng: SupportedLang;

  try {
    const locale = await invoke<string>('get_system_locale');
    detectedLng = locale.startsWith('zh') ? 'zh-CN' : 'en-US';
  } catch (e) {
    detectedLng = detectSystemLanguage();
  }

  const navLang = navigator.language;
  console.log('[i18n] chosen:', detectedLng, '| navigator.language:', navLang);

  await i18next.use(initReactI18next).init({
    resources,
    lng: detectedLng,
    fallbackLng: 'en-US',
    defaultNS: 'common',
    ns: ['common', 'navigation', 'settings', 'auth', 'sensitivity', 'editor'],
    interpolation: { escapeValue: false },
  });

  console.log('[i18n] init complete, i18next.language:', i18next.language);

  return i18next;
}

export default i18next;
