import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { useAuthStore } from './authStore';
import {
  observeDesktopPageIndex,
  useDesktopPageIndexStore as index,
} from './desktopPageIndexStore';
import { setRequestSession } from '@/lib/sessionRequests';
import {
  notifyVaultCommand,
  notifyVaultDataChange,
  onVaultDataChange,
} from '@/lib/vaultDataChanges';
import type { ObjectSummary } from '@/lib/generated/ipcContracts';

const object = (id: string, sectionType = 'identity'): ObjectSummary => ({
  id,
  name: id,
  typeId: 'identity',
  sectionType,
  iconName: 'id-card',
  isDeleted: false,
  sensitivityLevel: 'private',
  createdAt: '',
  updatedAt: '',
  templateId: null,
  templateType: null,
  tags: [],
  hasAttachments: false,
  properties: { secret: 'must-not-be-cached' },
});
const deferred = <T>() => {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => {
    resolve = done;
  });
  return { promise, resolve };
};

describe('desktop page index', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    useAuthStore.setState({ currentAccount: { id: 'a' } as never, isAuthenticated: true });
    setRequestSession('a');
    index.getState().clear();
  });
  afterEach(() => {
    setRequestSession(null);
    index.getState().clear();
    vi.restoreAllMocks();
  });

  it('keeps only metadata, filters tombstones/page records and preserves custom section IDs', async () => {
    vi.mocked(invoke).mockResolvedValue([
      object('a'),
      object('b', 'custom-uuid'),
      { ...object('p'), typeId: 'page' },
      { ...object('deleted'), isDeleted: true },
      { ...object('unknown'), typeId: 'unknown' },
    ]);
    await index.getState().refresh();
    expect(index.getState().objects).toEqual([
      { id: 'a', name: 'a', sectionId: 'identity', iconName: 'id-card' },
      { id: 'b', name: 'b', sectionId: 'custom-uuid', iconName: 'id-card' },
    ]);
    expect(JSON.stringify(index.getState())).not.toContain('must-not-be-cached');
  });

  it('rejects stale reads after a newer mutation refresh or account lock', async () => {
    const old = deferred<ObjectSummary[]>();
    vi.mocked(invoke)
      .mockReturnValueOnce(old.promise)
      .mockResolvedValueOnce([object('new')]);
    const reading = index.getState().refresh();
    await index.getState().refresh();
    old.resolve([object('old')]);
    await reading;
    expect(index.getState().objects.map((o) => o.id)).toEqual(['new']);
    const locked = deferred<ObjectSummary[]>();
    vi.mocked(invoke).mockReturnValueOnce(locked.promise);
    const pending = index.getState().refresh();
    setRequestSession(null);
    locked.resolve([object('leak')]);
    await pending;
    expect(index.getState().objects).toEqual([]);
  });

  it('refreshes mounted index after successful writes, never after reads/dry runs, and stops when unmounted', async () => {
    vi.mocked(invoke).mockResolvedValue([object('a')]);
    const stop = observeDesktopPageIndex();
    await vi.waitFor(() => expect(index.getState().loading).toBe(false));
    vi.mocked(invoke).mockClear();
    notifyVaultCommand('object_list');
    notifyVaultCommand('object_sync_with_template', { dryRun: true });
    expect(invoke).not.toHaveBeenCalled();
    notifyVaultCommand('object_update');
    await vi.waitFor(() => expect(invoke).toHaveBeenCalledTimes(1));
    stop();
    notifyVaultDataChange();
    expect(invoke).toHaveBeenCalledTimes(1);
    expect(index.getState().objects).toEqual([]);
  });

  it('isolates refresh failures from writes and allows retry', async () => {
    const stop = onVaultDataChange(() => {
      throw new Error('subscriber failed');
    });
    expect(() => notifyVaultCommand('object_create')).not.toThrow();
    stop();
    vi.mocked(invoke)
      .mockRejectedValueOnce(new Error('unavailable'))
      .mockResolvedValueOnce([object('a')]);
    await index.getState().refresh();
    expect(index.getState().failed).toBe(true);
    await index.getState().refresh();
    expect(index.getState().failed).toBe(false);
    expect(index.getState().objects).toHaveLength(1);
  });
});
