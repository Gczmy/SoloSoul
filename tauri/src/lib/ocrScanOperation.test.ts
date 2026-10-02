import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { EventCallback } from '@tauri-apps/api/event';
import { createOcrScanOperation, type OcrJobEvent } from './ocrScanOperation';
import { setRequestSession } from './sessionRequests';

const mocks = vi.hoisted(() => ({ ipc: vi.fn(), listen: vi.fn(), unlisten: vi.fn(), ios: false }));
vi.mock('@/lib/platform', () => ({ isIOSSync: () => mocks.ios }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: mocks.ipc }));
vi.mock('@tauri-apps/api/event', () => ({ listen: mocks.listen }));

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}
const RESULT = { text: 'Synthetic OCR result', confidence: 0.92, boxes: [] };
const FILE = 'C:/RF029-synthetic/input.png';
let handler: EventCallback<OcrJobEvent>;
let releases: (() => void)[];
let works: Promise<unknown>[];

function pending<T>(fallback: T) {
  const reply = deferred<T>();
  releases.push(() => reply.resolve(fallback));
  return reply;
}
function track<T>(work: Promise<T>) {
  works.push(work);
  return work;
}
function scanCalls() {
  return mocks.ipc.mock.calls.filter(
    ([command]) => command === 'ocr_scan_image' || command === 'ocr_scan_mrz',
  );
}
async function task(index = 0) {
  await vi.waitFor(() => expect(scanCalls().length).toBeGreaterThan(index));
  return scanCalls()[index][1].taskId as string;
}
function emit(
  taskId: string,
  state: OcrJobEvent['state'],
  sequence: number,
  patch: Partial<OcrJobEvent> = {},
) {
  handler({
    event: 'ocr-job-state',
    id: 1,
    payload: {
      taskId,
      accountId: 'rf029-a',
      sessionGeneration: 4,
      state,
      sequence,
      ...patch,
    },
  });
}

beforeEach(() => {
  mocks.ios = false;
  setRequestSession(null);
  setRequestSession('rf029-a');
  mocks.ipc.mockReset();
  mocks.listen.mockReset().mockImplementation((_event, callback) => {
    handler = callback;
    return Promise.resolve(mocks.unlisten);
  });
  mocks.unlisten.mockReset();
  releases = [];
  works = [];
});
afterEach(async () => {
  for (const release of releases) release();
  await Promise.allSettled(works);
  setRequestSession(null);
});

