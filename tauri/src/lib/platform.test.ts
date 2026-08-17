import { describe, it, expect } from 'vitest';
import { canPrefetchOnMobile } from './platform';

describe('canPrefetchOnMobile', () => {
  it('returns true only on a confirmed fast connection (4g, no saveData)', () => {
    expect(canPrefetchOnMobile({ saveData: false, effectiveType: '4g' })).toBe(true);
    expect(canPrefetchOnMobile({ effectiveType: '4g' })).toBe(true);
  });

  it('returns false when saveData is enabled', () => {
    expect(canPrefetchOnMobile({ saveData: true, effectiveType: '4g' })).toBe(false);
  });

  it('returns false on slow networks (slow-2g/2g/3g)', () => {
    expect(canPrefetchOnMobile({ effectiveType: 'slow-2g' })).toBe(false);
    expect(canPrefetchOnMobile({ effectiveType: '2g' })).toBe(false);
    expect(canPrefetchOnMobile({ effectiveType: '3g' })).toBe(false);
  });

  it('returns false when connection info is unavailable (iOS WKWebView / undefined)', () => {
    expect(canPrefetchOnMobile(undefined)).toBe(false);
    expect(canPrefetchOnMobile(null)).toBe(false);
    expect(canPrefetchOnMobile({})).toBe(false);
  });
});
