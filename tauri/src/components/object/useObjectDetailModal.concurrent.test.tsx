import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { useAuthStore } from '@/stores/authStore';
import {
  useObjectStore,
  type ObjectSummary,
  type ObjectData as ObjectDataView,
} from '@/stores/objectStore';
import type { ObjectData } from '@/lib/generated/ipcContracts';
import { toObjectDataView } from '@/lib/objectViewModel';
import { useObjectDetailModal } from './useObjectDetailModal';
import { useObjectEditLocked } from '@/hooks/useObjectEditLocked';

const dependencies = vi.hoisted(() => ({ templates: [], loadTemplates: async () => {} }));
vi.mock('@/stores/templateStore', () => ({
  useTemplateStore: (select: (state: typeof dependencies) => unknown) => select(dependencies),
}));
vi.mock('@/stores/settingsStore', () => {
  const state = { settings: { customPages: [] } };
  return { useSettingsStore: (select: (s: typeof state) => unknown) => select(state) };
});
vi.mock('@/hooks/useDragToAttach', () => ({
  useDragToAttach: () => ({ ref: { current: null }, dragState: 'idle' }),
}));
vi.mock('./useObjectDetailVerification', () => ({ useObjectDetailVerification: () => ({}) }));
vi.mock('@/lib/platform', () => ({
  isAndroidSync: () => false,
  isMobilePlatformSync: () => false,
}));

const plain: ObjectData = {
  id: 'object-a',
  accountId: 'acc-a',
  name: 'Public fixture',
  typeId: 'identity',
  sensitivityLevel: 'public',
  properties: { field: 'full public contents', __editLocked: false },
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
function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}
function summary(locked = false): ObjectSummary {
  return {
    id: plain.id,
    name: plain.name,
    typeId: plain.typeId,
    sensitivityLevel: 'public',
    properties: { field: 'short', __editLocked: locked },
    createdAt: plain.createdAt,
    updatedAt: plain.updatedAt,
  };
}
function detail(object: ObjectSummary) {
  return renderHook(
    (props) => {
      const result = useObjectDetailModal({ object: props.object, onClose: () => {} });
      return { ...result, editLocked: useObjectEditLocked(result.obj?.id, result.obj?.properties) };
    },
    { initialProps: { object } },
  );
}
beforeEach(() => {
  vi.mocked(invoke).mockReset().mockResolvedValue(undefined);
  useAuthStore.setState({ currentAccount: null, isAuthenticated: false });
  useAuthStore.getState().completeUnlock({ id: 'acc-a', name: 'Public A' });
  useObjectStore.getState().clearOnVaultLock();
});

it.each([true, false])(
  'late detail response cannot revert confirmed lock=%s or full data',
  async (locked) => {
    const read = deferred<ObjectData>();
    const stale = { ...plain, properties: { field: 'old full contents', __editLocked: !locked } };
    const saved = { ...plain, properties: { field: 'latest full contents', __editLocked: locked } };
    vi.mocked(invoke).mockImplementation((command) =>
      command === 'object_get' ? read.promise : Promise.resolve(saved),
    );
    const view = detail(summary(!locked));
    expect(useObjectStore.getState().isLoading).toBe(false);
    await act(async () => {
      await useObjectStore.getState().setObjectLocked(plain.id, locked);
    });
    expect(view.result.current.editLocked).toBe(locked);
    await act(async () => {
      read.resolve(stale);
      await read.promise;
    });
    expect(useObjectStore.getState().currentObjectCache[plain.id].properties).toEqual(
      saved.properties,
    );
    expect(view.result.current.editLocked).toBe(locked);
    expect(view.result.current.obj?.properties).toEqual(saved.properties);
    expect(useObjectStore.getState().isLoading).toBe(false);
    view.unmount();
  },
);

