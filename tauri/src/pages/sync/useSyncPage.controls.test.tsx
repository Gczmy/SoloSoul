import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { act, renderHook } from '@testing-library/react';
import { useSyncPage } from './useSyncPage';
import { useSyncStore, type SyncPeer } from '@/stores/syncStore';

const originalState = useSyncStore.getState();
const peer: SyncPeer = {
  id: 'peer-1',
  name: 'Laptop',
  addr: '192.0.2.1:42069',
  fingerprint: 'aabbccdd',
  trusted: false,
  lastSeen: '',
};

describe('useSyncPage controls', () => {
  beforeEach(() => {
    useSyncStore.setState({
      connectedPeers: [],
      incomingPairingRequest: null,
      pairingPendingPeerId: null,
      syncEnabled: false,
      autoSyncEnabled: false,
      uiPrefsSyncEnabled: true,
      isLoading: false,
      lastResult: null,
      loadStatus: vi.fn().mockResolvedValue(undefined),
      loadListenAddr: vi.fn().mockResolvedValue(undefined),
      loadAutoSyncStatus: vi.fn().mockResolvedValue(undefined),
      loadUiPrefsSync: vi.fn().mockResolvedValue(undefined),
      loadConflicts: vi.fn().mockResolvedValue(undefined),
      initNsdFailedListener: vi.fn().mockResolvedValue(() => {}),
      discoverDevices: vi.fn().mockResolvedValue(undefined),
      syncWithDevice: vi.fn().mockResolvedValue(undefined),
      enable: vi.fn().mockResolvedValue(undefined),
      setAutoSyncEnabled: vi.fn().mockResolvedValue(undefined),
      setUiPrefsSyncEnabled: vi.fn().mockResolvedValue(undefined),
      clearPairingPending: vi.fn(),
    });
  });

  afterEach(() => {
    useSyncStore.setState(originalState);
  });

  it('reads the latest switch state and ignores toggle clicks while a sync operation is loading', async () => {
    const { result } = renderHook(() => useSyncPage());
    const actions = useSyncStore.getState();
    await act(async () => {
      await result.current.handleToggleSync();
      await result.current.handleToggleAutoSync();
      await result.current.handleToggleUiPrefsSync();
    });
    expect(actions.enable).toHaveBeenCalledExactlyOnceWith(true);
    expect(actions.setAutoSyncEnabled).toHaveBeenCalledExactlyOnceWith(true);
    expect(actions.setUiPrefsSyncEnabled).toHaveBeenCalledExactlyOnceWith(false);

    act(() => useSyncStore.setState({ syncEnabled: true, autoSyncEnabled: true }));
    await act(async () => {
      await result.current.handleToggleSync();
      await result.current.handleToggleAutoSync();
    });
    expect(actions.enable).toHaveBeenLastCalledWith(false);
    expect(actions.setAutoSyncEnabled).toHaveBeenLastCalledWith(false);

    act(() => useSyncStore.setState({ isLoading: true }));
    await act(async () => {
      await result.current.handleToggleSync();
      await result.current.handleToggleAutoSync();
      await result.current.handleToggleUiPrefsSync();
    });
    expect(actions.enable).toHaveBeenCalledTimes(2);
    expect(actions.setAutoSyncEnabled).toHaveBeenCalledTimes(2);
    expect(actions.setUiPrefsSyncEnabled).toHaveBeenCalledTimes(1);
  });

  it('separates discovery and sync actions from ignored pairing and forget confirmations', async () => {
    act(() => useSyncStore.setState({ connectedPeers: [peer] }));
    const { result } = renderHook(() => useSyncPage());
    expect(result.current.pendingPeer).toEqual(peer);

    await act(async () => {
      await result.current.handleDiscover();
      await result.current.handleSyncWithDevice(peer.id);
      await result.current.handleScanSync(peer.addr);
    });
    expect(useSyncStore.getState().discoverDevices).toHaveBeenCalledWith(5000);
    expect(useSyncStore.getState().syncWithDevice).toHaveBeenCalledWith(peer.id);
    expect(useSyncStore.getState().syncWithDevice).toHaveBeenCalledWith(peer.addr);

    act(() => result.current.handleIgnorePending());
    expect(result.current.pendingPeer).toBeNull();
    act(() => result.current.handleOpenPairTarget(peer));
    expect(result.current.pairTarget).toEqual(peer);
    act(() => result.current.handleForgetRequest(peer));
    expect(result.current.forgetTarget).toEqual(peer);
    act(() => result.current.handleForgetCancel());
    expect(result.current.forgetTarget).toBeNull();
    act(() => result.current.handleOpenConflictDialog());
    expect(result.current.conflictDialogOpen).toBe(true);
  });
});
