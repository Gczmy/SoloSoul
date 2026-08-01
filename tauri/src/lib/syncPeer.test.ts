import { describe, it, expect } from 'vitest';
import { formatPeerName } from './syncPeer';

describe('formatPeerName', () => {
  it('uses SoloSoul-<fp 前 8 位> when fingerprint is present', () => {
    expect(
      formatPeerName({ id: 'node-1', fingerprint: 'a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6' }),
    ).toBe('SoloSoul-a1b2c3d4');
  });

  it('truncates short fingerprints without panic', () => {
    expect(formatPeerName({ id: 'node-1', fingerprint: 'ab12' })).toBe('SoloSoul-ab12');
  });

  it('falls back to name when fingerprint is empty', () => {
    expect(formatPeerName({ id: 'node-1', name: 'My Phone', fingerprint: '' })).toBe('My Phone');
  });

  it('falls back to node_id when no fingerprint and no name', () => {
    expect(formatPeerName({ id: 'node-1', fingerprint: '' })).toBe('node-1');
  });
});
