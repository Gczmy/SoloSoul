import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import i18n from '@/lib/i18n';
import { setRequestSession } from '@/lib/sessionRequests';
import type { DecryptedImportPreview, ImportPreview, ImportResult } from '@/types/exportImport';
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

describe('RF-918 ordinary import attachment selection', () => {
  const decrypted: DecryptedImportPreview = {
    objects: [
      {
        id: 'object-1',
        name: 'Fixture',
        typeId: 'note',
        sectionType: 'notes',
        sensitivityLevel: 'public',
        createdAt: '2026-09-28T00:00:00Z',
        updatedAt: '2026-09-28T00:00:00Z',
        tags: [],
      },
    ],
    conflicts: [],
    hasPreferences: false,
    hasAuditLog: false,
    attachments: [
      { id: 'attachment-1', objectId: 'object-1', fileName: 'one.txt', sizeBytes: 1 },
      { id: 'attachment-2', objectId: 'object-1', fileName: 'two.txt', sizeBytes: 2 },
    ],
  };

  it.each([
    { deselected: ['attachment-1', 'attachment-2'], expected: [] },
    { deselected: ['attachment-2'], expected: ['attachment-1'] },
  ])(
    'sends exactly the selected attachment IDs after deselecting $deselected',
    async ({ deselected, expected }) => {
      mocks.invoke.mockImplementation(async (command: string) => {
        if (command === 'import_decrypt_preview') return decrypted;
        if (command === 'import_execute_advanced') return complete;
        throw new Error('Unexpected IPC: ' + command);
      });
      const { result } = renderHook(() =>
        useImportState({
          accountId: 'account',
          onError: mocks.onError,
          onSuccess: mocks.onSuccess,
          t: i18n.t.bind(i18n),
          i18n,
          reloadScope: vi.fn(),
        }),
      );
      act(() => {
        result.current.onSetImportPath('C:/fixture.solosoul');
        result.current.setImportPw('export-password');
      });
      await act(async () => {
        await result.current.onDecrypt();
      });
      expect(result.current.importTotalSelected).toBe(1);
      expect(result.current.importSelectedAttachmentIds).toEqual(
        new Set(['attachment-1', 'attachment-2']),
      );
      act(() => {
        for (const id of deselected) result.current.onToggleImportAttachment(id);
      });
      expect(result.current.importSelectedAttachmentIds).toEqual(new Set(expected));
      await act(async () => {
        await result.current.onImport();
      });
      expect(mocks.invoke).toHaveBeenCalledWith(
        'import_execute_advanced',
        expect.objectContaining({
          accountId: 'account',
          req: expect.objectContaining({
            selections: [{ objectId: 'object-1', selected: true }],
            selectedAttachmentIds: expected,
          }),
        }),
      );
      expect(mocks.onSuccess).toHaveBeenCalledOnce();
      expect(mocks.onError).not.toHaveBeenCalled();
    },
  );
});

