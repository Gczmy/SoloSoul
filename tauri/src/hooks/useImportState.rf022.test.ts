import { act, cleanup, renderHook, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import i18n from '@/lib/i18n';
import { setRequestSession } from '@/lib/sessionRequests';
import type {
  AdvancedImportRequest,
  DecryptedImportPreview,
  ImportOperationSummary,
  ImportResult,
} from '@/types/exportImport';
import { useImportState } from './useImportState';
import { BackendCommandError, makeBackendError } from '@/lib/backendErrorWire';

const io = vi.hoisted(() => ({
  invoke: vi.fn(),
  stage: vi.fn(),
  cleanup: vi.fn(),
  choose: vi.fn(),
}));
vi.mock('@/lib/ipcClient', () => ({ invokeCommand: io.invoke }));
vi.mock('@/lib/mobileFileTransfer', () => ({
  cleanupStagedFile: io.cleanup,
  stageImportPackage: io.stage,
  isUriPath: (path: string) => path.startsWith('content://'),
}));
vi.mock('@/lib/dialog', () => ({ openWithPause: io.choose }));
const ID = '11111111-1111-4111-8111-111111111111';
const NEXT_ID = '22222222-2222-4222-8222-222222222222';
const complete: ImportResult = {
  operationId: ID,
  sessionGeneration: 7,
  status: 'complete',
  objectCount: 2,
  attachmentCount: 2,
  templateCount: 0,
  snapshotCount: 2,
  preferencesImported: false,
  attachmentFilesWritten: 2,
  failureStage: null,
  errorCode: null,
};
const partial: ImportResult = {
  ...complete,
  status: 'partial',
  attachmentCount: 0,
  attachmentFilesWritten: 0,
  failureStage: 'attachments',
  errorCode: 'IMPORT_FAILED',
};
const tree: DecryptedImportPreview = {
  objects: ['object-a', 'object-b'].map((id) => ({
    id,
    name: id,
    typeId: 'note',
    sectionType: 'notes',
    sensitivityLevel: 'public',
    createdAt: '',
    updatedAt: '',
    tags: [],
  })),
  conflicts: [
    { objectId: 'object-a', importedName: 'object-a', existingName: 'local', kind: 'renamedLocal' },
  ],
  attachments: [{ id: 'attachment-a', objectId: 'object-a', fileName: 'a.txt', sizeBytes: 1 }],
  hasPreferences: false,
  hasAuditLog: false,
};
function summary(overrides: Partial<ImportOperationSummary> = {}): ImportOperationSummary {
  return {
    operationId: ID,
    phase: 'attachments',
    sourceKind: 'manual',
    sourceName: 'original.solosoul',
    createdAt: '2026-09-30',
    updatedAt: '2026-09-30',
    sourceRequired: false,
    passwordRequired: false,
    outcome: partial,
    ...overrides,
  };
}
function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}
function mount() {
  const onError = vi.fn();
  const onSuccess = vi.fn();
  const reload = vi.fn();
  const hook = renderHook(
    ({ accountId }) =>
      useImportState({
        accountId,
        onError,
        onSuccess,
        reloadScope: reload,
        t: i18n.t.bind(i18n),
        i18n,
      }),
    { initialProps: { accountId: 'account-a' } },
  );
  return { ...hook, onError, onSuccess, reload };
}
async function prepare(f: ReturnType<typeof mount>, source = 'C:/original.solosoul') {
  act(() => {
    f.result.current.onSetImportPath(source);
    f.result.current.setImportPw('package-password');
  });
  await act(async () => {
    await f.result.current.onDecrypt();
  });
}
const executeCalls = () =>
  io.invoke.mock.calls.filter(([name]) => name === 'import_execute_advanced');
const resumeCalls = () =>
  io.invoke.mock.calls.filter(([name]) => name === 'import_operation_resume');
