import i18next from 'i18next';
import { initReactI18next } from 'react-i18next';
import { invokeCommand as invoke } from '@/lib/ipcClient';

import enCommon from '@/locales/en-US/common.json';
import enNav from '@/locales/en-US/navigation.json';
import enSettings from '@/locales/en-US/settings.json';
import enAuth from '@/locales/en-US/auth.json';
import enSensitivity from '@/locales/en-US/sensitivity.json';
import enEditor from '@/locales/en-US/editor.json';
import enPlugin from '@/locales/en-US/plugin.json';
import enOcr from '@/locales/en-US/ocr.json';

import zhCommon from '@/locales/zh-CN/common.json';
import zhNav from '@/locales/zh-CN/navigation.json';
import zhSettings from '@/locales/zh-CN/settings.json';
import zhAuth from '@/locales/zh-CN/auth.json';
import zhSensitivity from '@/locales/zh-CN/sensitivity.json';
import zhEditor from '@/locales/zh-CN/editor.json';
import zhPlugin from '@/locales/zh-CN/plugin.json';
import zhOcr from '@/locales/zh-CN/ocr.json';

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
    plugin: enPlugin,
    ocr: enOcr,
  },
  'zh-CN': {
    common: zhCommon,
    navigation: zhNav,
    settings: zhSettings,
    auth: zhAuth,
    sensitivity: zhSensitivity,
    editor: zhEditor,
    plugin: zhPlugin,
    ocr: zhOcr,
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
 *  1. localStorage — user's explicit manual preference (set via /language setting)
 *  2. IPC get_system_locale — Rust backend via OS API (authoritative on desktop)
 *  3. navigator.language — last resort
 *
 * 注：此前 Rust 通过 window.eval 注入 window.__SOLOSOUL_LOCALE__ 的方式已被移除（P005），
 * 改为前端通过 IPC get_system_locale 获取（Layer 2），无需后端提前注入。
 */
export async function initI18n(): Promise<typeof i18next> {
  let detectedLng: SupportedLang | null = null;

  // Layer 1: localStorage — user's explicit preference (most authoritative)
  const stored = localStorage.getItem(LANG_KEY);
  if (stored === 'zh-CN' || stored === 'en-US') detectedLng = stored;

  // Layer 2: Rust IPC (取代此前 window.eval + window.__SOLOSOUL_LOCALE__)
  if (!detectedLng) {
    const locale = await invoke<string>('get_system_locale');
    if (locale) {
      detectedLng = locale.startsWith('zh') ? 'zh-CN' : 'en-US';
    }
  }

  // Layer 3: navigator.language
  if (!detectedLng) {
    detectedLng = detectSystemLanguage();
  }

  await i18next.use(initReactI18next).init({
    resources,
    lng: detectedLng,
    fallbackLng: 'en-US',
    defaultNS: 'common',
    ns: ['common', 'navigation', 'settings', 'auth', 'sensitivity', 'editor', 'plugin', 'ocr'],
    interpolation: { escapeValue: false },
  });

  if (typeof localStorage !== 'undefined') {
    localStorage.setItem(LANG_KEY, detectedLng);
  }

  return i18next;
}

export default i18next;
