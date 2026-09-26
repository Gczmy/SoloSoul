import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import i18n from '@/lib/i18n';
import { setRequestSession } from '@/lib/sessionRequests';
import type { ImportResult } from '@/types/exportImport';
import { useImportState } from './useImportState';
import { useCloudSyncPage } from '@/pages/settings/cloudSync/useCloudSyncPage';

type IncomingEvent = {
  payload: { accountId: string; sessionGeneration: number; files?: string[] };
};

const mocks = vi.hoisted(() => ({
  accountId: 'account',
  listen: vi.fn(),
  incomingListeners: [] as ((event: IncomingEvent) => void)[],
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
    selector({ currentAccount: { id: mocks.accountId } }),
}));
vi.mock('@tauri-apps/api/event', () => ({ listen: mocks.listen }));

const complete: ImportResult = {
  sessionGeneration: 7,
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
  setRequestSession(null);
  mocks.accountId = 'account';
  setRequestSession(mocks.accountId);
  mocks.incomingListeners.length = 0;
  mocks.listen.mockImplementation((_name: string, callback: (event: IncomingEvent) => void) => {
    mocks.incomingListeners.push(callback);
    return Promise.resolve(vi.fn());
  });
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
        expect.objectContaining({ requestIsCurrent: expect.any(Function) }),
      );
      if (outcome.status === 'complete') {
        expect(mocks.invoke).toHaveBeenCalledWith(
          'cloud_sync_mark_applied',
          { accountId: 'account', sessionGeneration: 7, sourcePath: source },
          expect.objectContaining({ requestIsCurrent: expect.any(Function) }),
        );
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

describe('RF-003 cloud incoming session isolation', () => {
  it('ignores a completed import from the previous account after switching accounts', async () => {
    const sourceA = 'C:/cache/account/remote-device/123.solosoul';
    const sourceB = 'C:/cache/account-b/remote-device/456.solosoul';
    let resolveImport!: (result: ImportResult) => void;
    const deferredImport = new Promise<ImportResult>((resolve) => {
      resolveImport = resolve;
    });
    mocks.invoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'cloud_sync_list_incoming') {
        return [mocks.accountId === 'account' ? sourceA : sourceB];
      }
      if (cmd === 'import_execute_advanced') return deferredImport;
      return null;
    });
    const { result, rerender } = renderHook(() => useCloudSyncPage());
    await waitFor(() => expect(result.current.incomingFiles).toEqual([sourceA]));
    act(() => result.current.setConfigJson({ password: 'account-a-password' }));
    let pendingImport!: Promise<void>;
    act(() => {
      pendingImport = result.current.handleImportIncoming(sourceA);
    });
    expect(result.current.importingFile).toBe(sourceA);

    act(() => {
      setRequestSession('account-b');
      mocks.accountId = 'account-b';
      rerender();
    });
    await waitFor(() => expect(result.current.incomingFiles).toEqual([sourceB]));
    const configB = { username: 'account-b', password: 'account-b-password' };
    act(() => {
      result.current.setConfigJson(configB);
      result.current.setIntervalSecs(7200);
      result.current.setShowPasswordDialog(true);
    });

    await act(async () => {
      resolveImport(complete);
      await pendingImport;
    });

    expect(result.current.configJson).toEqual(configB);
    expect(result.current.intervalSecs).toBe(7200);
    expect(result.current.showPasswordDialog).toBe(true);
    expect(result.current.incomingFiles).toEqual([sourceB]);
    expect(result.current.importingFile).toBeNull();
    expect(mocks.invoke.mock.calls.some(([cmd]) => cmd === 'cloud_sync_mark_applied')).toBe(false);
    expect(mocks.onSuccess).not.toHaveBeenCalled();
    expect(mocks.onError).not.toHaveBeenCalled();
  });

  it('ignores stale listeners and foreign events and refreshes current files from the backend', async () => {
    const sourceA = 'C:/cache/account/remote-device/123.solosoul';
    const sourceB = 'C:/cache/account-b/remote-device/456.solosoul';
    const newSourceB = 'C:/cache/account-b/remote-device/789.solosoul';
    let resolveRefresh!: (files: string[]) => void;
    const deferredRefresh = new Promise<string[]>((resolve) => {
      resolveRefresh = resolve;
    });
    let refreshPending = false;
    mocks.invoke.mockImplementation(async (cmd: string) => {
      if (cmd !== 'cloud_sync_list_incoming') return null;
      if (refreshPending) return deferredRefresh;
      return [mocks.accountId === 'account' ? sourceA : sourceB];
    });
    const { result, rerender } = renderHook(() => useCloudSyncPage());
    await waitFor(() => expect(result.current.incomingFiles).toEqual([sourceA]));
    expect(mocks.incomingListeners).toHaveLength(1);
    const oldListener = mocks.incomingListeners[0];

    act(() => {
      setRequestSession('account-b');
      mocks.accountId = 'account-b';
      rerender();
    });
    await waitFor(() => expect(result.current.incomingFiles).toEqual([sourceB]));
    expect(mocks.incomingListeners).toHaveLength(2);
    const currentListener = mocks.incomingListeners[1];
    const incomingCallCount = () =>
      mocks.invoke.mock.calls.filter(([cmd]) => cmd === 'cloud_sync_list_incoming').length;
    const callsBeforeEvents = incomingCallCount();

    act(() => {
      const oldEvent = {
        payload: { accountId: 'account', sessionGeneration: 7, files: [sourceA] },
      };
      oldListener(oldEvent);
      currentListener(oldEvent);
    });
    expect(incomingCallCount()).toBe(callsBeforeEvents);
    expect(result.current.incomingFiles).toEqual([sourceB]);

    refreshPending = true;
    act(() => {
      currentListener({
        payload: { accountId: 'account-b', sessionGeneration: 8, files: [sourceA] },
      });
    });
    expect(incomingCallCount()).toBe(callsBeforeEvents + 1);
    expect(result.current.incomingFiles).toEqual([sourceB]);
    await act(async () => {
      resolveRefresh([sourceB, newSourceB]);
      await deferredRefresh;
    });
    expect(result.current.incomingFiles).toEqual([sourceB, newSourceB]);
    expect(mocks.onSuccess).not.toHaveBeenCalled();
    expect(mocks.onError).not.toHaveBeenCalled();
  });
});
