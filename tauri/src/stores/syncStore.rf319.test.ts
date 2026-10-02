import { beforeEach, afterEach, it, expect, vi } from 'vitest';
import fixture from '../../src-tauri/src/sync/contracts/fixtures.json';
import errors from '../../src-tauri/src/sync/contracts/rf319-fixtures.json';
import codes from '../../src-tauri/src/sync/contracts/rf319-codes.json';
import {
  BackendCommandError,
  readBackendError,
  backendErrorLogDetails,
  normalizeSyncError,
} from '@/lib/backendErrorWire';
import { invokeCommand } from '@/lib/ipcClient';
import { resolveBackendErrorMessage, getBackendErrorTranslationKey } from '@/lib/backendError';
import type { BackendErrorCode } from '@/lib/generated/ipcContracts';
import i18n from '@/lib/i18n';

const native = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: (...args: unknown[]) => native.invoke(...args) }));
import { useSyncStore } from './syncStore';
import { useAuthStore } from './authStore';
beforeEach(() => {
  localStorage.clear();
  useAuthStore.setState({
    isAuthenticated: true,
    currentAccount: { id: 'rf319-account', name: 'Synthetic' },
  });
  useSyncStore.getState().clearOnVaultLock();
  native.invoke.mockReset().mockImplementation(async (command: string) => {
    if (command === 'sync_with_device')
      throw '__SYNC_ERR__:connect_failed:RF319_PRIVATE_PATH_KEY_BODY';
    if (command === 'sync_get_status') return { ...fixture.status, connectedPeers: [] };
    return null;
  });
});
afterEach(() => {
  useSyncStore.getState().clearOnVaultLock();
  vi.restoreAllMocks();
});
it('RF319 actual native rejection never persists private cause in sync history', async () => {
  await useSyncStore.getState().syncWithDevice('127.0.0.1:9999');
  const state = useSyncStore.getState();
  expect(native.invoke).toHaveBeenCalled();
  expect(state.recentResults).toHaveLength(1);
  expect(state.error).not.toContain('RF319_PRIVATE');
  expect(JSON.stringify(state.recentResults)).not.toContain('RF319_PRIVATE');
  expect(localStorage.getItem('solosoul.syncHistory.v2.rf319-account')).not.toContain(
    'RF319_PRIVATE',
  );
});

