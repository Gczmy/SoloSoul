import { describe, it, expect, vi, afterEach } from 'vitest';
const invoke = vi.hoisted(() => vi.fn());
vi.mock('@tauri-apps/api/core', () => ({ invoke }));
import fixtures from '../../src-tauri/src/commands/object/tests/rf307-fixtures.json';

describe('RF307 actual object Store failure state', () => {
  afterEach(() => {
    invoke.mockReset();
    vi.resetModules();
  });
  it('preserves wire information for display translation without raw failure content', async () => {
    const { useObjectStore } = await import('./objectStore');
    invoke.mockRejectedValue({ ...fixtures.rollbackMissing, message: 'RF307-secret' });
    await useObjectStore.getState().loadObjects('synthetic');
    expect(useObjectStore.getState().error).toEqual(fixtures.rollbackMissing);
    expect(useObjectStore.getState().isLoading).toBe(false);
    expect(JSON.stringify(useObjectStore.getState())).not.toContain('RF307-secret');
  });
  it('old account rejection cannot refill cleared Store and write failure still rejects', async () => {
    const { useObjectStore } = await import('./objectStore');
    const { setRequestSession } = await import('@/lib/sessionRequests');
    setRequestSession('synthetic-a');
    let reject!: (error: unknown) => void;
    invoke.mockReturnValue(
      new Promise((_, r) => {
        reject = r;
      }),
    );
    const pending = useObjectStore.getState().loadObjects('synthetic-a');
    setRequestSession('synthetic-b');
    reject(fixtures.writeFailed);
    await pending;
    expect(useObjectStore.getState().error).toBeNull();
    expect(useObjectStore.getState().objects).toEqual([]);
    invoke.mockRejectedValue(fixtures.writeFailed);
    await expect(useObjectStore.getState().deleteObject('synthetic')).rejects.toMatchObject({
      backend: fixtures.writeFailed,
    });
    expect(useObjectStore.getState().error).toEqual(fixtures.writeFailed);
    setRequestSession(null);
  });
});
