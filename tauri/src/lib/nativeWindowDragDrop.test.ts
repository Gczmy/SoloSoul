import { beforeEach, describe, expect, it, vi } from 'vitest';
import { registerNativeWindowDragDrop } from './nativeWindowDragDrop';

const mocks = vi.hoisted(() => ({ listen: vi.fn(), warn: vi.fn() }));
vi.mock('@tauri-apps/api/webviewWindow', () => ({
  getCurrentWebviewWindow: () => ({ listen: mocks.listen }),
}));
vi.mock('./logger', () => ({ logger: { warn: mocks.warn } }));

type NativeHandler = (event: { payload?: unknown }) => void;
const position = { x: 10, y: 20 };
const names = [
  'tauri://drag-enter',
  'tauri://drag-over',
  'tauri://drag-drop',
  'tauri://drag-leave',
];

function readyListeners() {
  const handles: ReturnType<typeof vi.fn>[] = [];
  mocks.listen.mockImplementation(async () => {
    const unlisten = vi.fn();
    handles.push(unlisten);
    return unlisten;
  });
  return handles;
}

describe('native window drag-drop registration rollback', () => {
  beforeEach(() => vi.resetAllMocks());

  it('preserves all four Tauri window events, physical positions and payload shapes', async () => {
    const handles = readyListeners();
    const handler = vi.fn();
    const stop = await registerNativeWindowDragDrop(handler);
    expect(mocks.listen.mock.calls.map(([name]) => name)).toEqual(names);
    const callbacks = mocks.listen.mock.calls.map(([, callback]) => callback as NativeHandler);
    callbacks[0]({ payload: { paths: ['a.pdf'], position } });
    callbacks[1]({ payload: { paths: ['ignored'], position } });
    callbacks[2]({ payload: { paths: ['b.pdf'], position } });
    callbacks[3]({});
    expect(handler.mock.calls.map(([payload]) => payload)).toEqual([
      {
        type: 'enter',
        paths: ['a.pdf'],
        position: expect.objectContaining({ ...position, type: 'Physical' }),
      },
      { type: 'over', position: expect.objectContaining({ ...position, type: 'Physical' }) },
      {
        type: 'drop',
        paths: ['b.pdf'],
        position: expect.objectContaining({ ...position, type: 'Physical' }),
      },
      { type: 'leave' },
    ]);
    stop();
    stop();
    handles.forEach((handle) => expect(handle).toHaveBeenCalledTimes(1));
    callbacks[2]({ payload: { paths: ['late.pdf'], position } });
    expect(handler).toHaveBeenCalledTimes(4);
  });

  it('rolls back a partial failure and rejects queued callbacks from that attempt', async () => {
    const handles = readyListeners();
    const error = new Error('third registration rejected');
    mocks.listen
      .mockImplementationOnce(async () => {
        const handle = vi.fn();
        handles.push(handle);
        return handle;
      })
      .mockImplementationOnce(async () => {
        const handle = vi.fn();
        handles.push(handle);
        return handle;
      })
      .mockRejectedValueOnce(error);
    const handler = vi.fn();
    await expect(registerNativeWindowDragDrop(handler)).rejects.toBe(error);
    expect(handles).toHaveLength(2);
    handles.forEach((handle) => expect(handle).toHaveBeenCalledTimes(1));
    const failedCallbacks = mocks.listen.mock.calls
      .slice(0, 2)
      .map(([, callback]) => callback as NativeHandler);
    const stop = await registerNativeWindowDragDrop(handler);
    expect(mocks.listen.mock.calls.slice(3).map(([name]) => name)).toEqual(names);
    failedCallbacks.forEach((callback) => callback({ payload: { paths: ['old.pdf'], position } }));
    expect(handler).not.toHaveBeenCalled();
    (mocks.listen.mock.calls[5][1] as NativeHandler)({
      payload: { paths: ['current.pdf'], position },
    });
    expect(handler).toHaveBeenCalledTimes(1);
    stop();
    expect(handles).toHaveLength(6);
    handles.forEach((handle) => expect(handle).toHaveBeenCalledTimes(1));
  });

  it('does not finish a failed registration before asynchronous rollback finishes', async () => {
    let resolve!: () => void;
    const rollback = new Promise<void>((done) => {
      resolve = done;
    });
    const unlisten = vi.fn(() => rollback);
    const error = new Error('second registration rejected');
    mocks.listen.mockResolvedValueOnce(unlisten).mockRejectedValueOnce(error);
    let outcome: unknown;
    const attempt = registerNativeWindowDragDrop(vi.fn()).catch((reason) => {
      outcome = reason;
    });
    await vi.dynamicImportSettled();
    expect(unlisten).toHaveBeenCalledTimes(1);
    expect(outcome).toBeUndefined();
    resolve();
    await attempt;
    expect(outcome).toBe(error);
  });

  it('recovers through the shared subscription and keeps both old and new cards', async () => {
    const handles = readyListeners();
    const firstHandle = vi.fn();
    mocks.listen
      .mockResolvedValueOnce(firstHandle)
      .mockRejectedValueOnce(new Error('temporary rejection'));
    const { subscribeWindowDragDrop } = await import('./windowDragDrop');
    const original = vi.fn();
    const stopOriginal = subscribeWindowDragDrop(original);
    await vi.dynamicImportSettled();
    expect(mocks.listen).toHaveBeenCalledTimes(2);
    expect(firstHandle).toHaveBeenCalledTimes(1);
    const next = vi.fn();
    const third = vi.fn();
    const stopNext = subscribeWindowDragDrop(next);
    const stopThird = subscribeWindowDragDrop(third);
    await vi.dynamicImportSettled();
    expect(mocks.listen).toHaveBeenCalledTimes(6);
    (mocks.listen.mock.calls[0][1] as NativeHandler)({
      payload: { paths: ['stale.pdf'], position },
    });
    expect(original).not.toHaveBeenCalled();
    (mocks.listen.mock.calls[4][1] as NativeHandler)({
      payload: { paths: ['current.pdf'], position },
    });
    for (const receiver of [original, next, third]) expect(receiver).toHaveBeenCalledTimes(1);
    stopOriginal();
    stopNext();
    handles.forEach((handle) => expect(handle).not.toHaveBeenCalled());
    stopThird();
    expect(handles).toHaveLength(4);
    handles.forEach((handle) => expect(handle).toHaveBeenCalledTimes(1));
  });

  it('attempts every cleanup even when one throws or rejects, preserving the original failure', async () => {
    const first = vi.fn(() => {
      throw new Error('sync cleanup failure');
    });
    const second = vi.fn().mockRejectedValue(new Error('async cleanup failure'));
    const original = new Error('third registration rejected');
    mocks.listen
      .mockResolvedValueOnce(first)
      .mockResolvedValueOnce(second)
      .mockRejectedValueOnce(original);
    await expect(registerNativeWindowDragDrop(vi.fn())).rejects.toBe(original);
    expect(first).toHaveBeenCalledTimes(1);
    expect(second).toHaveBeenCalledTimes(1);
    expect(mocks.warn).toHaveBeenCalledTimes(2);
  });

  it('also observes asynchronous cleanup failures after a successful subscription', async () => {
    const handles = readyListeners();
    const stop = await registerNativeWindowDragDrop(vi.fn());
    handles[0].mockRejectedValue(new Error('cleanup rejection'));
    stop();
    await vi.dynamicImportSettled();
    handles.forEach((handle) => expect(handle).toHaveBeenCalledTimes(1));
    expect(mocks.warn).toHaveBeenCalledTimes(1);
  });
});
