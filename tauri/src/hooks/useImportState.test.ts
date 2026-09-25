import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import i18n from '@/lib/i18n';
import type { ImportResult } from '@/types/exportImport';
import { useImportState } from './useImportState';
import { useCloudSyncPage } from '@/pages/settings/cloudSync/useCloudSyncPage';

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  cleanup: vi.fn(),
  stage: vi.fn(),
  onError: vi.fn(),
  onSuccess: vi.fn(),
}));
vi.mock('@/lib/ipcClient', () => ({ invokeCommand: mocks.invoke }));
vi.mock('react-i18next', async (importOriginal) => {
  const actual = await importOriginal<typeof import('react-i18next')>();
  const { default: engine } = await import('i18next');
  const t = engine.t.bind(engine);
  return { ...actual, useTranslation: () => ({ t, i18n: engine }) };
});
vi.mock('@/lib/mobileFileTransfer', () => ({
  cleanupStagedFile: mocks.cleanup,
  stageImportPackage: mocks.stage,
  isUriPath: (path: string) => path.startsWith('content://'),
}));
vi.mock('@/hooks/useToastError', () => ({
  useToastError: () => ({ onError: mocks.onError, onSuccess: mocks.onSuccess }),
}));
vi.mock('@/stores/authStore', () => ({
  useAuthStore: (selector: (s: unknown) => unknown) =>
    selector({ currentAccount: { id: 'account' } }),
}));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn().mockResolvedValue(() => {}) }));

const complete: ImportResult = {
  status: 'complete',
  objectCount: 2,
  attachmentCount: 1,
  templateCount: 0,
  snapshotCount: 2,
  preferencesImported: false,
  attachmentFilesWritten: 1,
  failureStage: null,
  errorCode: null,
};
const partial: ImportResult = {
  ...complete,
  status: 'partial',
  failureStage: 'attachments',
  errorCode: 'IMPORT_FAILED',
};
const uncommitted: ImportResult = {
  ...complete,
  status: 'notCommitted',
  objectCount: 0,
  attachmentCount: 0,
  snapshotCount: 0,
  attachmentFilesWritten: 0,
  failureStage: 'preparation',
  errorCode: 'IMPORT_FAILED',
};

beforeEach(() => {
  vi.clearAllMocks();
  mocks.stage.mockResolvedValue('cached.solosoul');
});

describe('RF-020 ordinary import outcomes', () => {
  it.each([partial, uncommitted, complete])(
    'handles $status without discarding an incomplete source',
    async (outcome) => {
      mocks.invoke.mockResolvedValue(outcome);
      const reload = vi.fn();
      const { result } = renderHook(() =>
        useImportState({
          accountId: 'account',
          onError: mocks.onError,
          onSuccess: mocks.onSuccess,
          t: i18n.t.bind(i18n),
          i18n,
          reloadScope: reload,
        }),
      );
      act(() => {
        result.current.onSetImportPath('content://fixture');
        result.current.setImportPw('export-password');
        result.current.onToggleSelection('object');
      });
      await act(async () => {
        await result.current.onImport();
      });
      expect(result.current.isImporting).toBe(false);
      if (outcome.status === 'complete') {
        expect(mocks.onSuccess).toHaveBeenCalledOnce();
        expect(mocks.cleanup).toHaveBeenCalledWith('cached.solosoul');
        expect(result.current.importPath).toBe('');
      } else {
        expect(mocks.onSuccess).not.toHaveBeenCalled();
        expect(mocks.onError).toHaveBeenCalledOnce();
        expect(mocks.cleanup).not.toHaveBeenCalled();
        expect(result.current.importPath).toBe('content://fixture');
        expect(mocks.onError.mock.calls[0][0].message).not.toContain('settings:');
      }
      expect(reload).toHaveBeenCalledTimes(outcome.status === 'notCommitted' ? 0 : 1);
    },
  );
});

describe('RF-020 cloud incoming outcomes', () => {
  it.each([partial, uncommitted, complete])(
    'advances the waterline only for complete imports ($status)',
    async (outcome) => {
      const source = 'D:\\cache\\remote-device\\123.solosoul';
      mocks.invoke.mockImplementation(async (cmd: string) => {
        if (cmd === 'cloud_sync_list_incoming') return [source];
        if (cmd === 'cloud_sync_get_config') return null;
        if (cmd === 'import_execute_advanced') return outcome;
        return null;
      });
      const { result } = renderHook(() => useCloudSyncPage());
      await waitFor(() => expect(result.current.incomingFiles).toEqual([source]));
      act(() => result.current.setConfigJson({ password: 'export-password' }));
      await act(async () => {
        await result.current.handleImportIncoming(source);
      });
      expect(mocks.invoke).toHaveBeenCalledWith(
        'import_execute_advanced',
        expect.objectContaining({
          req: expect.objectContaining({
            selections: null,
            objectStrategies: {},
            sourcePath: source,
          }),
        }),
      );
      if (outcome.status === 'complete') {
        expect(mocks.invoke).toHaveBeenCalledWith('cloud_sync_mark_applied', {
          deviceId: 'remote-device',
          hlc: '123',
        });
        expect(result.current.incomingFiles).toEqual([]);
        expect(mocks.onSuccess).toHaveBeenCalledOnce();
      } else {
        expect(mocks.invoke.mock.calls.some(([cmd]) => cmd === 'cloud_sync_mark_applied')).toBe(
          false,
        );
        expect(result.current.incomingFiles).toEqual([source]);
        expect(mocks.onSuccess).not.toHaveBeenCalled();
        expect(mocks.onError).toHaveBeenCalledOnce();
      }
    },
  );
});