describe('RF029 OCR 用户操作 helper', () => {
  it('先安装监听；取消早于 Host 登记时 false 不结束，queued 后补发', async () => {
    const ready = pending<() => void>(mocks.unlisten);
    mocks.listen.mockImplementation((_event, callback) => {
      handler = callback;
      return ready.promise;
    });
    const reply = pending(RESULT);
    mocks.ipc.mockImplementation((command) =>
      command === 'ocr_cancel_scan' ? Promise.resolve(false) : reply.promise,
    );
    const onState = vi.fn();
    const operation = createOcrScanOperation({ accountId: 'rf029-a', onState });
    let settled = false;
    const work = track(
      operation.run(FILE, 'general').then((outcome) => {
        settled = true;
        return outcome;
      }),
    );
    expect(scanCalls()).toHaveLength(0);
    ready.resolve(mocks.unlisten);
    const taskId = await task();
    expect(taskId).toMatch(
      /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i,
    );
    operation.cancel();
    await vi.waitFor(() =>
      expect(mocks.ipc.mock.calls.filter(([cmd]) => cmd === 'ocr_cancel_scan')).toHaveLength(1),
    );
    emit(taskId, 'queued', 1);
    await vi.waitFor(() =>
      expect(mocks.ipc.mock.calls.filter(([cmd]) => cmd === 'ocr_cancel_scan')).toHaveLength(2),
    );
    expect(onState).toHaveBeenLastCalledWith('cancelRequested');
    expect(settled).toBe(false);
    emit(taskId, 'cancelled', 2);
    expect(settled).toBe(false);
    reply.reject(new Error('__OCR_CANCELLED__'));
    expect(await work).toEqual({ status: 'cancelled' });
    expect(mocks.unlisten).toHaveBeenCalledTimes(1);
  });

  it('取消和成功竞争以 invoke 成功为准，不将已完成正文改成取消', async () => {
    const reply = pending(RESULT);
    mocks.ipc.mockImplementation((command) =>
      command === 'ocr_cancel_scan' ? Promise.resolve(false) : reply.promise,
    );
    const operation = createOcrScanOperation({ accountId: 'rf029-a' });
    const work = track(operation.run(FILE, 'general'));
    const taskId = await task();
    operation.cancel();
    emit(taskId, 'completed', 1);
    reply.resolve(RESULT);
    expect(await work).toEqual({
      status: 'completed',
      result: RESULT,
      mrzResult: null,
      usedFallback: false,
    });
  });

  it('拒绝旧任务、其他账户、其他代次、乱序事件，settle 后进度不能改写结果', async () => {
    const reply = pending(RESULT);
    mocks.ipc.mockReturnValue(reply.promise);
    const onState = vi.fn();
    const operation = createOcrScanOperation({ accountId: 'rf029-a', onState });
    const work = track(operation.run(FILE, 'general'));
    const taskId = await task();
    emit(taskId, 'running', 3);
    const accepted = onState.mock.calls.length;
    emit('different-task', 'cancelRequested', 8);
    emit(taskId, 'cancelRequested', 8, { accountId: 'rf029-b' });
    emit(taskId, 'cancelRequested', 8, { sessionGeneration: 5 });
    emit(taskId, 'queued', 2);
    expect(onState).toHaveBeenCalledTimes(accepted);
    emit(taskId, 'completed', 4);
    emit(taskId, 'queued', 5);
    expect(onState).toHaveBeenCalledTimes(accepted);
    reply.resolve(RESULT);
    expect((await work).status).toBe('completed');
    emit(taskId, 'cancelRequested', 20);
    expect(onState).toHaveBeenCalledTimes(accepted);
  });

  it('MRZ None 回退使用新 UUID，旧任务事件不污染后继，只返回一次用户操作结果', async () => {
    const mrz = pending<null>(null);
    const image = pending(RESULT);
    mocks.ipc.mockImplementation((command) =>
      command === 'ocr_scan_mrz' ? mrz.promise : image.promise,
    );
    const onState = vi.fn();
    const operation = createOcrScanOperation({ accountId: 'rf029-a', onState });
    let settled = false;
    const work = track(
      operation.run(FILE, 'mrz').then((value) => {
        settled = true;
        return value;
      }),
    );
    const first = await task();
    emit(first, 'completed', 1);
    mrz.resolve(null);
    const second = await task(1);
    expect(second).not.toBe(first);
    expect(settled).toBe(false);
    const calls = onState.mock.calls.length;
    emit(first, 'cancelRequested', 2);
    expect(onState).toHaveBeenCalledTimes(calls);
    image.resolve(RESULT);
    expect(await work).toEqual({
      status: 'completed',
      result: RESULT,
      mrzResult: null,
      usedFallback: true,
    });
    expect(mocks.listen).toHaveBeenCalledTimes(1);
    expect(mocks.unlisten).toHaveBeenCalledTimes(1);
  });

  it.each(['cancel', 'session'] as const)(
    'MRZ 悬停后 %s，None 不得启动后继扫描',
    async (change) => {
      const reply = pending<null>(null);
      mocks.ipc.mockImplementation((command) =>
        command === 'ocr_cancel_scan' ? Promise.resolve(false) : reply.promise,
      );
      const operation = createOcrScanOperation({ accountId: 'rf029-a' });
      const work = track(operation.run(FILE, 'mrz'));
      await task();
      if (change === 'cancel') operation.cancel();
      else setRequestSession('rf029-b');
      reply.resolve(null);
      expect(await work).toEqual({ status: change === 'cancel' ? 'cancelled' : 'stale' });
      expect(scanCalls()).toHaveLength(1);
    },
  );

  it('监听注册悬停时卸载，注册完成后释放监听且不派发扫描', async () => {
    const ready = pending<() => void>(mocks.unlisten);
    mocks.listen.mockReturnValue(ready.promise);
    const operation = createOcrScanOperation({ accountId: 'rf029-a' });
    const work = track(operation.run(FILE, 'general'));
    operation.dispose();
    ready.resolve(mocks.unlisten);
    expect(await work).toEqual({ status: 'stale' });
    expect(scanCalls()).toHaveLength(0);
    expect(mocks.unlisten).toHaveBeenCalledTimes(1);
  });

  it('卸载时提前取消未登记任务，后到 queued 仍补发且不更新 UI', async () => {
    const reply = pending(RESULT);
    mocks.ipc.mockImplementation((command) =>
      command === 'ocr_cancel_scan' ? Promise.resolve(false) : reply.promise,
    );
    const onState = vi.fn();
    const operation = createOcrScanOperation({ accountId: 'rf029-a', onState });
    const work = track(operation.run(FILE, 'general'));
    const taskId = await task();
    operation.dispose();
    const count = onState.mock.calls.length;
    emit(taskId, 'queued', 1);
    await vi.waitFor(() =>
      expect(mocks.ipc.mock.calls.filter(([cmd]) => cmd === 'ocr_cancel_scan')).toHaveLength(2),
    );
    expect(onState).toHaveBeenCalledTimes(count);
    reply.resolve(RESULT);
    expect(await work).toEqual({ status: 'stale' });
  });

  it.each([
    ['__OCR_CANCELLED__', { status: 'cancelled' }],
    ['__OCR_SESSION_STALE__', { status: 'stale' }],
    ['__OCR_QUEUE_FULL__', { status: 'failed', error: '__OCR_QUEUE_FULL__' }],
  ])('映射稳定机器错误 %s', async (error, expected) => {
    mocks.ipc.mockRejectedValue(new Error(String(error)));
    const work = track(createOcrScanOperation({ accountId: 'rf029-a' }).run(FILE, 'general'));
    expect(await work).toEqual(expected);
  });
});

describe('RF-203 iOS 操作前置门控', () => {
  it.each(['general', 'mrz'] as const)('%s 不创建桥接任务和进度订阅', async (mode) => {
    mocks.ios = true;
    mocks.ipc.mockResolvedValue(RESULT);
    const onState = vi.fn();
    const operation = createOcrScanOperation({ accountId: 'rf029-a', onState });
    expect(await operation.run(FILE, mode)).toEqual({
      status: 'failed',
      error: '__OCR_UNSUPPORTED_PLATFORM__',
    });
    expect(mocks.ipc).not.toHaveBeenCalled();
    expect(mocks.listen).not.toHaveBeenCalled();
    expect(onState).not.toHaveBeenCalled();
  });
});
