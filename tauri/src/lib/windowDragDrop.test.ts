import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { DragDropPayload } from './windowDragDrop';

const mocks = vi.hoisted(() => ({ register: vi.fn(), warn: vi.fn() }));
vi.mock('@tauri-apps/api/webviewWindow', () => ({
  getCurrentWebviewWindow: () => ({ onDragDropEvent: mocks.register }),
}));
vi.mock('./logger', () => ({ logger: { warn: mocks.warn } }));

type NativeHandler = (event: { payload: DragDropPayload }) => void;
const over: DragDropPayload = { type: 'over', position: { x: 10, y: 20 } };
function deferred() {
  let resolve!: (unlisten: () => void) => void;
  let reject!: (error: Error) => void;
  const promise = new Promise<() => void>((done, fail) => {
    resolve = done;
    reject = fail;
  });
  return { promise, resolve, reject };
}

describe('shared window drag-drop lifecycle', () => {
  beforeEach(() => {
    vi.resetModules();
    vi.resetAllMocks();
  });

  it('uses one native registration for 50 receivers and releases only after the last', async () => {
    const unlisten = vi.fn();
    mocks.register.mockResolvedValue(unlisten);
    const { subscribeWindowDragDrop } = await import('./windowDragDrop');
    const handlers = Array.from({ length: 50 }, () => vi.fn());
    const dispose = handlers.map(subscribeWindowDragDrop);
    await vi.dynamicImportSettled();
    expect(mocks.register).toHaveBeenCalledTimes(1);
    const native = mocks.register.mock.calls[0][0] as NativeHandler;
    const payloads: DragDropPayload[] = [
      over,
      { type: 'leave' },
      { type: 'enter', paths: ['a'], position: over.position },
      { type: 'drop', paths: ['a'], position: over.position },
    ];
    for (const payload of payloads) {
      native({ payload });
      handlers.forEach((handler) => expect(handler).toHaveBeenLastCalledWith(payload));
    }
    dispose.slice(0, 49).forEach((stop) => stop());
    expect(unlisten).not.toHaveBeenCalled();
    native({ payload: over });
    expect(handlers[0]).toHaveBeenCalledTimes(4);
    expect(handlers[49]).toHaveBeenCalledTimes(5);
    dispose[49]();
    dispose[49]();
    expect(unlisten).toHaveBeenCalledTimes(1);
    native({ payload: over });
    expect(handlers[49]).toHaveBeenCalledTimes(5);
  });

  it('releases a late handle when every receiver left during registration', async () => {
    const pending = deferred();
    const unlisten = vi.fn();
    mocks.register.mockReturnValue(pending.promise);
    const { subscribeWindowDragDrop } = await import('./windowDragDrop');
    const handler = vi.fn();
    const stop = subscribeWindowDragDrop(handler);
    await vi.dynamicImportSettled();
    stop();
    (mocks.register.mock.calls[0][0] as NativeHandler)({ payload: over });
    expect(handler).not.toHaveBeenCalled();
    pending.resolve(unlisten);
    await pending.promise;
    expect(unlisten).toHaveBeenCalledTimes(1);
    stop();
    expect(unlisten).toHaveBeenCalledTimes(1);
  });

  it('reuses the pending native handle after an immediate unmount and remount', async () => {
    const pending = deferred();
    const unlisten = vi.fn();
    mocks.register.mockReturnValue(pending.promise);
    const { subscribeWindowDragDrop } = await import('./windowDragDrop');
    const old = vi.fn();
    subscribeWindowDragDrop(old)();
    const next = vi.fn();
    const stop = subscribeWindowDragDrop(next);
    await vi.dynamicImportSettled();
    expect(mocks.register).toHaveBeenCalledTimes(1);
    pending.resolve(unlisten);
    await pending.promise;
    expect(unlisten).not.toHaveBeenCalled();
    (mocks.register.mock.calls[0][0] as NativeHandler)({ payload: over });
    expect(old).not.toHaveBeenCalled();
    expect(next).toHaveBeenCalledWith(over);
    stop();
    expect(unlisten).toHaveBeenCalledTimes(1);
  });

  it('isolates receiver errors and skips a receiver removed during dispatch', async () => {
    mocks.register.mockResolvedValue(vi.fn());
    const { subscribeWindowDragDrop } = await import('./windowDragDrop');
    let removeSecond = () => {};
    const stops = [
      subscribeWindowDragDrop(() => {
        removeSecond();
        throw new Error('receiver failed');
      }),
    ];
    const removed = vi.fn();
    removeSecond = subscribeWindowDragDrop(removed);
    const remaining = vi.fn();
    stops.push(subscribeWindowDragDrop(remaining));
    await vi.dynamicImportSettled();
    (mocks.register.mock.calls[0][0] as NativeHandler)({ payload: over });
    expect(removed).not.toHaveBeenCalled();
    expect(remaining).toHaveBeenCalledWith(over);
    expect(mocks.warn).toHaveBeenCalledTimes(1);
    stops.forEach((stop) => stop());
  });

  it('keeps separate ownership for two subscriptions of the same callback', async () => {
    const unlisten = vi.fn();
    mocks.register.mockResolvedValue(unlisten);
    const { subscribeWindowDragDrop } = await import('./windowDragDrop');
    const handler = vi.fn();
    const first = subscribeWindowDragDrop(handler);
    const second = subscribeWindowDragDrop(handler);
    await vi.dynamicImportSettled();
    first();
    (mocks.register.mock.calls[0][0] as NativeHandler)({ payload: over });
    expect(handler).toHaveBeenCalledTimes(1);
    expect(unlisten).not.toHaveBeenCalled();
    second();
    expect(unlisten).toHaveBeenCalledTimes(1);
  });

  it('recovers on a fresh mount after unavailable native registration', async () => {
    const pending = deferred();
    const unlisten = vi.fn();
    mocks.register.mockReturnValueOnce(pending.promise).mockResolvedValueOnce(unlisten);
    const { subscribeWindowDragDrop } = await import('./windowDragDrop');
    const stop = subscribeWindowDragDrop(vi.fn());
    await vi.dynamicImportSettled();
    pending.reject(new Error('native unavailable'));
    await pending.promise.catch(() => {});
    expect(mocks.warn).toHaveBeenCalledTimes(1);
    stop();
    const handler = vi.fn();
    const nextStop = subscribeWindowDragDrop(handler);
    await vi.dynamicImportSettled();
    expect(mocks.register).toHaveBeenCalledTimes(2);
    (mocks.register.mock.calls[1][0] as NativeHandler)({ payload: over });
    expect(handler).toHaveBeenCalledWith(over);
    nextStop();
    expect(unlisten).toHaveBeenCalledTimes(1);
  });

  it('allows a fresh subscription even when native cleanup throws', async () => {
    mocks.register
      .mockResolvedValueOnce(() => {
        throw new Error('cleanup failed');
      })
      .mockResolvedValueOnce(vi.fn());
    const { subscribeWindowDragDrop } = await import('./windowDragDrop');
    const stop = subscribeWindowDragDrop(vi.fn());
    await vi.dynamicImportSettled();
    expect(stop).not.toThrow();
    expect(mocks.warn).toHaveBeenCalledTimes(1);
    const nextStop = subscribeWindowDragDrop(vi.fn());
    await vi.dynamicImportSettled();
    expect(mocks.register).toHaveBeenCalledTimes(2);
    nextStop();
  });
});
