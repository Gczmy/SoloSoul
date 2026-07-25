import '@testing-library/jest-dom';
import { vi } from 'vitest';
import i18next from 'i18next';
import enCommon from '@/locales/en-US/common.json';
import zhCommon from '@/locales/zh-CN/common.json';

// Initialize i18next for tests so helpers like formatBytes (which use
// i18next.t directly rather than react-i18next) resolve real translations.
i18next.init({
  resources: {
    'en-US': { common: enCommon },
    'zh-CN': { common: zhCommon },
  },
  lng: 'en-US',
  fallbackLng: 'en-US',
  defaultNS: 'common',
  ns: ['common'],
  interpolation: { escapeValue: false },
});

// Mock Tauri IPC
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  addPluginListener: vi.fn(() => Promise.resolve({ unregister: vi.fn() })),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));

// Mock matchMedia
Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: vi.fn().mockImplementation((query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  })),
});

// Mock scrollIntoView (not available in jsdom)
Element.prototype.scrollIntoView = vi.fn();

// Mock ResizeObserver (not available in jsdom)
class MockResizeObserver {
  observe = vi.fn();
  unobserve = vi.fn();
  disconnect = vi.fn();
}
Object.defineProperty(window, 'ResizeObserver', {
  writable: true,
  value: MockResizeObserver,
});

// Polyfill HTMLDialogElement.showModal/close (not available in jsdom)
if (!HTMLDialogElement.prototype.showModal) {
  HTMLDialogElement.prototype.showModal = function () {
    this.open = true;
  };
  HTMLDialogElement.prototype.close = function () {
    this.open = false;
    this.dispatchEvent(new Event('close'));
  };
}

// Mock react-i18next for component tests.
// Note: `t(key, defaultValue)` with a string default is treated the same as
// `t(key)` here and returns the key, which matches most existing tests that
// assert on translation keys. Tests needing the fallback string should mock
// or assert on the key instead.
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, options?: { defaultValue?: string }) => options?.defaultValue ?? key,
    i18n: { language: 'en', changeLanguage: vi.fn(() => Promise.resolve()) },
  }),
  I18nextProvider: ({ children }: { children: React.ReactNode }) => children,
}));
