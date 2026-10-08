import { describe, it, expect, vi, beforeEach } from 'vitest';

describe('getSchemeById', () => {
  it('returns the warm-stone scheme by id', async () => {
    const { getSchemeById } = await import('./themeSchemes');
    const scheme = getSchemeById('warm-stone');
    expect(scheme).toBeDefined();
    expect(scheme!.id).toBe('warm-stone');
    expect(scheme!.mode).toBe('light');
  });

  it('returns the warm-stone-dark scheme by id', async () => {
    const { getSchemeById } = await import('./themeSchemes');
    const scheme = getSchemeById('warm-stone-dark');
    expect(scheme).toBeDefined();
    expect(scheme!.id).toBe('warm-stone-dark');
    expect(scheme!.mode).toBe('dark');
  });

  it('returns undefined for an unknown id', async () => {
    const { getSchemeById } = await import('./themeSchemes');
    expect(getSchemeById('nonexistent')).toBeUndefined();
  });

  it('returns undefined for null', async () => {
    const { getSchemeById } = await import('./themeSchemes');
    expect(getSchemeById(null)).toBeUndefined();
  });

  it('returns a scheme with the expected fields', async () => {
    const { getSchemeById } = await import('./themeSchemes');
    const scheme = getSchemeById('obsidian-black');
    expect(scheme).toMatchObject({
      id: 'obsidian-black',
      nameKey: expect.stringContaining('scheme_'),
      mode: 'dark',
      preview: expect.objectContaining({ bg: expect.any(String) }),
      variables: expect.objectContaining({ '--bg-base': expect.any(String) }),
    });
  });
});

describe('getSchemesByMode', () => {
  it('returns all light schemes', async () => {
    const { getSchemesByMode, THEME_SCHEMES } = await import('./themeSchemes');
    const lightSchemes = getSchemesByMode('light');
    const totalLight = THEME_SCHEMES.filter((s) => s.mode === 'light').length;
    expect(lightSchemes.length).toBe(totalLight);
    expect(lightSchemes.length).toBeGreaterThan(0);
  });

  it('all returned light schemes have mode === "light"', async () => {
    const { getSchemesByMode } = await import('./themeSchemes');
    const lightSchemes = getSchemesByMode('light');
    expect(lightSchemes.every((s) => s.mode === 'light')).toBe(true);
  });

  it('returns all dark schemes', async () => {
    const { getSchemesByMode, THEME_SCHEMES } = await import('./themeSchemes');
    const darkSchemes = getSchemesByMode('dark');
    const totalDark = THEME_SCHEMES.filter((s) => s.mode === 'dark').length;
    expect(darkSchemes.length).toBe(totalDark);
    expect(darkSchemes.length).toBeGreaterThan(0);
  });

  it('all returned dark schemes have mode === "dark"', async () => {
    const { getSchemesByMode } = await import('./themeSchemes');
    const darkSchemes = getSchemesByMode('dark');
    expect(darkSchemes.every((s) => s.mode === 'dark')).toBe(true);
  });

  it('light + dark count equals total THEME_SCHEMES', async () => {
    const { getSchemesByMode, THEME_SCHEMES } = await import('./themeSchemes');
    const lightCount = getSchemesByMode('light').length;
    const darkCount = getSchemesByMode('dark').length;
    expect(lightCount + darkCount).toBe(THEME_SCHEMES.length);
  });

  it('each scheme has a unique id', async () => {
    const { THEME_SCHEMES } = await import('./themeSchemes');
    const ids = THEME_SCHEMES.map((s) => s.id);
    const uniqueIds = new Set(ids);
    expect(uniqueIds.size).toBe(ids.length);
  });
});

