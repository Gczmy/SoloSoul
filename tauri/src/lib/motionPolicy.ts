/** Desktop animations are an application preference; mobile keeps system accessibility rules. */
export function followsSystemMotion() {
  return document.documentElement.dataset.motionPolicy !== 'application';
}

export function applyMotionPlatform(platform: string) {
  const root = document.documentElement;
  const desktop = ['windows', 'macos', 'linux'].includes(platform);
  root.dataset.motionPolicy = desktop ? 'application' : 'system';
  root.dataset.reduceMotion = String(!desktop && root.dataset.systemReduceMotion === 'true');
}

export function applyNativeMotionPreference(platform: string, reduced: boolean) {
  document.documentElement.dataset.systemReduceMotion = String(reduced);
  applyMotionPlatform(
    platform === 'other' ? (document.documentElement.dataset.platform ?? platform) : platform,
  );
}

export function prefersReducedMotion() {
  const root = document.documentElement;
  return (
    root.dataset.userReduceMotion === 'true' ||
    (followsSystemMotion() &&
      (root.dataset.reduceMotion === 'true' ||
        window.matchMedia?.('(prefers-reduced-motion: reduce)').matches === true))
  );
}
