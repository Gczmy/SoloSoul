import { beforeEach, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { useObjectStore } from './objectStore';
import { useAuthStore } from './authStore';
import type { ObjectData } from '@/lib/generated/ipcContracts';

const plain: ObjectData = {
  id: 'object-a',
  accountId: 'acc-a',
  name: 'Public fixture',
  typeId: 'identity',
  sensitivityLevel: 'public',
  properties: { field: 'old public contents', __editLocked: false },
  templateId: null,
  templateType: null,
  templateHash: null,
  ignoredTemplateHash: null,
  propertyLabels: null,
  contractTypeId: null,
  deletedAt: null,
  createdAt: '2026-01-01',
  updatedAt: '2026-01-01',
};
const saved: ObjectData = {
  ...plain,
  name: 'Saved public fixture',
  properties: { field: 'new public contents', __editLocked: true },
};
function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}
const updateInput = () => ({
  name: saved.name,
  sensitivityLevel: 'public',
  properties: { field: 'new public contents' },
});
const mutations = {
  update: () => useObjectStore.getState().updateObject(plain.id, updateInput()),
  lock: () => useObjectStore.getState().setObjectLocked(plain.id, true),
  delete: () => useObjectStore.getState().deleteObject(plain.id),
};
beforeEach(() => {
  vi.mocked(invoke).mockReset().mockResolvedValue(undefined);
  useAuthStore.setState({ currentAccount: null, isAuthenticated: false });
  useAuthStore.getState().completeUnlock({ id: 'acc-a', name: 'Public A' });
  useObjectStore.getState().clearOnVaultLock();
});

it.each(Object.keys(mutations) as Array<keyof typeof mutations>)(
  '%s acknowledgement survives a later read and rejects stale hydration',
  async (kind) => {
    const write = deferred<ObjectData>();
    const read = deferred<ObjectData>();
    vi.mocked(invoke).mockImplementation((command) =>
      command === 'object_get' ? read.promise : write.promise,
    );
    const writing = mutations[kind]();
    await vi.waitFor(() => expect(invoke).toHaveBeenCalledTimes(1));
    const reading = useObjectStore.getState().getObject(plain.accountId, plain.id);
    write.resolve(saved);
    await expect(writing).resolves.toEqual(
      kind === 'delete' ? undefined : expect.objectContaining({ name: saved.name }),
    );
    read.resolve(plain);
    expect(await reading).toBeUndefined();
    expect(useObjectStore.getState().currentObjectCache[plain.id]).toEqual(
      kind === 'delete' ? undefined : expect.objectContaining({ name: saved.name }),
    );
    expect(useObjectStore.getState().isLoading).toBe(false);
  },
);

it.each(Object.keys(mutations) as Array<keyof typeof mutations>)(
  '%s success after an earlier read cleans loading when the stale read settles',
  async (kind) => {
    const read = deferred<ObjectData>();
    vi.mocked(invoke).mockImplementation((command) =>
      command === 'object_get' ? read.promise : Promise.resolve(saved),
    );
    const reading = useObjectStore.getState().getObject(plain.accountId, plain.id);
    await mutations[kind]();
    expect(useObjectStore.getState().isLoading).toBe(true);
    read.resolve(plain);
    expect(await reading).toBeUndefined();
    expect(useObjectStore.getState().isLoading).toBe(false);
    expect(useObjectStore.getState().error).toBeNull();
  },
);

it.each(Object.keys(mutations) as Array<keyof typeof mutations>)(
  '%s failure still rejects and keeps an existing read valid',
  async (kind) => {
    const read = deferred<ObjectData>();
    vi.mocked(invoke).mockImplementation((command) =>
      command === 'object_get'
        ? read.promise
        : Promise.reject(new Error('synthetic write failure')),
    );
    const reading = useObjectStore.getState().getObject(plain.accountId, plain.id);
    await expect(mutations[kind]()).rejects.toThrow();
    read.resolve(plain);
    expect(await reading).toEqual(expect.objectContaining({ name: plain.name }));
    expect(useObjectStore.getState().currentObjectCache[plain.id].name).toBe(plain.name);
    expect(useObjectStore.getState().isLoading).toBe(false);
  },
);

