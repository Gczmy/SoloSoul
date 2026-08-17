import { describe, it, expect } from 'vitest';
import { resolveRouteLoader, prefetchRoute } from './routeLoaders';

describe('resolveRouteLoader', () => {
  it('resolves known route paths to a loader function', () => {
    expect(typeof resolveRouteLoader('/')).toBe('function');
    expect(typeof resolveRouteLoader('/settings')).toBe('function');
    expect(typeof resolveRouteLoader('/llm-chat')).toBe('function');
  });

  it('strips query/hash before lookup', () => {
    expect(resolveRouteLoader('/workspace?section=identity')).toBe(
      resolveRouteLoader('/workspace'),
    );
    expect(resolveRouteLoader('/settings/trash#x')).toBe(resolveRouteLoader('/settings/trash'));
  });

  it('falls back to the parent page loader for dynamic segments', () => {
    expect(resolveRouteLoader('/workspace/custom/abc')).toBe(resolveRouteLoader('/workspace'));
    expect(resolveRouteLoader('/editor/123')).toBe(resolveRouteLoader('/editor'));
  });

  it('returns undefined for unknown paths', () => {
    expect(resolveRouteLoader('/no-such-page')).toBeUndefined();
    expect(resolveRouteLoader('')).toBeUndefined();
  });
});

describe('prefetchRoute', () => {
  it('is a silent no-op for unknown paths', () => {
    expect(() => prefetchRoute('/no-such-page')).not.toThrow();
  });
});
