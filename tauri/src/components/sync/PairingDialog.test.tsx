import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { PairingDialog } from './PairingDialog';
import type { SyncPeer } from '@/stores/syncStore';

const mockPeer: SyncPeer = {
  id: 'node_b',
  name: 'Device B',
  addr: '127.0.0.1:12345',
  fingerprint: 'abcd1234abcd1234',
  trusted: false,
  lastSeen: '2s ago',
};

describe('PairingDialog', () => {
  it('does not render when closed', () => {
    render(
      <PairingDialog
        isOpen={false}
        peer={mockPeer}
        onTrust={vi.fn()}
        onIgnore={vi.fn()}
      />,
    );
    expect(screen.queryByText(mockPeer.fingerprint)).not.toBeInTheDocument();
  });

  it('renders peer name and fingerprint when open', () => {
    render(
      <PairingDialog
        isOpen={true}
        peer={mockPeer}
        onTrust={vi.fn()}
        onIgnore={vi.fn()}
      />,
    );
    expect(screen.getByText(mockPeer.name)).toBeInTheDocument();
    expect(screen.getByText(mockPeer.fingerprint)).toBeInTheDocument();
  });

  it('calls onTrust when clicking Trust & Pair', () => {
    const onTrust = vi.fn();
    render(
      <PairingDialog
        isOpen={true}
        peer={mockPeer}
        onTrust={onTrust}
        onIgnore={vi.fn()}
      />,
    );
    fireEvent.click(screen.getByRole('button', { name: /trust & pair/i }));
    expect(onTrust).toHaveBeenCalledTimes(1);
  });

  it('calls onIgnore when clicking Ignore', () => {
    const onIgnore = vi.fn();
    render(
      <PairingDialog
        isOpen={true}
        peer={mockPeer}
        onTrust={vi.fn()}
        onIgnore={onIgnore}
      />,
    );
    fireEvent.click(screen.getByRole('button', { name: /ignore/i }));
    expect(onIgnore).toHaveBeenCalledTimes(1);
  });

  it('renders fallback when fingerprint is missing', () => {
    const peerWithoutFingerprint: SyncPeer = { ...mockPeer, fingerprint: '' };
    render(
      <PairingDialog
        isOpen={true}
        peer={peerWithoutFingerprint}
        onTrust={vi.fn()}
        onIgnore={vi.fn()}
      />,
    );
    expect(screen.getByText(/no fingerprint available/i)).toBeInTheDocument();
  });
});