it('failed lock leaves valid full detail hydration available', async () => {
  const read = deferred<ObjectData>();
  vi.mocked(invoke).mockImplementation((command) =>
    command === 'object_get' ? read.promise : Promise.reject(new Error('public write failure')),
  );
  const view = detail(summary());
  await expect(useObjectStore.getState().setObjectLocked(plain.id, true)).rejects.toThrow();
  await act(async () => {
    read.resolve(plain);
    await read.promise;
  });
  expect(view.result.current.obj?.properties).toEqual(plain.properties);
  expect(view.result.current.editLocked).toBe(false);
  expect(useObjectStore.getState().error).toBeNull();
  view.unmount();
});

it('closing details prevents an in-flight response from filling the shared cache', async () => {
  const read = deferred<ObjectData>();
  vi.mocked(invoke).mockReturnValue(read.promise);
  const view = detail(summary());
  view.unmount();
  await act(async () => {
    read.resolve(plain);
    await read.promise;
  });
  expect(useObjectStore.getState().currentObjectCache).toEqual({});
  expect(useObjectStore.getState().isLoading).toBe(false);
});

it('switching objects aborts old hydration and retains the new full object', async () => {
  const read = deferred<ObjectData>();
  const next = { ...plain, id: 'object-b', name: 'Public B' };
  vi.mocked(invoke).mockImplementation((_command, args) =>
    (args as { objectId: string }).objectId === plain.id ? read.promise : Promise.resolve(next),
  );
  const view = detail(summary());
  view.rerender({ object: { ...summary(), id: next.id, name: next.name } });
  await waitFor(() => expect(view.result.current.obj?.id).toBe(next.id));
  await act(async () => {
    read.resolve(plain);
    await read.promise;
  });
  expect(useObjectStore.getState().currentObjectCache[plain.id]).toBeUndefined();
  expect(view.result.current.obj?.id).toBe(next.id);
  view.unmount();
});

it('A to locked to B blocks old detail data and preserves the current session', async () => {
  const read = deferred<ObjectData>();
  vi.mocked(invoke).mockReturnValue(read.promise);
  const view = detail(summary());
  await act(async () => {
    useAuthStore.setState({ currentAccount: null, isAuthenticated: false });
    useAuthStore.getState().completeUnlock({ id: 'acc-b', name: 'Public B' });
  });
  await act(async () => {
    read.resolve(plain);
    await read.promise;
  });
  expect(view.result.current.obj).toBeUndefined();
  expect(useObjectStore.getState().currentObjectCache).toEqual({});
  expect(useObjectStore.getState().error).toBeNull();
  view.unmount();
});

it('a complete replacement prop aborts an older summary read', async () => {
  const read = deferred<ObjectData>();
  vi.mocked(invoke).mockReturnValue(read.promise);
  const view = renderHook(
    (props: { object: ObjectSummary | ObjectDataView }) =>
      useObjectDetailModal({ object: props.object, onClose: () => {} }),
    { initialProps: { object: summary() as ObjectSummary | ObjectDataView } },
  );
  const saved = toObjectDataView({ ...plain, name: 'Fresh complete prop' });
  view.rerender({ object: saved });
  await act(async () => {
    read.resolve(plain);
    await read.promise;
  });
  expect(view.result.current.obj?.name).toBe(saved.name);
  expect(useObjectStore.getState().currentObjectCache).toEqual({});
  view.unmount();
});

it('a real read failure stays local and a later retry still hydrates details', async () => {
  vi.mocked(invoke)
    .mockRejectedValueOnce(new Error('public read failure'))
    .mockResolvedValue(plain);
  const first = detail(summary());
  await waitFor(() => expect(invoke).toHaveBeenCalledOnce());
  await act(async () => {
    await Promise.resolve();
  });
  expect(useObjectStore.getState().currentObjectCache).toEqual({});
  expect(useObjectStore.getState().error).toBeNull();
  first.unmount();
  const second = detail(summary());
  await waitFor(() => expect(second.result.current.obj?.properties).toEqual(plain.properties));
  expect(useObjectStore.getState().isLoading).toBe(false);
  second.unmount();
});
