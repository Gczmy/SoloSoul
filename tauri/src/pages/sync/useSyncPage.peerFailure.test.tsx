import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { act, renderHook } from '@testing-library/react';
import { useSyncPage } from './useSyncPage';
import { useSyncStore, type SyncPeer } from '@/stores/syncStore';

const peer: SyncPeer = {
  id: 'peer-1',
  name: 'Old Mac',
  addr: '192.0.2.1:42069',
  fingerprint: 'aabbccdd',
  trusted: false,
  lastSeen: '',
};

const originalActions = useSyncStore.getState();

describe('useSyncPage peer mutation failure', () => {
  beforeEach(() => {
    useSyncStore.setState({
      connectedPeers: [peer],
      incomingPairingRequest: null,
      pairingPendingPeerId: null,
      syncEnabled: false,
      lastResult: null,
      error: 'trust denied',
      loadStatus: vi.fn().mockResolvedValue(undefined),
      loadListenAddr: vi.fn().mockResolvedValue(undefined),
      loadAutoSyncStatus: vi.fn().mockResolvedValue(undefined),
      loadUiPrefsSync: vi.fn().mockResolvedValue(undefined),
      loadConflicts: vi.fn().mockResolvedValue(undefined),
      initNsdFailedListener: vi.fn().mockResolvedValue(() => {}),
      trustPeer: vi.fn().mockRejectedValue(new Error('trust denied')),
      forgetPeer: vi.fn().mockRejectedValue(new Error('forget denied')),
    });
  });

  afterEach(() => {
    useSyncStore.setState({
      connectedPeers: [],
      pairingPendingPeerId: null,
      error: null,
      loadStatus: originalActions.loadStatus,
      loadListenAddr: originalActions.loadListenAddr,
      loadAutoSyncStatus: originalActions.loadAutoSyncStatus,
      loadUiPrefsSync: originalActions.loadUiPrefsSync,
      loadConflicts: originalActions.loadConflicts,
      initNsdFailedListener: originalActions.initNsdFailedListener,
      trustPeer: originalActions.trustPeer,
      forgetPeer: originalActions.forgetPeer,
    });
  });

  it('信任失败时保留手动配对确认目标并等待重试', async () => {
    const { result, unmount } = renderHook(() => useSyncPage());
    act(() => result.current.handleOpenPairTarget(peer));

    await act(async () => {
      await result.current.handleTrustPending();
    });
    expect(result.current.pairTarget).toEqual(peer);
    expect(result.current.pairWaitState).toBe('idle');
    unmount();
  });

  it('忘记失败时保留二次确认目标并等待重试', async () => {
    const { result, unmount } = renderHook(() => useSyncPage());
    act(() => result.current.handleForgetRequest(peer));

    await act(async () => {
      await result.current.handleForgetConfirm();
    });
    expect(result.current.forgetTarget).toEqual(peer);
    unmount();
  });

  it('发起方信任失败时不进入等待对端的重试循环', async () => {
    useSyncStore.setState({ pairingPendingPeerId: peer.id, pairingPendingAddr: peer.addr });
    const { result, unmount } = renderHook(() => useSyncPage());

    await act(async () => {
      await result.current.handleConfirmPairing();
    });
    expect(result.current.pairWaitState).toBe('idle');
    expect(useSyncStore.getState().trustPeer).toHaveBeenCalledWith(peer.id, true, peer.fingerprint);
    unmount();
  });
});