it('serializes same-object mutations and captures queued update input at request time', async () => {
  const first = deferred<ObjectData>();
  const second = deferred<ObjectData>();
  vi.mocked(invoke)
    .mockReturnValueOnce(first.promise)
    .mockReturnValueOnce(second.promise)
    .mockResolvedValue(undefined);
  const locking = mutations.lock();
  const input = updateInput();
  const updating = useObjectStore.getState().updateObject(plain.id, input);
  const deleting = mutations.delete();
  input.properties.field = 'caller changed the draft while queued';
  await vi.waitFor(() => expect(invoke).toHaveBeenCalledTimes(1));
  expect(vi.mocked(invoke).mock.calls[0][0]).toBe('object_set_locked');
  first.resolve(saved);
  await locking;
  await vi.waitFor(() => expect(invoke).toHaveBeenCalledTimes(2));
  expect(invoke).toHaveBeenNthCalledWith(
    2,
    'object_update',
    expect.objectContaining({
      input: expect.objectContaining({ properties: { field: 'new public contents' } }),
    }),
  );
  second.resolve(saved);
  await updating;
  await deleting;
  expect(vi.mocked(invoke).mock.calls.map(([command]) => command)).toEqual([
    'object_set_locked',
    'object_update',
    'object_delete',
  ]);
  expect(useObjectStore.getState().currentObjectCache).toEqual({});
  expect(useObjectStore.getState().isLoading).toBe(false);
});

it('a failed mutation rejects its caller without poisoning the next queued mutation', async () => {
  const first = deferred<ObjectData>();
  vi.mocked(invoke).mockReturnValueOnce(first.promise).mockResolvedValue(saved);
  const updating = mutations.update();
  const failed = expect(updating).rejects.toThrow();
  const locking = mutations.lock();
  first.reject(new Error('synthetic failure'));
  await failed;
  await expect(locking).resolves.toEqual(expect.objectContaining({ name: saved.name }));
  expect(useObjectStore.getState().currentObjectCache[plain.id].name).toBe(saved.name);
  expect(useObjectStore.getState().isLoading).toBe(false);
});

it.each(['acc-b', 'acc-a'])(
  'old queued writes cannot dispatch after a lock and unlock as %s or clear new loading',
  async (accountId) => {
    const old = deferred<ObjectData>();
    const fresh = deferred<ObjectData>();
    vi.mocked(invoke).mockImplementation((command) =>
      command === 'object_get' ? fresh.promise : old.promise,
    );
    const writing = mutations.update();
    const queued = mutations.lock();
    const rejectedWrite = expect(writing).rejects.toThrow('expired session');
    const rejectedQueue = expect(queued).rejects.toThrow('expired session');
    await vi.waitFor(() => expect(invoke).toHaveBeenCalledTimes(1));
    useAuthStore.setState({ currentAccount: null, isAuthenticated: false });
    useAuthStore.getState().completeUnlock({ id: accountId, name: 'Public current' });
    const reading = useObjectStore.getState().getObject(accountId, plain.id);
    old.resolve(saved);
    await rejectedWrite;
    await rejectedQueue;
    expect(vi.mocked(invoke).mock.calls.map(([command]) => command)).toEqual([
      'object_update',
      'object_get',
    ]);
    expect(useObjectStore.getState().isLoading).toBe(true);
    fresh.resolve({ ...plain, accountId, name: 'Current session' });
    await reading;
    expect(useObjectStore.getState().isLoading).toBe(false);
    expect(useObjectStore.getState().currentObjectCache[plain.id].name).toBe('Current session');
  },
);

it('overlapping list and object reads retain loading until all current-session owners finish', async () => {
  const oldList = deferred<unknown[]>();
  const newList = deferred<unknown[]>();
  const read = deferred<ObjectData>();
  vi.mocked(invoke)
    .mockReturnValueOnce(oldList.promise)
    .mockReturnValueOnce(newList.promise)
    .mockReturnValueOnce(read.promise);
  const first = useObjectStore.getState().loadObjects('acc-a');
  const second = useObjectStore.getState().loadObjects('acc-a');
  const third = useObjectStore.getState().getObject('acc-a', plain.id);
  newList.resolve([]);
  await second;
  expect(useObjectStore.getState().isLoading).toBe(true);
  read.resolve(plain);
  await third;
  expect(useObjectStore.getState().isLoading).toBe(true);
  oldList.resolve([]);
  await first;
  expect(useObjectStore.getState().isLoading).toBe(false);
});

