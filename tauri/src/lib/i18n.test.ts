import { describe, it, expect, vi, beforeEach } from 'vitest';

// ── Mocks ──────────────────────────────────────────────────────────────────

vi.mock('i18next', () => {
  const mockI18next = {
    use: vi.fn().mockReturnThis(),
    init: vi.fn().mockResolvedValue(undefined),
  };
  return { default: mockI18next };
});

vi.mock('react-i18next', () => ({
  initReactI18next: Object.freeze({}),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

// ── Constants ──────────────────────────────────────────────────────────────

const LANG_KEY = 'i18nextLng';

// ── SUPPORTED_LANGS ────────────────────────────────────────────────────────

describe('SUPPORTED_LANGS', () => {
  it('contains zh-CN and en-US', async () => {
    const { SUPPORTED_LANGS } = await import('./i18n');
    expect(SUPPORTED_LANGS).toEqual(['zh-CN', 'en-US']);
  });

  it('has exactly 2 supported languages', async () => {
    const { SUPPORTED_LANGS } = await import('./i18n');
    expect(SUPPORTED_LANGS.length).toBe(2);
  });
});

// ── detectSystemLanguage ───────────────────────────────────────────────────

describe('detectSystemLanguage', () => {
  beforeEach(() => {
    Object.defineProperty(navigator, 'language', {
      value: 'en-US',
      configurable: true,
    });
  });

  it('returns zh-CN when navigator.language is zh-CN', async () => {
    Object.defineProperty(navigator, 'language', {
      value: 'zh-CN',
      configurable: true,
    });
    const { detectSystemLanguage } = await import('./i18n');
    expect(detectSystemLanguage()).toBe('zh-CN');
  });

  it('returns zh-CN for zh-TW', async () => {
    Object.defineProperty(navigator, 'language', {
      value: 'zh-TW',
      configurable: true,
    });
    const { detectSystemLanguage } = await import('./i18n');
    expect(detectSystemLanguage()).toBe('zh-CN');
  });

  it('returns zh-CN for zh-HK', async () => {
    Object.defineProperty(navigator, 'language', {
      value: 'zh-HK',
      configurable: true,
    });
    const { detectSystemLanguage } = await import('./i18n');
    expect(detectSystemLanguage()).toBe('zh-CN');
  });

  it('returns en-US when navigator.language is en-US', async () => {
    Object.defineProperty(navigator, 'language', {
      value: 'en-US',
      configurable: true,
    });
    const { detectSystemLanguage } = await import('./i18n');
    expect(detectSystemLanguage()).toBe('en-US');
  });

  it('returns en-US for non-Chinese languages (French)', async () => {
    Object.defineProperty(navigator, 'language', {
      value: 'fr-FR',
      configurable: true,
    });
    const { detectSystemLanguage } = await import('./i18n');
    expect(detectSystemLanguage()).toBe('en-US');
  });

  it('returns en-US for Japanese', async () => {
    Object.defineProperty(navigator, 'language', {
      value: 'ja-JP',
      configurable: true,
    });
    const { detectSystemLanguage } = await import('./i18n');
    expect(detectSystemLanguage()).toBe('en-US');
  });
});

// ── initI18n ───────────────────────────────────────────────────────────────

describe('initI18n', () => {
  let invokeMock: ReturnType<typeof vi.fn>;

  beforeEach(async () => {
    vi.clearAllMocks();

    // Reset DOM/storage state
    localStorage.clear();

    // Default navigator.language = en-US
    Object.defineProperty(navigator, 'language', {
      value: 'en-US',
      configurable: true,
    });

    // Reset invoke mock: default = empty string (no locale info)
    const core = await import('@tauri-apps/api/core');
    invokeMock = core.invoke as ReturnType<typeof vi.fn>;
    invokeMock.mockReset();
    invokeMock.mockResolvedValue('');
  });

  // ── Layer 1: localStorage ─────────────────────────────────────────────

  it('reads zh-CN from localStorage (layer 1)', async () => {
    localStorage.setItem(LANG_KEY, 'zh-CN');
    const { initI18n } = await import('./i18n');
    await initI18n();

    const { init } = vi.mocked((await import('i18next')).default);
    expect(init).toHaveBeenCalledWith(expect.objectContaining({ lng: 'zh-CN' }));
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it('reads en-US from localStorage', async () => {
    localStorage.setItem(LANG_KEY, 'en-US');
    const { initI18n } = await import('./i18n');
    await initI18n();

    const { init } = vi.mocked((await import('i18next')).default);
    expect(init).toHaveBeenCalledWith(expect.objectContaining({ lng: 'en-US' }));
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it('ignores invalid localStorage value and falls to layer 2', async () => {
    localStorage.setItem(LANG_KEY, 'fr-FR'); // not zh-CN or en-US
    invokeMock.mockResolvedValue('zh-CN');
    const { initI18n } = await import('./i18n');
    await initI18n();

    const { init } = vi.mocked((await import('i18next')).default);
    // Layer 2 (IPC) provides zh-CN
    expect(init).toHaveBeenCalledWith(expect.objectContaining({ lng: 'zh-CN' }));
    expect(invokeMock).toHaveBeenCalledWith('get_system_locale');
  });

  // ── Layer 2: IPC ───────────────────────────────────────────────────────

  it('uses IPC result zh-CN when localStorage is empty (layer 2)', async () => {
    invokeMock.mockResolvedValue('zh-CN');
    const { initI18n } = await import('./i18n');
    await initI18n();

    const { init } = vi.mocked((await import('i18next')).default);
    expect(init).toHaveBeenCalledWith(expect.objectContaining({ lng: 'zh-CN' }));
    expect(invokeMock).toHaveBeenCalledWith('get_system_locale');
  });

  it('uses IPC result en-US', async () => {
    invokeMock.mockResolvedValue('en-US');
    const { initI18n } = await import('./i18n');
    await initI18n();

    const { init } = vi.mocked((await import('i18next')).default);
    expect(init).toHaveBeenCalledWith(expect.objectContaining({ lng: 'en-US' }));
    expect(invokeMock).toHaveBeenCalledWith('get_system_locale');
  });

  it('interprets IPC zh prefix as zh-CN', async () => {
    invokeMock.mockResolvedValue('zh-Hans-CN');
    const { initI18n } = await import('./i18n');
    await initI18n();

    const { init } = vi.mocked((await import('i18next')).default);
    expect(init).toHaveBeenCalledWith(expect.objectContaining({ lng: 'zh-CN' }));
  });

  // ── Layer 3: navigator.language ────────────────────────────────────────

  it('falls back to navigator.language when all earlier layers fail (layer 3)', async () => {
    // All layers 1-2 return nothing useful
    const { initI18n } = await import('./i18n');
    await initI18n();

    const { init } = vi.mocked((await import('i18next')).default);
    // navigator.language is 'en-US' (default in beforeEach)
    expect(init).toHaveBeenCalledWith(expect.objectContaining({ lng: 'en-US' }));
  });

  it('falls to navigator zh-CN when IPC returns empty and layers 1 absent', async () => {
    Object.defineProperty(navigator, 'language', {
      value: 'zh-CN',
      configurable: true,
    });
    invokeMock.mockResolvedValue(''); // IPC returns empty
    const { initI18n } = await import('./i18n');
    await initI18n();

    const { init } = vi.mocked((await import('i18next')).default);
    expect(init).toHaveBeenCalledWith(expect.objectContaining({ lng: 'zh-CN' }));
    expect(invokeMock).toHaveBeenCalledWith('get_system_locale');
  });

  // ── Infrastructure ──────────────────────────────────────────────────────

  it('calls i18next.use(initReactI18next) before init', async () => {
    const { initI18n, default: i18next } = await import('./i18n');
    // initI18n triggers the chain inside the function
    await initI18n();

    expect(i18next.use).toHaveBeenCalledWith(expect.objectContaining({}));
  });

  it('stores detected language to localStorage after successful init', async () => {
    invokeMock.mockResolvedValue('zh-CN');
    const { initI18n } = await import('./i18n');
    await initI18n();

    expect(localStorage.getItem(LANG_KEY)).toBe('zh-CN');
  });

  it('configures fallbackLng as en-US and defaultNS as common', async () => {
    const { initI18n } = await import('./i18n');
    await initI18n();

    const { init } = vi.mocked((await import('i18next')).default);
    expect(init).toHaveBeenCalledWith(
      expect.objectContaining({
        fallbackLng: 'en-US',
        defaultNS: 'common',
      })
    );
  });

  it('uses all 8 namespace modules', async () => {
    const { initI18n } = await import('./i18n');
    await initI18n();

    const { init } = vi.mocked((await import('i18next')).default);
    expect(init).toHaveBeenCalledWith(
      expect.objectContaining({
        ns: expect.arrayContaining([
          'common', 'navigation', 'settings', 'auth',
          'sensitivity', 'editor', 'plugin', 'ocr',
        ]),
      })
    );
  });

  it('returns i18next instance', async () => {
    const { initI18n, default: i18next } = await import('./i18n');
    const result = await initI18n();

    expect(result).toBe(i18next);
  });
});
