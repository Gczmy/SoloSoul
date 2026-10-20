import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  applyMotionPlatform,
  applyNativeMotionPreference,
  prefersReducedMotion,
} from './motionPolicy';

beforeEach(() => {
  vi.spyOn(window, 'matchMedia').mockReturnValue({ matches: false } as MediaQueryList);
});

afterEach(() => {
  for (const key of ['motionPolicy', 'systemReduceMotion', 'reduceMotion', 'userReduceMotion'])
    delete document.documentElement.dataset[key];
  vi.restoreAllMocks();
});

describe('application motion policy', () => {
  it.each(['windows', 'macos', 'linux'])(
    '%s animates despite system and native preferences',
    (platform) => {
      vi.spyOn(window, 'matchMedia').mockReturnValue({ matches: true } as MediaQueryList);
      applyNativeMotionPreference(platform, true);
      expect(document.documentElement.dataset.systemReduceMotion).toBe('true');
      expect(document.documentElement.dataset.reduceMotion).toBe('false');
      expect(prefersReducedMotion()).toBe(false);
      document.documentElement.dataset.userReduceMotion = 'true';
      expect(prefersReducedMotion()).toBe(true);
    },
  );

  it.each(['android', 'ios'])('%s retains system reduction', (platform) => {
    applyNativeMotionPreference(platform, true);
    expect(prefersReducedMotion()).toBe(true);
    applyNativeMotionPreference(platform, false);
    expect(prefersReducedMotion()).toBe(false);
    vi.spyOn(window, 'matchMedia').mockReturnValue({ matches: true } as MediaQueryList);
    expect(prefersReducedMotion()).toBe(true);
  });

  it('switching to a mobile platform restores the preserved native preference', () => {
    applyNativeMotionPreference('windows', true);
    applyMotionPlatform('android');
    expect(prefersReducedMotion()).toBe(true);
  });
});
