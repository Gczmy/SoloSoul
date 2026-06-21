import { describe, it, expect, vi, afterEach } from 'vitest';
import type { AccentPreset } from '@/types';

// Mock Tauri invoke (imported but unused by pure functions)
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

// Mock themeSchemes (imported but unused by pure functions under test)
vi.mock('./themeSchemes', () => ({
  applyScheme: vi.fn(),
  resolveActiveScheme: vi.fn().mockReturnValue('warm-stone'),
  getSchemeById: vi.fn().mockReturnValue({ variables: { '--bg-base': '#1c1c1e' } }),
}));

describe('hexToRgb', () => {
  it('converts 6-character hex to RGB', async () => {
    const { hexToRgb } = await import('./theme');
    expect(hexToRgb('#5B7C99')).toEqual([91, 124, 153]);
  });

  it('converts 3-character hex to RGB', async () => {
    const { hexToRgb } = await import('./theme');
    // #abc → #aabbcc → [170, 187, 204]
    expect(hexToRgb('#abc')).toEqual([170, 187, 204]);
  });

  it('returns null for invalid hex length', async () => {
    const { hexToRgb } = await import('./theme');
    expect(hexToRgb('#ab')).toBeNull();
    expect(hexToRgb('#1234567')).toBeNull();
  });

  it('returns null for empty string', async () => {
    const { hexToRgb } = await import('./theme');
    expect(hexToRgb('')).toBeNull();
  });

  it('returns null for non-hex characters', async () => {
    const { hexToRgb } = await import('./theme');
    expect(hexToRgb('#xyzxyz')).toBeNull();
  });

  it('handles hex without hash prefix', async () => {
    const { hexToRgb } = await import('./theme');
    // The function strips #, so 'abc' is treated as 3-char
    expect(hexToRgb('abc')).toEqual([170, 187, 204]);
  });

  it('handles boundaries 0 and 255', async () => {
    const { hexToRgb } = await import('./theme');
    expect(hexToRgb('#000000')).toEqual([0, 0, 0]);
    expect(hexToRgb('#FFFFFF')).toEqual([255, 255, 255]);
  });

  it('handles mixed case hex', async () => {
    const { hexToRgb } = await import('./theme');
    expect(hexToRgb('#aBcDeF')).toEqual([171, 205, 239]);
  });
});

describe('rgbToHex', () => {
  it('converts RGB to 6-character hex', async () => {
    const { rgbToHex } = await import('./theme');
    expect(rgbToHex(91, 124, 153)).toBe('#5b7c99');
  });

  it('handles black and white', async () => {
    const { rgbToHex } = await import('./theme');
    expect(rgbToHex(0, 0, 0)).toBe('#000000');
    expect(rgbToHex(255, 255, 255)).toBe('#ffffff');
  });

  it('clamps values below 0', async () => {
    const { rgbToHex } = await import('./theme');
    const result = rgbToHex(-10, -50, -100);
    expect(result).toBe('#000000');
  });

  it('clamps values above 255', async () => {
    const { rgbToHex } = await import('./theme');
    const result = rgbToHex(300, 400, 500);
    expect(result).toBe('#ffffff');
  });

  it('rounds fractional values', async () => {
    const { rgbToHex } = await import('./theme');
    const result = rgbToHex(91.4, 124.6, 153.2);
    // 91.4 → 91, 124.6 → 125, 153.2 → 153
    expect(result).toBe('#5b7d99');
  });
});

describe('adjustAccentHover', () => {
  it('darkens a color by 12%', async () => {
    const { adjustAccentHover } = await import('./theme');
    const result = adjustAccentHover('#5B7C99');
    // Each channel: c + c * (-0.12) = c * 0.88
    // R: 91 * 0.88 = 80.08 → 80
    // G: 124 * 0.88 = 109.12 → 109
    // B: 153 * 0.88 = 134.64 → 135
    expect(result).toBe('#506d87');
  });

  it('darkens white to near-gray', async () => {
    const { adjustAccentHover } = await import('./theme');
    const result = adjustAccentHover('#FFFFFF');
    // 255 * 0.88 = 224.4 → 224
    expect(result).toBe('#e0e0e0');
  });

  it('darkens black remains black', async () => {
    const { adjustAccentHover } = await import('./theme');
    const result = adjustAccentHover('#000000');
    expect(result).toBe('#000000');
  });

  it('returns original hex if invalid', async () => {
    const { adjustAccentHover } = await import('./theme');
    const result = adjustAccentHover('invalid');
    expect(result).toBe('invalid');
  });
});

describe('applyAccentColor', () => {
  afterEach(() => {
    // Clean up after each test: remove data-accent and custom properties
    document.documentElement.removeAttribute('data-accent');
    document.documentElement.style.removeProperty('--accent-primary');
    document.documentElement.style.removeProperty('--accent-hover');
  });

  it('sets data-accent attribute for preset colors', async () => {
    const { applyAccentColor } = await import('./theme');
    applyAccentColor('ocean');
    expect(document.documentElement.getAttribute('data-accent')).toBe('ocean');
  });

  it('removes inline CSS properties for preset (uses [data-accent] CSS selectors)', async () => {
    const { applyAccentColor } = await import('./theme');
    applyAccentColor('amber');
    expect(document.documentElement.style.getPropertyValue('--accent-primary')).toBe('');
    expect(document.documentElement.style.getPropertyValue('--accent-hover')).toBe('');
  });

  it('sets custom accent color and hover variant', async () => {
    const { applyAccentColor } = await import('./theme');
    applyAccentColor('custom', '#FF6B6B');
    expect(document.documentElement.getAttribute('data-accent')).toBe('custom');
    expect(document.documentElement.style.getPropertyValue('--accent-primary')).toBe('#FF6B6B');
    // Hover: 255*0.88=224, 107*0.88=94, 107*0.88=94 → #e05e5e
    expect(document.documentElement.style.getPropertyValue('--accent-hover')).toBe('#e05e5e');
  });

  it('falls back to ocean for unknown preset', async () => {
    const { applyAccentColor } = await import('./theme');
    applyAccentColor('nonexistent' as AccentPreset);
    expect(document.documentElement.getAttribute('data-accent')).toBe('ocean');
  });

  it('handles empty custom hex gracefully — falls back to ocean', async () => {
    const { applyAccentColor } = await import('./theme');
    // custom without hex — ACCENT_COLORS['custom'] is '' (falsy), falls back to 'ocean'
    applyAccentColor('custom');
    expect(document.documentElement.getAttribute('data-accent')).toBe('ocean');
    expect(document.documentElement.style.getPropertyValue('--accent-primary')).toBe('');
  });
});
