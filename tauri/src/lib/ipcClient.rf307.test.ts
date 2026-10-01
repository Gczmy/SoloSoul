import { describe, it, expect, vi, beforeEach } from 'vitest';
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
import { invoke } from '@tauri-apps/api/core';
import { invokeCommand } from './ipcClient';
import { BackendCommandError } from './backendErrorWire';
import fixtures from '../../src-tauri/src/commands/object/tests/rf307-fixtures.json';
const SECRET = 'RF307-SYNTHETIC-SECRET-FIELD-key-path';

describe('RF307 actual native transport rejection', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
  });
  it('keeps machine details and logs only safe projection before throwing Error', async () => {
    vi.stubEnv('DEV', true);
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {});
    try {
      vi.mocked(invoke).mockRejectedValue({
        ...fixtures.longName,
        cause: SECRET,
        safeDetails: { ...fixtures.longName.safeDetails, field: SECRET },
      });
      const error = await invokeCommand('object_create', { input: {} }).catch((e) => e);
      expect(error).toBeInstanceOf(BackendCommandError);
      if (!(error instanceof BackendCommandError)) throw error;
      expect(error.backend).toEqual(fixtures.longName);
      expect(JSON.stringify(warn.mock.calls)).not.toContain(SECRET);
      expect(JSON.stringify(error)).not.toContain(SECRET);
      expect(warn).toHaveBeenCalledWith("[ipc] command 'object_create' failed:", fixtures.longName);
    } finally {
      warn.mockRestore();
      vi.unstubAllEnvs();
    }
  });
  it('legacy object and unknown rejections are sanitized, while unmigrated error identity is retained', async () => {
    vi.mocked(invoke).mockRejectedValue(`Object with ID '${SECRET}' already exists`);
    await expect(invokeCommand('object_create', {})).rejects.toMatchObject({
      backend: { code: 'OBJECT_ID_EXISTS', safeDetails: null },
    });
    vi.mocked(invoke).mockRejectedValue(new Error(SECRET));
    await expect(invokeCommand('snapshot_rollback', {})).rejects.toMatchObject({
      backend: { code: 'INTERNAL_ERROR', retryable: false },
    });
    const legacy = new Error('Invalid password');
    vi.mocked(invoke).mockRejectedValue(legacy);
    await expect(invokeCommand('login')).rejects.toBe(legacy);
  });
  it('success null and successful object results pass through unchanged', async () => {
    vi.mocked(invoke).mockResolvedValue(null);
    await expect(invokeCommand('snapshot_rollback', {})).resolves.toBeNull();
    const object = { id: 'synthetic', properties: { value: SECRET } };
    vi.mocked(invoke).mockResolvedValue(object);
    await expect(invokeCommand('object_get', {})).resolves.toBe(object);
  });
});