beforeEach(async () => {
  io.invoke.mockReset();
  io.stage.mockReset();
  io.cleanup.mockReset();
  io.choose.mockReset();
  io.stage.mockResolvedValue('C:/cache/original.solosoul');
  io.choose.mockResolvedValue(null);
  setRequestSession(null);
  setRequestSession('account-a');
  await i18n.changeLanguage('en-US');
  vi.spyOn(crypto, 'randomUUID').mockReturnValueOnce(ID).mockReturnValue(NEXT_ID);
  io.invoke.mockImplementation(async (command: string) => {
    if (command === 'import_operations_list') return [];
    if (command === 'import_decrypt_preview') return tree;
    if (command === 'import_operation_get') return summary();
    if (command === 'import_execute_advanced' || command === 'import_operation_resume')
      return complete;
    throw new Error('Unexpected IPC: ' + command);
  });
});
afterEach(() => {
  cleanup();
  setRequestSession(null);
  vi.restoreAllMocks();
});

describe('RF022 original import operation', () => {
  it('assigns one UUID and freezes options before URI staging; same-tick double clicks dispatch once', async () => {
    const staged = deferred<string>();
    const f = mount();
    await prepare(f);
    act(() => {
      f.result.current.onSetImportPath('content://original');
      f.result.current.setImportPw('package-password');
      f.result.current.onToggleSelection('object-a');
      f.result.current.setShowStrategySelector(true);
      f.result.current.setImportStrategy('keepBoth');
      f.result.current.onToggleImportAttachment('attachment-a');
    });
    io.stage.mockReturnValue(staged.promise);
    let first!: Promise<void>;
    let second!: Promise<void>;
    act(() => {
      first = f.result.current.onImport();
      second = f.result.current.onImport();
      f.result.current.onToggleSelection('object-a');
      f.result.current.onToggleImportAttachment('attachment-a');
      f.result.current.setImportStrategy('overwrite');
      f.result.current.setShowStrategySelector(false);
      f.result.current.setImportPw('changed-password');
    });
    expect(crypto.randomUUID).toHaveBeenCalledTimes(1);
    expect(f.result.current.importOperations.currentId).toBe(ID);
    expect(executeCalls()).toHaveLength(0);
    await act(async () => {
      await i18n.changeLanguage('zh-CN');
      staged.resolve('C:/cache/frozen.solosoul');
      await Promise.all([first, second]);
    });
    expect(executeCalls()).toHaveLength(1);
    expect(executeCalls()[0][1].req).toEqual({
      operationId: ID,
      selections: [{ objectId: 'object-a', selected: true }],
      strategy: 'keepBoth',
      sourcePath: 'C:/cache/frozen.solosoul',
      password: 'package-password',
      selectedAttachmentIds: ['attachment-a'],
      objectStrategies: {},
      locale: 'en-US',
    });
  });
  it('unknown reply looks up the same ID and replays only the original frozen request when not registered', async () => {
    let writes = 0;
    io.invoke.mockImplementation(async (command: string) => {
      if (command === 'import_operations_list') return [];
      if (command === 'import_decrypt_preview') return tree;
      if (command === 'import_operation_get') throw '__IMPORT_ERR__:OPERATION_NOT_FOUND';
      if (command === 'import_execute_advanced') {
        if (++writes === 1) throw new Error('transport unknown');
        return complete;
      }
      throw new Error(command);
    });
    const f = mount();
    await prepare(f);
    act(() => {
      f.result.current.setShowStrategySelector(true);
      f.result.current.setImportStrategy('keepBoth');
      f.result.current.onSetObjectConflictStrategy('object-a', 'overwrite');
    });
    await act(async () => {
      await f.result.current.onImport();
    });
    expect(f.result.current.importOperations.currentId).toBe(ID);
    await act(async () => {
      await i18n.changeLanguage('zh-CN');
      await f.result.current.importOperations.onRetry();
    });
    expect(crypto.randomUUID).toHaveBeenCalledTimes(1);
    expect(executeCalls()).toHaveLength(2);
    expect(executeCalls()[1][1].req).toEqual(executeCalls()[0][1].req);
    expect(executeCalls()[0][1].req.objectStrategies).toEqual({ 'object-a': 'overwrite' });
    expect(
      io.invoke.mock.calls.some(
        ([command, args]) => command === 'import_operation_get' && args.operationId === ID,
      ),
    ).toBe(true);
    expect(f.onSuccess).toHaveBeenCalledOnce();
  });
  it('unknown reply with a registered operation enters Resume without another Fresh execute', async () => {
    io.invoke.mockImplementation(async (command: string) => {
      if (command === 'import_operations_list') return [];
      if (command === 'import_decrypt_preview') return tree;
      if (command === 'import_execute_advanced') throw new Error('reply lost');
      if (command === 'import_operation_get') return summary();
      if (command === 'import_operation_resume') return complete;
      throw new Error(command);
    });
    const f = mount();
    await prepare(f);
    await act(async () => {
      await f.result.current.onImport();
    });
    await act(async () => {
      await f.result.current.importOperations.onRetry();
    });
    expect(f.result.current.importOperations.selected?.operationId).toBe(ID);
    await act(async () => {
      await f.result.current.importOperations.onResume();
    });
    expect(executeCalls()).toHaveLength(1);
    expect(resumeCalls()).toHaveLength(1);
    expect(resumeCalls()[0][1]).toEqual({
      accountId: 'account-a',
      operationId: ID,
      password: null,
      sourcePath: null,
    });
    expect(crypto.randomUUID).toHaveBeenCalledTimes(1);
  });
  it('Partial goes through get and asks for the new required password without reusing Fresh selections', async () => {
    io.invoke.mockImplementation(async (command: string) => {
      if (command === 'import_operations_list') return [];
      if (command === 'import_decrypt_preview') return tree;
      if (command === 'import_execute_advanced') return partial;
      if (command === 'import_operation_get') return summary({ passwordRequired: true });
      if (command === 'import_operation_resume') return complete;
      throw new Error(command);
    });
    const f = mount();
    await prepare(f);
    await act(async () => {
      await f.result.current.onImport();
    });
    expect(f.result.current.importOperations.password).toBe('');
    expect(f.result.current.importOperations.canResume).toBe(false);
    await act(async () => {
      await f.result.current.importOperations.onResume();
    });
    expect(resumeCalls()).toHaveLength(0);
    act(() => f.result.current.importOperations.onSetPassword('new-package-password'));
    await act(async () => {
      await f.result.current.importOperations.onResume();
    });
    expect(resumeCalls()[0][1]).toEqual({
      accountId: 'account-a',
      operationId: ID,
      password: 'new-package-password',
      sourcePath: null,
    });
    expect(executeCalls()).toHaveLength(1);
    expect(f.onSuccess).toHaveBeenCalledOnce();
  });
  it('ready pending operation resumes with no preview, password, or selected objects; double action invokes once', async () => {
    const pending = deferred<ImportResult>();
    io.invoke.mockImplementation(async (command: string) => {
      if (command === 'import_operations_list') return [summary()];
      if (command === 'import_operation_get') return summary();
      if (command === 'import_operation_resume') return pending.promise;
      throw new Error(command);
    });
    const f = mount();
    await waitFor(() => expect(f.result.current.importOperations.items).toHaveLength(1));
    await act(async () => {
      await f.result.current.importOperations.onSelect(ID);
    });
    expect(f.result.current.importPreview).toBeNull();
    expect(f.result.current.importTotalSelected).toBe(0);
    expect(f.result.current.importOperations.canResume).toBe(true);
    let first!: Promise<void>;
    let second!: Promise<void>;
    act(() => {
      first = f.result.current.importOperations.onResume();
      second = f.result.current.importOperations.onResume();
    });
    await waitFor(() => expect(resumeCalls()).toHaveLength(1));
    await act(async () => {
      pending.resolve(complete);
      await Promise.all([first, second]);
    });
    expect(crypto.randomUUID).not.toHaveBeenCalled();
    expect(executeCalls()).toHaveLength(0);
  });
  it('resumed Partial exposes absolute operation counts without adding the previous attempt', async () => {
    io.invoke.mockImplementation(async (command: string) => {
      if (command === 'import_operations_list') return [summary()];
      if (command === 'import_operation_get') return summary();
      if (command === 'import_operation_resume') return partial;
      throw new Error(command);
    });
    const f = mount();
    await act(async () => {
      await f.result.current.importOperations.onSelect(ID);
    });
    await act(async () => {
      await f.result.current.importOperations.onResume();
    });
    await act(async () => {
      await f.result.current.importOperations.onResume();
    });
    expect(f.result.current.importOperations.selected?.outcome.objectCount).toBe(2);
    expect(f.result.current.importOperations.selected?.outcome.snapshotCount).toBe(2);
    expect(resumeCalls().map(([, args]) => args.operationId)).toEqual([ID, ID]);
    expect(f.onSuccess).not.toHaveBeenCalled();
  });
  it.each(['source', 'selection', 'strategy', 'newFresh'] as const)(
    'an explicit %s change creates a new Fresh ID after an unknown reply',
    async (change) => {
      io.invoke.mockImplementation(async (command: string) => {
        if (command === 'import_operations_list') return [];
        if (command === 'import_decrypt_preview') return tree;
        if (command === 'import_execute_advanced') throw new Error('unknown');
        throw new Error(command);
      });
      const f = mount();
      await prepare(f);
      await act(async () => {
        await f.result.current.onImport();
      });
      act(() => {
        if (change === 'source') {
          f.result.current.onSetImportPath('C:/other.solosoul');
          f.result.current.setImportPw('package-password');
          f.result.current.onToggleSelection('other');
        }
        if (change === 'selection') f.result.current.onToggleSelection('object-a');
        if (change === 'strategy') f.result.current.setImportStrategy('keepBoth');
        if (change === 'newFresh') f.result.current.importOperations.onNewImport();
      });
      if (change === 'newFresh') await prepare(f);
      await act(async () => {
        await f.result.current.onImport();
      });
      expect(executeCalls().map(([, args]) => args.req.operationId)).toEqual([ID, NEXT_ID]);
    },
  );
  it('a missing task or an operation conflict from Resume never falls back to Fresh', async () => {
    const f = mount();
    await act(async () => {
      await f.result.current.importOperations.onSelect(ID);
    });
    io.invoke.mockImplementation(async (command: string) => {
      if (command === 'import_operations_list') return [];
      if (command === 'import_operation_resume') throw '__IMPORT_ERR__:OPERATION_CONFLICT';
      throw new Error(command);
    });
    await act(async () => {
      await f.result.current.importOperations.onResume();
    });
    expect(f.result.current.importOperations.selected?.operationId).toBe(ID);
    expect(executeCalls()).toHaveLength(0);
    expect(crypto.randomUUID).not.toHaveBeenCalled();
  });
  it('canceling original-package selection retains ID and frozen job; reselecting a source does not allocate a new ID', async () => {
    io.invoke.mockImplementation(async (command: string) => {
      if (command === 'import_operations_list') return [];
      if (command === 'import_operation_get') return summary({ sourceRequired: true });
      if (command === 'import_operation_resume') return partial;
      throw new Error(command);
    });
    const f = mount();
    await act(async () => {
      await f.result.current.importOperations.onSelect(ID);
    });
    await act(async () => {
      await f.result.current.importOperations.onPickSource();
    });
    expect(f.result.current.importOperations.currentId).toBe(ID);
    expect(f.result.current.importOperations.replacementSource).toBe('');
    expect(resumeCalls()).toHaveLength(0);
    io.choose.mockResolvedValue('content://original');
    await act(async () => {
      await f.result.current.importOperations.onPickSource();
    });
    await act(async () => {
      await f.result.current.importOperations.onResume();
    });
    expect(resumeCalls()[0][1]).toEqual({
      accountId: 'account-a',
      operationId: ID,
      password: null,
      sourcePath: 'C:/cache/original.solosoul',
    });
    expect(crypto.randomUUID).not.toHaveBeenCalled();
  });
  it('closing the running view keeps the native job and its leased URI source and cannot let old finally clear a new run', async () => {
    const old = deferred<ImportResult>();
    const next = deferred<ImportResult>();
    io.invoke.mockImplementation(
      async (command: string, args?: { req?: AdvancedImportRequest }) => {
        if (command === 'import_operations_list') return [];
        if (command === 'import_decrypt_preview') return tree;
        if (command === 'import_execute_advanced')
          return args?.req?.operationId === ID ? old.promise : next.promise;
        throw new Error(command);
      },
    );
    const f = mount();
    await prepare(f, 'content://original');
    let original!: Promise<void>;
    act(() => {
      original = f.result.current.onImport();
    });
    await waitFor(() => expect(executeCalls()).toHaveLength(1));
    act(() => f.result.current.importOperations.onContinueLater());
    expect(f.result.current.importPw).toBe('');
    expect(io.cleanup).not.toHaveBeenCalledWith('C:/cache/original.solosoul');
    await prepare(f, 'C:/next.solosoul');
    let newer!: Promise<void>;
    act(() => {
      newer = f.result.current.onImport();
    });
    await waitFor(() => expect(executeCalls()).toHaveLength(2));
    await act(async () => {
      old.resolve(complete);
      await original;
    });
    expect(f.result.current.isImporting).toBe(true);
    expect(f.result.current.importOperations.currentId).toBe(NEXT_ID);
    expect(f.onSuccess).not.toHaveBeenCalled();
    expect(f.reload).not.toHaveBeenCalled();
    expect(io.invoke.mock.calls.some(([command]) => /delete|cancel/.test(command))).toBe(false);
    await act(async () => {
      next.resolve({ ...complete, operationId: NEXT_ID });
      await newer;
    });
    expect(f.onSuccess).toHaveBeenCalledOnce();
  });
  it.each(['complete', 'partial', 'failure'] as const)(
    'unmount ignores late %s without success/error/reload callbacks',
    async (outcome) => {
      const old = deferred<ImportResult>();
      io.invoke.mockImplementation(async (command: string) => {
        if (command === 'import_operations_list') return [];
        if (command === 'import_decrypt_preview') return tree;
        if (command === 'import_execute_advanced') return old.promise;
        throw new Error(command);
      });
      const f = mount();
      await prepare(f);
      let running!: Promise<void>;
      act(() => {
        running = f.result.current.onImport();
      });
      await waitFor(() => expect(executeCalls()).toHaveLength(1));
      f.unmount();
      await act(async () => {
        if (outcome === 'failure') old.reject(new Error('old failure'));
        else old.resolve(outcome === 'complete' ? complete : partial);
        await running;
      });
      expect(f.onError).not.toHaveBeenCalled();
      expect(f.onSuccess).not.toHaveBeenCalled();
      expect(f.reload).not.toHaveBeenCalled();
    },
  );
  it('same React batch lock and same-account unlock rejects an old callback before it can begin/abort a current request', async () => {
    const old = deferred<ImportResult>();
    const next = deferred<ImportResult>();
    io.invoke.mockImplementation(
      async (command: string, args?: { req?: AdvancedImportRequest }) => {
        if (command === 'import_operations_list') return [];
        if (command === 'import_decrypt_preview') return tree;
        if (command === 'import_execute_advanced')
          return args?.req?.operationId === ID ? old.promise : next.promise;
        throw new Error(command);
      },
    );
    const f = mount();
    await prepare(f);
    const oldHandler = f.result.current.onImport;
    let prior!: Promise<void>;
    act(() => {
      prior = oldHandler();
    });
    await waitFor(() => expect(executeCalls()).toHaveLength(1));
    act(() => {
      setRequestSession(null);
      setRequestSession('account-a');
    });
    await prepare(f);
    let current!: Promise<void>;
    act(() => {
      current = f.result.current.onImport();
    });
    await waitFor(() => expect(executeCalls()).toHaveLength(2));
    await act(async () => {
      await oldHandler();
      old.resolve(partial);
      await prior;
    });
    expect(executeCalls()).toHaveLength(2);
    expect(f.result.current.isImporting).toBe(true);
    expect(f.onError).not.toHaveBeenCalled();
    await act(async () => {
      next.resolve({ ...complete, operationId: NEXT_ID });
      await current;
    });
    expect(f.onSuccess).toHaveBeenCalledOnce();
  });
  it('account A delayed summary cannot replace B pending state or dispatch under account B', async () => {
    const old = deferred<ImportOperationSummary>();
    io.invoke.mockImplementation(async (command: string, args: { accountId: string }) => {
      if (command === 'import_operations_list')
        return [summary({ operationId: args.accountId === 'account-a' ? ID : NEXT_ID })];
      if (command === 'import_operation_get') return old.promise;
      throw new Error(command);
    });
    const f = mount();
    let reading!: Promise<void>;
    act(() => {
      reading = f.result.current.importOperations.onSelect(ID);
    });
    await waitFor(() =>
      expect(io.invoke.mock.calls.some(([command]) => command === 'import_operation_get')).toBe(
        true,
      ),
    );
    act(() => {
      setRequestSession('account-b');
      f.rerender({ accountId: 'account-b' });
    });
    await waitFor(() =>
      expect(f.result.current.importOperations.items[0]?.operationId).toBe(NEXT_ID),
    );
    await act(async () => {
      old.resolve(summary());
      await reading;
    });
    expect(f.result.current.importOperations.selected).toBeNull();
    expect(f.result.current.importOperations.items[0].operationId).toBe(NEXT_ID);
    expect(f.onError).not.toHaveBeenCalled();
    expect(
      io.invoke.mock.calls
        .find(([command]) => command === 'import_operation_get')?.[2]
        .requestIsCurrent(),
    ).toBe(false);
  });
  it('original-package dialog from A cannot publish after selecting B and returning to A', async () => {
    const picker = deferred<string | null>();
    io.choose.mockReturnValue(picker.promise);
    io.invoke.mockImplementation(async (command: string, args: { operationId?: string }) => {
      if (command === 'import_operations_list') return [];
      if (command === 'import_operation_get')
        return summary({ operationId: args.operationId ?? ID, sourceRequired: true });
      throw new Error(command);
    });
    const f = mount();
    await act(async () => {
      await f.result.current.importOperations.onSelect(ID);
    });
    let selecting!: Promise<void>;
    act(() => {
      selecting = f.result.current.importOperations.onPickSource();
    });
    await waitFor(() => expect(io.choose).toHaveBeenCalledOnce());
    await act(async () => {
      await f.result.current.importOperations.onSelect(NEXT_ID);
    });
    await act(async () => {
      await f.result.current.importOperations.onSelect(ID);
    });
    await act(async () => {
      picker.resolve('C:/late.solosoul');
      await selecting;
    });
    expect(f.result.current.importOperations.replacementSource).toBe('');
    expect(f.result.current.importOperations.selected?.operationId).toBe(ID);
  });
  it.each(['BAD_PASSWORD', 'DATABASE_WRITE_FAILED'])(
    'retrying a notCommitted %s result carrying the operation ID keeps frozen options after preview invalidation',
    async (errorCode) => {
      let attempts = 0;
      const rejected: ImportResult = {
        ...complete,
        operationId: ID,
        status: 'notCommitted',
        objectCount: 0,
        attachmentCount: 0,
        snapshotCount: 0,
        attachmentFilesWritten: 0,
        failureStage: 'preparation',
        errorCode,
      };
      io.invoke.mockImplementation(async (command: string) => {
        if (command === 'import_operations_list') return [];
        if (command === 'import_decrypt_preview') return tree;
        if (command === 'import_operation_get') throw '__IMPORT_ERR__:OPERATION_NOT_FOUND';
        if (command === 'import_execute_advanced') return ++attempts === 1 ? rejected : complete;
        throw new Error(command);
      });
      const f = mount();
      await prepare(f);
      await act(async () => {
        await f.result.current.onImport();
      });
      act(() => f.result.current.setImportPw('correct-package-password'));
      expect(f.result.current.importTotalSelected).toBe(0);
      await act(async () => {
        await f.result.current.importOperations.onRetry();
      });
      expect(executeCalls()).toHaveLength(2);
      expect(executeCalls()[1][1].req).toEqual({
        ...executeCalls()[0][1].req,
        password: 'correct-package-password',
      });
      expect(crypto.randomUUID).toHaveBeenCalledTimes(1);
    },
  );
  it('a completed native source is never passed to URI-cache deletion and a later fresh import is usable', async () => {
    io.invoke.mockImplementation(
      async (command: string, args?: { req?: AdvancedImportRequest }) => {
        if (command === 'import_operations_list') return [];
        if (command === 'import_decrypt_preview') return tree;
        if (command === 'import_execute_advanced')
          return { ...complete, operationId: args?.req?.operationId ?? null };
        throw new Error(command);
      },
    );
    const f = mount();
    await prepare(f);
    await act(async () => {
      await f.result.current.onImport();
    });
    expect(io.cleanup).not.toHaveBeenCalledWith('C:/original.solosoul');
    await prepare(f, 'C:/second.solosoul');
    await act(async () => {
      await f.result.current.onImport();
    });
    expect(executeCalls().map(([, args]) => args.req.operationId)).toEqual([ID, NEXT_ID]);
    expect(f.onSuccess).toHaveBeenCalledTimes(2);
  });
  it('a retained old source callback cannot allocate an ID or dispatch after a new source is selected', async () => {
    const f = mount();
    await prepare(f);
    const old = f.result.current.onImport;
    act(() => f.result.current.onSetImportPath('C:/different.solosoul'));
    await act(async () => {
      await old();
    });
    expect(crypto.randomUUID).not.toHaveBeenCalled();
    expect(executeCalls()).toHaveLength(0);
    expect(f.result.current.importPath).toBe('C:/different.solosoul');
  });
  it('a retained Resume callback cannot dispatch using another selected operation', async () => {
    io.invoke.mockImplementation(async (command: string, args?: { operationId?: string }) => {
      if (command === 'import_operations_list') return [];
      if (command === 'import_operation_get')
        return summary({ operationId: args?.operationId ?? ID });
      throw new Error(command);
    });
    const f = mount();
    await act(async () => {
      await f.result.current.importOperations.onSelect(ID);
    });
    const old = f.result.current.importOperations.onResume;
    await act(async () => {
      await f.result.current.importOperations.onSelect(NEXT_ID);
    });
    await act(async () => {
      await old();
    });
    expect(resumeCalls()).toHaveLength(0);
    expect(f.result.current.importOperations.selected?.operationId).toBe(NEXT_ID);
  });
  it('source proof mismatch retains the operation and never dispatches a Fresh replacement', async () => {
    io.choose.mockResolvedValue('C:/different-bytes.solosoul');
    io.invoke.mockImplementation(async (command: string) => {
      if (command === 'import_operations_list') return [];
      if (command === 'import_operation_get') return summary({ sourceRequired: true });
      if (command === 'import_operation_resume') throw '__IMPORT_ERR__:SOURCE_MISMATCH';
      throw new Error(command);
    });
    const f = mount();
    await act(async () => {
      await f.result.current.importOperations.onSelect(ID);
    });
    await act(async () => {
      await f.result.current.importOperations.onPickSource();
    });
    await act(async () => {
      await f.result.current.importOperations.onResume();
    });
    expect(f.result.current.importOperations.selected?.operationId).toBe(ID);
    expect(executeCalls()).toHaveLength(0);
    expect(crypto.randomUUID).not.toHaveBeenCalled();
    expect(f.onError).toHaveBeenCalledOnce();
  });
  it('registered recovery resumes without a random transfer password or recreating the account', async () => {
    io.invoke.mockImplementation(async (command: string) => {
      if (command === 'import_operations_list') return [];
      if (command === 'import_operation_get') return summary({ sourceKind: 'recovery' });
      if (command === 'import_operation_resume') return complete;
      throw new Error(command);
    });
    const f = mount();
    await act(async () => {
      await f.result.current.importOperations.onSelect(ID);
    });
    expect(f.result.current.importOperations.canResume).toBe(true);
    await act(async () => {
      await f.result.current.importOperations.onResume();
    });
    expect(resumeCalls()[0][1].password).toBeNull();
    expect(
      io.invoke.mock.calls.some(([command]) => /restore_from_host|create.*account/.test(command)),
    ).toBe(false);
  });
  it('a sourceRequired hint cannot silently replace the operation after lock during URI staging', async () => {
    const staged = deferred<string>();
    io.stage.mockReturnValue(staged.promise);
    io.choose.mockResolvedValue('content://original');
    io.invoke.mockImplementation(async (command: string) => {
      if (command === 'import_operations_list') return [];
      if (command === 'import_operation_get') return summary({ sourceRequired: true });
      throw new Error(command);
    });
    const f = mount();
    await act(async () => {
      await f.result.current.importOperations.onSelect(ID);
    });
    await act(async () => {
      await f.result.current.importOperations.onPickSource();
    });
    let running!: Promise<void>;
    act(() => {
      running = f.result.current.importOperations.onResume();
    });
    await waitFor(() => expect(io.stage).toHaveBeenCalledOnce());
    act(() => {
      setRequestSession(null);
      setRequestSession('account-a');
    });
    await act(async () => {
      staged.resolve('C:/cache/stale-resume.solosoul');
      await running;
    });
    expect(resumeCalls()).toHaveLength(0);
    expect(executeCalls()).toHaveLength(0);
    expect(io.cleanup).toHaveBeenCalledWith('C:/cache/stale-resume.solosoul');
    expect(f.onError).not.toHaveBeenCalled();
  });

  it('a retained pre-selection callback cannot freeze obsolete options after an explicit selection change', async () => {
    const f = mount();
    await prepare(f);
    const prior = f.result.current.onImport;
    act(() => f.result.current.onToggleSelection('object-a'));
    await act(async () => {
      await prior();
    });
    expect(executeCalls()).toHaveLength(0);
    expect(crypto.randomUUID).not.toHaveBeenCalled();
    await act(async () => {
      await f.result.current.onImport();
    });
    expect(executeCalls()[0][1].req.selections).toEqual([
      { objectId: 'object-a', selected: false },
      { objectId: 'object-b', selected: true },
    ]);
  });
  it('a retained pre-password Resume callback cannot dispatch the previous credential', async () => {
    io.invoke.mockImplementation(async (command: string) => {
      if (command === 'import_operations_list') return [];
      if (command === 'import_operation_get') return summary({ passwordRequired: true });
      if (command === 'import_operation_resume') return complete;
      throw new Error(command);
    });
    const f = mount();
    await act(async () => {
      await f.result.current.importOperations.onSelect(ID);
    });
    act(() => f.result.current.importOperations.onSetPassword('first-password'));
    const prior = f.result.current.importOperations.onResume;
    act(() => f.result.current.importOperations.onSetPassword('second-password'));
    await act(async () => {
      await prior();
    });
    expect(resumeCalls()).toHaveLength(0);
    await act(async () => {
      await f.result.current.importOperations.onResume();
    });
    expect(resumeCalls()[0][1].password).toBe('second-password');
  });
});

