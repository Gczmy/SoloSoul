import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import type { IpcEvents } from '@/lib/generated/ipcContracts';
import fixture from '../../src-tauri/src/sync/contracts/fixtures.json';
const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  listeners: new Map<string, (event: { payload: unknown }) => void>(),
}));
vi.mock('@tauri-apps/api/core', () => ({ invoke: (...args: unknown[]) => mocks.invoke(...args) }));
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(async (name: string, callback: (event: { payload: unknown }) => void) => {
    mocks.listeners.set(name, callback);
    return vi.fn();
  }),
}));
import { useSyncStore, __resetSyncCompletedMergeForTest } from './syncStore';
import { useAuthStore } from './authStore';

beforeEach(() => {
  localStorage.clear();
  mocks.listeners.clear();
  __resetSyncCompletedMergeForTest();
  useAuthStore.setState({
    isAuthenticated: true,
    currentAccount: { id: 'synthetic-account', name: 'Synthetic' },
  });
  useSyncStore.getState().clearOnVaultLock();
  mocks.invoke
    .mockReset()
    .mockImplementation(async (name: string) =>
      name === 'sync_get_status'
        ? { ...fixture.status, connectedPeers: [] }
        : name === 'sync_list_conflicts'
          ? []
          : null,
    );
});
afterEach(() => {
  useSyncStore.getState().clearOnVaultLock();
  vi.restoreAllMocks();
});

it('RF305 late pairing, completion, conflict and NSD callbacks cannot refill a locked store', async () => {
  const s = useSyncStore.getState();
  await Promise.all([
    s.initPairingRequestListener(),
    s.initSyncCompletedListener(),
    s.initConflictListener(),
    s.initNsdFailedListener(),
  ]);
  const queued = [...mocks.listeners.entries()];
  s.clearOnVaultLock();
  mocks.invoke.mockClear();
  const payloads: Record<string, unknown> = {
    'sync-pairing-request': fixture.pairing,
    'sync-completed': fixture.completed,
    'sync-conflicts-updated': fixture.conflictsUpdated,
    'sync-nsd-failed': fixture.nsdFailed,
  };
  for (const [name, callback] of queued) callback({ payload: payloads[name] });
  expect(useSyncStore.getState()).toMatchObject({
    incomingPairingRequest: null,
    lastResult: null,
    recentResults: [],
    hasUnreadConflicts: false,
    error: null,
  });
  expect(mocks.invoke).not.toHaveBeenCalled();
});

it('RF305 current SAS updates and merged inbound totals preserve pairing identity and both directions', async () => {
  const s = useSyncStore.getState();
  await s.initPairingRequestListener();
  const first: IpcEvents['sync-pairing-request'] = fixture.pairing;
  mocks.listeners.get('sync-pairing-request')!({ payload: first });
  mocks.listeners.get('sync-pairing-request')!({ payload: { ...first, sasCode: '932841' } });
  expect(useSyncStore.getState().incomingPairingRequest).toMatchObject({
    id: first.nodeId,
    fingerprint: first.fingerprint,
    addr: first.addr,
    sasCode: '932841',
  });
  await s.initSyncCompletedListener();
  const complete: IpcEvents['sync-completed'] = fixture.completed;
  const callback = mocks.listeners.get('sync-completed')!;
  callback({ payload: complete });
  callback({
    payload: { ...complete, examined: 2, applied: 0, skipped: 2, conflicts: 0, outboundRecords: 3 },
  });
  expect(useSyncStore.getState().lastResult).toMatchObject({
    peerNodeId: complete.peerNodeId,
    inbound: true,
    examined: 9,
    applied: 4,
    skipped: 5,
    conflictCount: 2,
    outboundRecords: 8,
  });
  expect(useSyncStore.getState().recentResults).toHaveLength(1);
  await new Promise<void>((resolve) => setTimeout(resolve, 0));
});