describe('RF-919 import preview source ownership', () => {
  const manifest = (filePath: string): ImportPreview => ({
    filePath,
    version: '1',
    objectCount: 1,
    hasAttachments: false,
    extraFiles: [],
    exportTime: null,
    passwordHint: null,
  });
  const decrypted = (id: string): DecryptedImportPreview => ({
    objects: [
      {
        id,
        name: id,
        typeId: 'note',
        sectionType: 'notes',
        sensitivityLevel: 'public',
        createdAt: '2026-09-28T00:00:00Z',
        updatedAt: '2026-09-28T00:00:00Z',
        tags: [],
      },
    ],
    conflicts: [],
    hasPreferences: false,
    hasAuditLog: false,
    attachments: [],
  });
  const pending = <T>() => {
    let resolve!: (value: T) => void;
    let reject!: (reason: Error) => void;
    const promise = new Promise<T>((yes, no) => {
      resolve = yes;
      reject = no;
    });
    return { promise, resolve, reject };
  };
  const renderImport = () =>
    renderHook(() =>
      useImportState({
        accountId: 'account',
        onError: mocks.onError,
        onSuccess: mocks.onSuccess,
        t: i18n.t.bind(i18n),
        i18n,
        reloadScope: vi.fn(),
      }),
    );

  it('keeps the new package manifest when an earlier parse finishes late', async () => {
    const oldParse = pending<ImportPreview>();
    mocks.invoke.mockImplementation((command: string, args: { filePath: string }) => {
      if (command !== 'import_parse_package') throw new Error('Unexpected IPC: ' + command);
      return args.filePath === 'C:/old.solosoul'
        ? oldParse.promise
        : Promise.resolve(manifest('C:/new.solosoul'));
    });
    const { result } = renderImport();
    act(() => result.current.onSetImportPath('C:/old.solosoul'));
    let oldRequest!: Promise<void>;
    act(() => {
      oldRequest = result.current.onPreview();
    });
    await waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith('import_parse_package', {
        filePath: 'C:/old.solosoul',
      }),
    );
    act(() => result.current.onSetImportPath('C:/new.solosoul'));
    await act(async () => {
      await result.current.onPreview();
    });
    expect(result.current.importPreview).toEqual(manifest('C:/new.solosoul'));
    await act(async () => {
      oldParse.resolve(manifest('C:/old.solosoul'));
      await oldRequest;
    });
    expect(result.current.importPreview).toEqual(manifest('C:/new.solosoul'));
    expect(mocks.onError).not.toHaveBeenCalled();
  });

  it('keeps the new decrypted tree and selections when the old decrypt finishes late', async () => {
    const oldDecrypt = pending<DecryptedImportPreview>();
    mocks.invoke.mockImplementation((command: string, args: { filePath: string }) => {
      if (command !== 'import_decrypt_preview') throw new Error('Unexpected IPC: ' + command);
      return args.filePath === 'C:/old.solosoul'
        ? oldDecrypt.promise
        : Promise.resolve(decrypted('new-object'));
    });
    const { result } = renderImport();
    act(() => {
      result.current.onSetImportPath('C:/old.solosoul');
      result.current.setImportPw('old-password');
    });
    let oldRequest!: Promise<void>;
    act(() => {
      oldRequest = result.current.onDecrypt();
    });
    await waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith('import_decrypt_preview', {
        filePath: 'C:/old.solosoul',
        password: 'old-password',
      }),
    );
    act(() => {
      result.current.onSetImportPath('C:/new.solosoul');
      result.current.setImportPw('new-password');
    });
    await act(async () => {
      await result.current.onDecrypt();
    });
    expect(result.current.decryptedPreview).toEqual(decrypted('new-object'));
    await act(async () => {
      oldDecrypt.resolve(decrypted('old-object'));
      await oldRequest;
    });
    expect(result.current.decryptedPreview).toEqual(decrypted('new-object'));
    expect(result.current.importSelections).toEqual(new Map([['new-object', true]]));
    expect(mocks.onError).not.toHaveBeenCalled();
  });

  it('does not reuse a staged Android URI after its source is replaced', async () => {
    const oldStage = pending<string>();
    mocks.stage.mockImplementation((uri: string) =>
      uri === 'content://old' ? oldStage.promise : Promise.resolve('cached-new.solosoul'),
    );
    mocks.invoke.mockImplementation((command: string, args: { filePath: string }) => {
      if (command !== 'import_parse_package') throw new Error('Unexpected IPC: ' + command);
      return Promise.resolve(manifest(args.filePath));
    });
    const { result } = renderImport();
    act(() => result.current.onSetImportPath('content://old'));
    let oldRequest!: Promise<void>;
    act(() => {
      oldRequest = result.current.onPreview();
    });
    await waitFor(() => expect(mocks.stage).toHaveBeenCalledWith('content://old'));
    act(() => result.current.onSetImportPath('content://new'));
    await act(async () => {
      await result.current.onPreview();
    });
    expect(result.current.importPreview).toEqual(manifest('cached-new.solosoul'));
    await act(async () => {
      oldStage.resolve('cached-old.solosoul');
      await oldRequest;
    });
    expect(mocks.cleanup).toHaveBeenCalledWith('cached-old.solosoul');
    expect(mocks.invoke).not.toHaveBeenCalledWith('import_parse_package', {
      filePath: 'cached-old.solosoul',
    });
    expect(result.current.importPreview).toEqual(manifest('cached-new.solosoul'));
  });

  it('does not replace the source while its import is still executing', async () => {
    const oldImport = pending<ImportResult>();
    mocks.invoke.mockImplementation((command: string) => {
      if (command === 'import_execute_advanced') return oldImport.promise;
      throw new Error('Unexpected IPC: ' + command);
    });
    const { result } = renderImport();
    act(() => {
      result.current.onSetImportPath('C:/old.solosoul');
      result.current.setImportPw('export-password');
      result.current.onToggleSelection('object-1');
    });
    let importRequest!: Promise<void>;
    act(() => {
      importRequest = result.current.onImport();
    });
    await waitFor(() => expect(result.current.isImporting).toBe(true));
    act(() => result.current.onSetImportPath('C:/new.solosoul'));
    expect(result.current.importPath).toBe('C:/old.solosoul');
    await act(async () => {
      oldImport.resolve(complete);
      await importRequest;
    });
    expect(result.current.isImporting).toBe(false);
    act(() => result.current.onSetImportPath('C:/new.solosoul'));
    expect(result.current.importPath).toBe('C:/new.solosoul');
  });

  it('does not surface an old parse error after the import page unmounts', async () => {
    const oldParse = pending<ImportPreview>();
    mocks.invoke.mockImplementation((command: string) => {
      if (command === 'import_parse_package') return oldParse.promise;
      throw new Error('Unexpected IPC: ' + command);
    });
    const { result, unmount } = renderImport();
    act(() => result.current.onSetImportPath('C:/old.solosoul'));
    let oldRequest!: Promise<void>;
    act(() => {
      oldRequest = result.current.onPreview();
    });
    await waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith('import_parse_package', {
        filePath: 'C:/old.solosoul',
      }),
    );
    unmount();
    await act(async () => {
      oldParse.reject(new Error('old package failed'));
      await oldRequest;
    });
    expect(mocks.onError).not.toHaveBeenCalled();
  });
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