it('RF318 structured operation-missing reply replays only the original frozen operation', async () => {
  let writes = 0;
  io.invoke.mockImplementation(async (command: string) => {
    if (command === 'import_operations_list') return [];
    if (command === 'import_decrypt_preview') return tree;
    if (command === 'import_operation_get')
      throw new BackendCommandError(makeBackendError('IMPORT_OPERATION_NOT_FOUND'));
    if (command === 'import_execute_advanced') {
      if (++writes === 1)
        throw new BackendCommandError(makeBackendError('TRANSFER_TASK_UNCONFIRMED'));
      return complete;
    }
    throw new Error(command);
  });
  const f = mount();
  await prepare(f);
  act(() => {
    f.result.current.setShowStrategySelector(true);
    f.result.current.setImportStrategy('keepBoth');
    f.result.current.onSetObjectConflictStrategy('object-a', 'overwrite');
  });
  await act(async () => {
    await f.result.current.onImport();
  });
  expect(f.result.current.importOperations.currentId).toBe(ID);
  await act(async () => {
    await i18n.changeLanguage('zh-CN');
    await f.result.current.importOperations.onRetry();
  });
  expect(crypto.randomUUID).toHaveBeenCalledTimes(1);
  expect(executeCalls()).toHaveLength(2);
  expect(executeCalls()[1][1].req).toEqual(executeCalls()[0][1].req);
  expect(executeCalls()[0][1].req.objectStrategies).toEqual({ 'object-a': 'overwrite' });
  expect(f.onSuccess).toHaveBeenCalledOnce();
});
