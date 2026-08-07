import { describe, expect, it } from 'vitest';
import { MASK_PLACEHOLDER, maskValue, shouldMaskSensitivity } from './masking';

describe('masking (P036)', () => {
  it('only public is never masked', () => {
    expect(shouldMaskSensitivity('public')).toBe(false);
    expect(shouldMaskSensitivity('internal')).toBe(true);
    expect(shouldMaskSensitivity('sensitive')).toBe(true);
    expect(shouldMaskSensitivity('critical')).toBe(true);
  });

  it('maskValue uses unified 8-dot placeholder', () => {
    expect(MASK_PLACEHOLDER).toBe('••••••••');
    expect(maskValue('abc', 'internal')).toBe(MASK_PLACEHOLDER);
    expect(maskValue('abc', 'critical')).toBe(MASK_PLACEHOLDER);
    expect(maskValue('abc', 'public')).toBe('abc');
  });
});