it.each([
  errors.pairing,
  errors.legacyPairing,
  '__SYNC_ERR__:pairing_pending:node-B:482913',
  '__SYNC_ERR__:pairing_pending:node-B',
])('actual pairing rejection keeps optional SAS in the current flow only: %j', async (raw) => {
  native.invoke.mockImplementation(async (command: string) => {
    if (command === 'sync_with_device') throw raw;
    if (command === 'sync_get_status')
      return { ...fixture.status, syncEnabled: true, connectedPeers: [] };
    return null;
  });
  await useSyncStore.getState().syncWithDevice('127.0.0.1:9999');
  const state = useSyncStore.getState();
  expect(state.pairingPendingPeerId).toBe('node-B');
  expect(state.pairingPendingAddr).toBe('127.0.0.1:9999');
  const sas = typeof raw === 'string' ? raw.endsWith(':482913') : 'sasCode' in raw.safeDetails;
  expect(state.pairingPendingSasCode).toBe(sas ? '482913' : null);
  expect(state.error).toBeNull();
  expect(state.recentResults).toHaveLength(0);
  expect(localStorage.getItem('solosoul.syncHistory.v2.rf319-account')).toBeNull();
});
it.each(Object.entries(errors).filter(([key]) => !['pairing', 'legacyPairing'].includes(key)))(
  'Host fixture %s survives actual native IPC without private body',
  async (_key, raw) => {
    native.invoke.mockRejectedValueOnce({
      ...raw,
      message: 'RF319_PRIVATE',
      cause: 'RF319_PRIVATE',
      safeDetails: { ...raw.safeDetails, body: 'RF319_PRIVATE' },
    });
    const error = await invokeCommand('sync_with_device', { deviceId: 'synthetic' }).catch(
      (e: unknown) => e,
    );
    expect(error).toBeInstanceOf(BackendCommandError);
    expect(readBackendError(error)).toEqual(raw);
    expect(JSON.stringify(error)).not.toContain('RF319_PRIVATE');
  },
);
it('pairing diagnostic logs omit node and SAS while Error retains validated UI metadata', async () => {
  vi.stubEnv('DEV', true);
  const warn = vi.spyOn(console, 'warn').mockImplementation(() => {});
  try {
    native.invoke.mockRejectedValueOnce({ ...errors.pairing, message: 'RF319_PRIVATE' });
    const error = await invokeCommand('sync_with_device', {}).catch((e: unknown) => e);
    expect(readBackendError(error)).toEqual(errors.pairing);
    expect(warn).toHaveBeenCalledWith("[ipc] command 'sync_with_device' failed:", {
      code: 'SYNC_PAIRING_PENDING',
      safeDetails: { stage: 'pairing' },
      retryable: false,
    });
    expect(JSON.stringify(warn.mock.calls)).not.toMatch(/482913|node-B|RF319_PRIVATE/);
    expect(backendErrorLogDetails(error)).toEqual({
      code: 'SYNC_PAIRING_PENDING',
      safeDetails: { stage: 'pairing' },
      retryable: false,
    });
  } finally {
    vi.unstubAllEnvs();
  }
});
it('malformed pairing fields never enter the pairing confirmation state', async () => {
  for (const details of [
    { stage: 'pairing' },
    { stage: 'pairing', syncPeerId: '../private', sasCode: '482913' },
    { stage: 'pairing', syncPeerId: 'node-B', sasCode: 'RF319_PRIVATE' },
    { stage: 'connect', syncPeerId: 'node-B', sasCode: '482913' },
  ]) {
    expect(readBackendError({ ...errors.pairing, safeDetails: details })).toBeNull();
  }
  native.invoke.mockRejectedValueOnce('__SYNC_ERR__:pairing_pending:node-B:RF319_PRIVATE');
  await useSyncStore.getState().syncWithDevice('target');
  expect(useSyncStore.getState().pairingPendingPeerId).toBeNull();
  expect(useSyncStore.getState().error).toBe('SYNC_PAIRING_INVALID');
});
it('real persisted failed history is sanitized on account activation, with success statistics retained', () => {
  const key = 'solosoul.syncHistory.v2.rf319-history';
  const success = { ...fixture.result, at: 1 };
  localStorage.setItem(
    key,
    JSON.stringify([
      {
        ...success,
        failed: true,
        summary: 'RF319_PRIVATE',
        errorSummary: '__SYNC_ERR__:connect_failed:RF319_PRIVATE',
      },
      success,
    ]),
  );
  useAuthStore.setState({
    isAuthenticated: true,
    currentAccount: { id: 'rf319-history', name: 'Synthetic' },
  });
  const rows = useSyncStore.getState().recentResults;
  expect(rows).toHaveLength(2);
  expect(rows[0].errorSummary).toBe('SYNC_CONNECT_FAILED');
  expect(rows[1]).toEqual(success);
  expect(localStorage.getItem(key)).not.toContain('RF319_PRIVATE');
});
it('locked and expired request preflight reject machine errors before native invoke', async () => {
  vi.stubEnv('MODE', 'production');
  try {
    useAuthStore.setState({ isAuthenticated: false, currentAccount: null });
    await expect(invokeCommand('sync_with_device', {})).rejects.toMatchObject({
      backend: { code: 'VAULT_LOCKED' },
    });
    useAuthStore.setState({
      isAuthenticated: true,
      currentAccount: { id: 'rf319-account', name: 'Synthetic' },
    });
    await expect(
      invokeCommand('sync_with_device', {}, { requestIsCurrent: () => false }),
    ).rejects.toMatchObject({ backend: { code: 'SESSION_EXPIRED' } });
    expect(native.invoke).not.toHaveBeenCalled();
  } finally {
    vi.unstubAllEnvs();
  }
});
it('a delayed pairing callback cannot restore metadata after vault lock', async () => {
  let reject!: (value: unknown) => void;
  native.invoke.mockReturnValueOnce(
    new Promise((_resolve, r) => {
      reject = r;
    }),
  );
  const pending = useSyncStore.getState().syncWithDevice('target');
  useAuthStore.setState({ isAuthenticated: false, currentAccount: null });
  reject(errors.pairing);
  await pending;
  expect(useSyncStore.getState().pairingPendingPeerId).toBeNull();
  expect(useSyncStore.getState().pairingPendingSasCode).toBeNull();
  expect(useSyncStore.getState().recentResults).toHaveLength(0);
});
it('all sync codes have independent bilingual display messages and uncertain outcomes remain explicit', async () => {
  for (const language of ['zh-CN', 'en-US']) {
    await i18n.changeLanguage(language);
    for (const code of codes) {
      expect(i18n.exists(getBackendErrorTranslationKey(code as BackendErrorCode))).toBe(true);
      const message = resolveBackendErrorMessage(new BackendCommandError(normalizeSyncError(code)));
      expect(message).toBe(i18n.t(getBackendErrorTranslationKey(code as BackendErrorCode)));
      expect(message).not.toMatch(/backend_|RF319_PRIVATE/);
      expect(message.length).toBeGreaterThan(2);
    }
    const timeout = resolveBackendErrorMessage(
      new BackendCommandError(readBackendError(errors.timeout)!),
    );
    const refusal = resolveBackendErrorMessage(
      new BackendCommandError(readBackendError(errors.refused)!),
    );
    const unknown = resolveBackendErrorMessage(
      new BackendCommandError(readBackendError(errors.unconfirmed)!),
    );
    expect(new Set([timeout, refusal, unknown]).size).toBe(3);
  }
});

it('RF319 a code-only legacy pairing event stays classified without inventing node or SAS', async () => {
  native.invoke.mockRejectedValueOnce('SYNC_PAIRING_PENDING');
  await useSyncStore.getState().syncWithDevice('target');
  expect(useSyncStore.getState().error).toBe('SYNC_PAIRING_PENDING');
  expect(useSyncStore.getState().pairingPendingPeerId).toBeNull();
  expect(useSyncStore.getState().pairingPendingSasCode).toBeNull();
  expect(readBackendError(normalizeSyncError('SYNC_PAIRING_PENDING'))?.code).toBe(
    'SYNC_PAIRING_PENDING',
  );
});

it.each([
  ['__SYNC_ERR__:connect_failed:\nRF319_PRIVATE', 'SYNC_CONNECT_FAILED'],
  ['No account is unlocked', 'VAULT_LOCKED'],
  ['Vault is not unlocked', 'VAULT_LOCKED'],
  ['IMPORT_OPERATIONS_ACTIVE', 'VAULT_BUSY'],
])(
  'RF319 old multiline/guard rejection %s stays classified without private cause',
  async (raw, code) => {
    native.invoke.mockRejectedValueOnce(raw);
    const error = await invokeCommand('sync_with_device', { deviceId: 'synthetic' }).catch(
      (e: unknown) => e,
    );
    expect(readBackendError(error)?.code).toBe(code);
    expect(JSON.stringify(error)).not.toContain('RF319_PRIVATE');
  },
);