describe('resolveActiveScheme', () => {
  const LIGHT = 'warm-stone';
  const DARK = 'warm-stone-dark';

  beforeEach(() => {
    // Reset matchMedia to light mode (matches=false) before each test
    vi.mocked(window.matchMedia).mockImplementation(
      (query) =>
        ({
          matches: false,
          media: query,
          onchange: null,
          addEventListener: vi.fn(),
          removeEventListener: vi.fn(),
          dispatchEvent: vi.fn(),
        }) as unknown as MediaQueryList,
    );
  });

  // ── System preset with explicit resolvedSystemTheme ─────────────

  it('returns defaultDarkTheme when preset=system and resolvedSystemTheme=dark', async () => {
    const { resolveActiveScheme } = await import('./themeSchemes');
    const result = resolveActiveScheme('system', LIGHT, DARK, 'dark');
    expect(result).toBe(DARK);
  });

  it('returns defaultLightTheme when preset=system and resolvedSystemTheme=light', async () => {
    const { resolveActiveScheme } = await import('./themeSchemes');
    const result = resolveActiveScheme('system', LIGHT, DARK, 'light');
    expect(result).toBe(LIGHT);
  });

  // ── System preset without resolvedSystemTheme (uses matchMedia) ─

  it('returns defaultLightTheme when preset=system, no resolvedSystemTheme, matchMedia light', async () => {
    const { resolveActiveScheme } = await import('./themeSchemes');
    // matchMedia returns matches=false (light) from beforeEach
    const result = resolveActiveScheme('system', LIGHT, DARK);
    expect(result).toBe(LIGHT);
  });

  it('returns defaultDarkTheme when preset=system, no resolvedSystemTheme, matchMedia dark', async () => {
    vi.mocked(window.matchMedia).mockImplementation(
      (query) =>
        ({
          matches: true,
          media: query,
          onchange: null,
          addEventListener: vi.fn(),
          removeEventListener: vi.fn(),
          dispatchEvent: vi.fn(),
        }) as unknown as MediaQueryList,
    );
    const { resolveActiveScheme } = await import('./themeSchemes');
    const result = resolveActiveScheme('system', LIGHT, DARK);
    expect(result).toBe(DARK);
  });

  // ── Non-system preset ───────────────────────────────────────────

  it('returns defaultDarkTheme when themePreset is warm-stone-dark', async () => {
    const { resolveActiveScheme } = await import('./themeSchemes');
    const result = resolveActiveScheme('warm-stone-dark', LIGHT, DARK);
    expect(result).toBe(DARK);
  });

  it('returns defaultLightTheme when themePreset is warm-stone-light', async () => {
    const { resolveActiveScheme } = await import('./themeSchemes');
    const result = resolveActiveScheme('warm-stone-light', LIGHT, DARK);
    expect(result).toBe(LIGHT);
  });

  it('returns defaultLightTheme for any other preset', async () => {
    const { resolveActiveScheme } = await import('./themeSchemes');
    // Any preset that is not 'system' and not 'warm-stone-dark' maps to light
    const result = resolveActiveScheme('warm-stone-light', LIGHT, DARK);
    expect(result).toBe(LIGHT);
  });

  // ── Custom default theme names ──────────────────────────────────

  it('uses custom default theme names in system+dark mode', async () => {
    const { resolveActiveScheme } = await import('./themeSchemes');
    const result = resolveActiveScheme('system', 'clean-slate', 'deep-ocean', 'dark');
    expect(result).toBe('deep-ocean');
  });

  it('uses custom default theme names in non-system light mode', async () => {
    const { resolveActiveScheme } = await import('./themeSchemes');
    const result = resolveActiveScheme('warm-stone-light', 'paper-white', 'midnight');
    expect(result).toBe('paper-white');
  });

  // ── edge: warm-stone-dark with different defaults ───────────────

  it('warm-stone-dark always returns the provided defaultDarkTheme', async () => {
    const { resolveActiveScheme } = await import('./themeSchemes');
    const result = resolveActiveScheme('warm-stone-dark', 'paper-white', 'obsidian-black');
    expect(result).toBe('obsidian-black');
  });
});

describe('主题边界与文字可读性', () => {
  it('全部色板在基础、卡片、悬停和选中表面维持中性文字及普通字号对比度', async () => {
    const { THEME_SCHEMES } = await import('./themeSchemes');
    const luminance = (hex: string) =>
      [1, 3, 5]
        .map((offset) => {
          const n = parseInt(hex.slice(offset, offset + 2), 16) / 255;
          return n <= 0.04045 ? n / 12.92 : ((n + 0.055) / 1.055) ** 2.4;
        })
        .reduce((sum, value, i) => sum + value * [0.2126, 0.7152, 0.0722][i], 0);
    for (const scheme of THEME_SCHEMES) {
      for (const text of ['--text-primary', '--text-secondary', '--text-tertiary']) {
        const hex = scheme.variables[text];
        expect(hex.slice(1, 3), scheme.id).toBe(hex.slice(3, 5));
        expect(hex.slice(3, 5), scheme.id).toBe(hex.slice(5, 7));
        for (const surface of [
          '--bg-base',
          '--bg-elevated',
          '--bg-elevated-hover',
          '--bg-hover',
          '--bg-active',
        ]) {
          const fg = luminance(hex),
            bg = luminance(scheme.variables[surface]);
          expect(
            (Math.max(fg, bg) + 0.05) / (Math.min(fg, bg) + 0.05),
            `${scheme.id}: ${text} on ${surface}`,
          ).toBeGreaterThanOrEqual(4.5);
        }
      }
      expect(scheme.variables).not.toHaveProperty('--text-accent');
      expect(scheme.variables).not.toHaveProperty('--text-error');
    }
  });

  it('切换与清除色板会清理旧文字偏色，同时保留独立强调色与错误语义', async () => {
    const { applyScheme, getSchemeById } = await import('./themeSchemes');
    const root = document.documentElement;
    const previous = root.getAttribute('style');
    try {
      root.style.setProperty('--text-accent', '#8ec9b8');
      root.style.setProperty('--text-warm', '#b8a88a');
      root.style.setProperty('--accent-primary', '#6699ff');
      root.style.setProperty('--text-error', '#e07a6e');
      for (const id of ['warm-stone-dark', 'deep-ocean', 'warm-stone', 'soft-cream']) {
        applyScheme(id);
        expect(root.style.getPropertyValue('--text-primary')).toBe(
          getSchemeById(id)!.mode === 'dark' ? '#ffffff' : '#111111',
        );
      }
      expect(root.style.getPropertyValue('--text-accent')).toBe('');
      expect(root.style.getPropertyValue('--text-warm')).toBe('');
      expect(root.style.getPropertyValue('--accent-primary')).toBe('#6699ff');
      expect(root.style.getPropertyValue('--text-error')).toBe('#e07a6e');
      applyScheme(null);
      expect(root.style.getPropertyValue('--text-primary')).toBe('');
      expect(root.style.getPropertyValue('--accent-primary')).toBe('#6699ff');
    } finally {
      if (previous === null) root.removeAttribute('style');
      else root.setAttribute('style', previous);
    }
  });
});
