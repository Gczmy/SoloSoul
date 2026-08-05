import { describe, it, expect } from 'vitest';
import { formatDiscoveredName, formatPeerName } from './syncPeer';

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

describe('formatDiscoveredName', () => {
  it('keeps friendly SoloSoul-<fp8> names as-is', () => {
    expect(formatDiscoveredName({ name: 'SoloSoul-a1b2c3d4' })).toBe('SoloSoul-a1b2c3d4');
  });

  it('truncates bare node_<uuid> names', () => {
    expect(
      formatDiscoveredName({ name: 'node_f2c22bc0a1b2c3d4e5f6a7b8c9d0e1f2' }),
    ).toBe('node_f2c22bc0…');
  });

  it('strips mDNS fullname suffix and truncates node_<uuid>', () => {
    expect(
      formatDiscoveredName({
        name: 'node_f2c22bc0a1b2c3d4e5f6a7b8c9d0e1f2._solosoul._tcp.local.',
      }),
    ).toBe('node_f2c22bc0…');
  });

  it('strips .local. suffix from hostnames', () => {
    expect(formatDiscoveredName({ name: 'macbook.local.' })).toBe('macbook');
  });

  it('falls back to Unknown device for empty name', () => {
    expect(formatDiscoveredName({ name: '' })).toBe('Unknown device');
    expect(formatDiscoveredName({})).toBe('Unknown device');
  });
});
