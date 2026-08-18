import { describe, it, expect } from 'vitest';
import { isMobilePlatformSync, isMacOSSync, isWindowsSync, supportsHover } from './platform';

describe('isMobilePlatformSync', () => {
  it('returns false when platform cache is not primed (default)', () => {
    expect(isMobilePlatformSync()).toBe(false);
  });
});

describe('isMacOSSync / isWindowsSync', () => {
  it('return false when platform cache is not primed (default)', () => {
    expect(isMacOSSync()).toBe(false);
    expect(isWindowsSync()).toBe(false);
  });
});

describe('supportsHover', () => {
  it('returns true when matchMedia is unavailable (jsdom fallback)', () => {
    expect(supportsHover()).toBe(true);
  });
});