it('a superseded read and a newer failed read cannot leave loading stuck', async () => {
  const old = deferred<ObjectData>();
  vi.mocked(invoke)
    .mockReturnValueOnce(old.promise)
    .mockRejectedValueOnce(new Error('read failure'));
  const first = useObjectStore.getState().getObject('acc-a', plain.id);
  await useObjectStore.getState().getObject('acc-a', plain.id);
  expect(useObjectStore.getState().isLoading).toBe(true);
  old.resolve(plain);
  await first;
  expect(useObjectStore.getState().isLoading).toBe(false);
  expect(useObjectStore.getState().currentObjectCache).toEqual({});
  expect(useObjectStore.getState().error).not.toBeNull();
});

it('committed template sync keeps its success receipt when its follow-up read fails', async () => {
  const result = { added: [], removed: [], updated: [] };
  vi.mocked(invoke).mockImplementation((command) =>
    command === 'object_sync_with_template'
      ? Promise.resolve(result)
      : Promise.reject(new Error('read failure')),
  );
  await expect(useObjectStore.getState().applySyncTemplate('acc-a', plain.id)).resolves.toEqual(
    result,
  );
  expect(useObjectStore.getState().error).not.toBeNull();
  expect(useObjectStore.getState().isLoading).toBe(false);
});

it('confirmed template-ignore invalidates an older read without cancelling its own receipt', async () => {
  const read = deferred<ObjectData>();
  vi.mocked(invoke).mockImplementation((command) =>
    command === 'object_get' ? read.promise : Promise.resolve(undefined),
  );
  const reading = useObjectStore.getState().getObject('acc-a', plain.id);
  await useObjectStore.getState().ignoreTemplateSync(plain.id, 'public-hash');
  read.resolve(plain);
  expect(await reading).toBeUndefined();
  expect(useObjectStore.getState().currentObjectCache).toEqual({});
  expect(useObjectStore.getState().isLoading).toBe(false);
});

it('normal reads and quiet detail hydration remain independent without any write', async () => {
  const quiet = deferred<ObjectData>();
  const normal = deferred<ObjectData>();
  vi.mocked(invoke).mockReturnValueOnce(quiet.promise).mockReturnValueOnce(normal.promise);
  const first = useObjectStore.getState().getObject('acc-a', plain.id, { quiet: true });
  const second = useObjectStore.getState().getObject('acc-a', plain.id);
  quiet.resolve(plain);
  expect(await first).toEqual(expect.objectContaining({ name: plain.name }));
  expect(useObjectStore.getState().isLoading).toBe(true);
  normal.resolve(saved);
  expect(await second).toEqual(expect.objectContaining({ name: saved.name }));
  expect(useObjectStore.getState().isLoading).toBe(false);
});

it('old-session queue cleanup cannot remove the new session tail for the same object', async () => {
  const old = deferred<ObjectData>();
  const fresh = deferred<ObjectData>();
  vi.mocked(invoke)
    .mockReturnValueOnce(old.promise)
    .mockReturnValueOnce(fresh.promise)
    .mockResolvedValue(undefined);
  const oldWrite = mutations.update();
  const rejected = expect(oldWrite).rejects.toThrow('expired session');
  await vi.waitFor(() => expect(invoke).toHaveBeenCalledTimes(1));
  useAuthStore.setState({ currentAccount: null, isAuthenticated: false });
  useAuthStore.getState().completeUnlock({ id: 'acc-b', name: 'Public B' });
  const newWrite = useObjectStore.getState().setObjectLocked(plain.id, true);
  await vi.waitFor(() => expect(invoke).toHaveBeenCalledTimes(2));
  old.resolve(saved);
  await rejected;
  const newDelete = useObjectStore.getState().deleteObject(plain.id);
  await Promise.resolve();
  expect(invoke).toHaveBeenCalledTimes(2);
  fresh.resolve({ ...saved, accountId: 'acc-b' });
  await newWrite;
  await newDelete;
  expect(vi.mocked(invoke).mock.calls.map(([command]) => command)).toEqual([
    'object_update',
    'object_set_locked',
    'object_delete',
  ]);
  expect(useObjectStore.getState().currentObjectCache).toEqual({});
  expect(useObjectStore.getState().isLoading).toBe(false);
});
