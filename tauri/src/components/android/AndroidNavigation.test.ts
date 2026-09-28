import { describe, expect, it } from 'vitest';
import { androidNewObjectUrl } from './AndroidNavigation';

describe('Android create-object destination', () => {
  it('preserves a valid encoded custom page id', () => {
    expect(androidNewObjectUrl('/workspace/custom/a%25b', '')).toBe('/editor?parentId=a%25b');
  });

  it('falls back to a new root object for a malformed deep-link escape', () => {
    expect(androidNewObjectUrl('/workspace/custom/bad%ZZ', '')).toBe('/editor');
  });
});
