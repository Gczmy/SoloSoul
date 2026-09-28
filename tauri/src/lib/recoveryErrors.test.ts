import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { TFunction } from 'i18next';
import { checkRecoveryIdConflict, friendlyConnectError } from './recoveryErrors';

const mockInvoke = vi.fn();
vi.mock('@/lib/ipcClient', () => ({
  invokeCommand: (...args: unknown[]) => mockInvoke(...args),
}));

const t = ((key: string) => `translated:${key}`) as TFunction;

describe('recovery connection guidance', () => {
  it('distinguishes network timeout from a failed identity check', () => {
    expect(friendlyConnectError('Connection TIMED OUT', t)).toBe(
      'translated:common:recovery_connect_timeout',
    );
    expect(friendlyConnectError('Identity verification failed', t)).toBe(
      'translated:common:recovery_mitm',
    );
  });

  it('guides retry when the host closes before transfer and translates a known Rust error', () => {
    expect(friendlyConnectError('read prefix failed: unexpected EOF', t)).toBe(
      'translated:common:recovery_host_closed_early',
    );
    expect(friendlyConnectError('Account ID already exists', t)).toBe(
      'translated:common:account_id_exists',
    );
  });

  it('preserves an unknown diagnostic instead of inventing a cause', () => {
    expect(friendlyConnectError('opaque recovery failure', t)).toBe('opaque recovery failure');
  });
});

describe('recovery account conflict precheck', () => {
  beforeEach(() => mockInvoke.mockReset());

  it('does not query the Vault without an account ID', async () => {
    expect(await checkRecoveryIdConflict(null)).toBe(false);
    expect(mockInvoke).not.toHaveBeenCalled();
  });

  it('detects the exact account ID before the user enters a recovery password', async () => {
    mockInvoke.mockResolvedValueOnce([{ id: 'account-other' }, { id: 'account-target' }]);

    expect(await checkRecoveryIdConflict('account-target')).toBe(true);
    expect(mockInvoke).toHaveBeenCalledWith('vault_list_accounts');
  });

  it('returns no precheck conflict when the account is absent or listing fails', async () => {
    mockInvoke.mockResolvedValueOnce([{ id: 'account-other' }]);
    expect(await checkRecoveryIdConflict('account-target')).toBe(false);

    mockInvoke.mockRejectedValueOnce(new Error('vault unavailable'));
    expect(await checkRecoveryIdConflict('account-target')).toBe(false);
  });
});
