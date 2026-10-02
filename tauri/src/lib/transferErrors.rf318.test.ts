import { afterEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { invokeTypedCommand } from '@/lib/typedIpc';
import { resolveBackendErrorMessage, getBackendErrorTranslationKey } from '@/lib/backendError';
import {
  BackendCommandError,
  readBackendError,
  normalizeTransferError,
  backendErrorLogDetails,
} from '@/lib/backendErrorWire';
import type { BackendErrorCode } from '@/lib/generated/ipcContracts';
import fixtures from '../../src-tauri/src/commands/export_import/contracts/rf318-fixtures.json';
import codes from '../../src-tauri/src/commands/export_import/contracts/rf318-codes.json';
vi.mock('./i18n', async () => {
  const { createInstance } = await import('i18next');
  const zh = (await import('../locales/zh-CN/common.json')).default;
  const en = (await import('../locales/en-US/common.json')).default;
  const instance = createInstance();
  await instance.init({
    lng: 'zh-CN',
    fallbackLng: 'en-US',
    defaultNS: 'common',
    resources: { 'zh-CN': { common: zh }, 'en-US': { common: en } },
    interpolation: { escapeValue: false },
  });
  return { default: instance };
});
import i18n from './i18n';
const PRIVATE = 'RF318_SYNTHETIC_PRIVATE_ID';
afterEach(() => vi.restoreAllMocks());
describe('RF318 transfer rejection and outcome separation', () => {
  it('the actual backup IPC error projection does not expose a private missing-backup identifier', async () => {
    vi.mocked(invoke).mockRejectedValueOnce("Backup '" + PRIVATE + "' not found");
    const error = await invokeTypedCommand('backup_restore', { backupId: PRIVATE }).catch(
      (error: unknown) => error,
    );
    expect(error).toBeInstanceOf(BackendCommandError);
    expect(resolveBackendErrorMessage(error)).not.toContain(PRIVATE);
    expect(readBackendError(error)?.code).toBe('BACKUP_NOT_FOUND');
  });
  it.each([
    ['__IMPORT_ERR__:BAD_PASSWORD:' + PRIVATE, 'IMPORT_BAD_PASSWORD'],
    ['__IMPORT_ERR__:FAILED:' + PRIVATE, 'IMPORT_FAILED'],
    ['__EXPORT_ERR__:MASTER_VERIFY_FAILED:' + PRIVATE, 'EXPORT_PASSWORD_CHECK_FAILED'],
    ['__EXPORT_ERR__:NO_OBJECTS_SELECTED:' + PRIVATE, 'EXPORT_SCOPE_EMPTY'],
    ['__EXPORT_ERR__:FAILED:' + PRIVATE, 'EXPORT_FAILED'],
    [PRIVATE, 'INTERNAL_ERROR'],
  ])('actual transfer IPC adapts legacy %s without persisting details', async (raw, code) => {
    vi.mocked(invoke).mockRejectedValueOnce(new Error(raw));
    const error = await invokeTypedCommand('import_operations_list', { accountId: 'a' }).catch(
      (e: unknown) => e,
    );
    expect(readBackendError(error)?.code).toBe(code);
    expect(JSON.stringify(error)).not.toContain(PRIVATE);
    expect(resolveBackendErrorMessage(error)).not.toContain(PRIVATE);
    expect(JSON.stringify(backendErrorLogDetails(error))).not.toContain(PRIVATE);
  });
  it('real Host serde fixtures retain only approved phase/count fields', () => {
    for (const f of Object.values(fixtures)) {
      expect(readBackendError(f)).toEqual(f);
      const decoded = readBackendError({
        ...f,
        message: PRIVATE,
        cause: PRIVATE,
        safeDetails: { ...f.safeDetails, path: PRIVATE, field: PRIVATE },
      })!;
      expect(decoded).toEqual(f);
      expect(new BackendCommandError(decoded).message).toBe(f.code);
    }
    for (const count of [-1, 1.1, Infinity, Number.MAX_SAFE_INTEGER + 1, '1', 0]) {
      expect(
        readBackendError({
          ...fixtures.restorePartial,
          safeDetails: { stage: 'write', completedCount: count },
        })?.safeDetails,
      ).toEqual({ stage: 'write' });
    }
    expect(
      readBackendError({ ...fixtures.password, safeDetails: { stage: 'read', completedCount: 1 } })
        ?.safeDetails,
    ).toEqual({ stage: 'read' });
    expect(
      readBackendError({
        ...fixtures.restorePartial,
        safeDetails: { stage: 'validate', completedCount: 1 },
      })?.safeDetails,
    ).toEqual({ stage: 'validate' });
    expect(
      readBackendError({ ...fixtures.password, safeDetails: { stage: 'unknown' } }),
    ).toBeNull();
    expect(
      readBackendError({ status: 'partial', objectCount: 1, errorCode: 'IMPORT_FAILED' }),
    ).toBeNull();
  });
  it('all transfer machine codes have actual independent bilingual translations', async () => {
    for (const language of ['zh-CN', 'en-US']) {
      await i18n.changeLanguage(language);
      for (const code of codes) {
        const key = getBackendErrorTranslationKey(code as BackendErrorCode);
        const message = resolveBackendErrorMessage(
          new BackendCommandError(normalizeTransferError(code)),
        );
        expect(i18n.exists(key)).toBe(true);
        expect(message).not.toMatch(/backend_|RF318_SYNTHETIC/);
        expect(message.length).toBeGreaterThan(2);
      }
      const noWrite = resolveBackendErrorMessage(
        new BackendCommandError(readBackendError(fixtures.restoreNone)!),
      );
      const partial = resolveBackendErrorMessage(
        new BackendCommandError(readBackendError(fixtures.restorePartial)!),
      );
      const uncertain = resolveBackendErrorMessage(
        new BackendCommandError(readBackendError(fixtures.unconfirmed)!),
      );
      expect(new Set([noWrite, partial, uncertain]).size).toBe(3);
    }
  });
});
