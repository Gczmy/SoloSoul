import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { MrzResult, OcrResult } from '@/lib/ipc';

const { ipc } = vi.hoisted(() => ({ ipc: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: ipc }));
vi.mock('@/lib/logger', () => ({
  logger: { warn: vi.fn(), error: vi.fn(), info: vi.fn(), debug: vi.fn() },
}));

// 仅作为 IPC 参数的合成路径；测试不会访问文件系统或原生 OCR。
const A_PATH = 'C:/RF029-synthetic/account-a/passport.png';
const B_PATH = 'C:/RF029-synthetic/account-b/current.png';
const A_RESULT: OcrResult = { text: 'RF029_OLD_ACCOUNT_A', confidence: 0.82, boxes: [] };
const B_RESULT: OcrResult = { text: 'RF029_CURRENT_ACCOUNT_B', confidence: 0.94, boxes: [] };
const A_ERROR = 'RF029_SYNTHETIC_OLD_A_FAILURE';
const B_ERROR = 'RF029_SYNTHETIC_CURRENT_B_FAILURE';

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}

let store: typeof import('./ocrScanStore').useOcrScanStore;
let auth: typeof import('./authStore').useAuthStore;
let releaseReplies: (() => void)[];
let scans: Promise<void>[];

function track(work: Promise<void>) {
  scans.push(work);
  // 仍在测试正文 await 原 Promise；这里只防止失败断言提前退出时出现未处理拒绝。
  void work.catch(() => {});
  return work;
}

function callsFor(command: string, filePath: string) {
  return ipc.mock.calls.filter(([name, args]) => name === command && args?.filePath === filePath);
}

async function beginAThenB(mode: 'general' | 'mrz') {
  const oldReply = deferred<OcrResult | MrzResult | null>();
  const currentReply = deferred<OcrResult>();
  releaseReplies.push(
    () => oldReply.resolve(null),
    () => currentReply.resolve(B_RESULT),
  );
  const oldCommand = mode === 'mrz' ? 'ocr_scan_mrz' : 'ocr_scan_image';
  ipc.mockImplementation((command: string, args?: Record<string, unknown>) => {
    if (command === oldCommand && args?.filePath === A_PATH) return oldReply.promise;
    if (command === 'ocr_scan_image' && args?.filePath === B_PATH) return currentReply.promise;
    if (command === 'ocr_scan_image' && args?.filePath === A_PATH) {
      // 若旧实现错误地启动 fallback，立即返回合成结果，避免红测悬挂在第二个请求。
      return Promise.resolve(A_RESULT);
    }
    throw new Error(`Unexpected RF029 test command: ${command}`);
  });

  store.getState().setScanMode(mode);
  const oldWork = track(store.getState().performScan(A_PATH));
  await vi.waitFor(() => expect(callsFor(oldCommand, A_PATH)).toHaveLength(1));
  const oldId = store.getState().currentScanId;
  expect(store.getState().isScanning).toBe(true);

  // 使用真实 Auth Store 推进本地会话；显式调用生产 clear 对齐 AppRoutes 的锁定清理。
  auth.setState({ currentAccount: null, isAuthenticated: false });
  store.getState().clearOnVaultLock();
  expect(store.getState().scanHistory).toEqual([]);
  auth.getState().completeUnlock({ id: 'rf029-account-b', name: 'Synthetic B' });
  store.getState().setScanMode('general');
  const currentWork = track(store.getState().performScan(B_PATH));
  await vi.waitFor(() => expect(callsFor('ocr_scan_image', B_PATH)).toHaveLength(1));
  const currentId = store.getState().currentScanId;
  expect(currentId).toBeTruthy();
  expect(currentId).not.toBe(oldId);
  expect(store.getState().isScanning).toBe(true);

  return { oldReply, currentReply, oldWork, currentWork, currentId };
}

function expectBPending(currentId: string | null) {
  const state = store.getState();
  expect.soft(state.isScanning).toBe(true);
  expect.soft(state.currentScanId).toBe(currentId);
  expect.soft(state.lastScanError).toBeNull();
  expect.soft(state.scanHistory).toEqual([
    expect.objectContaining({
      id: currentId,
      filePath: B_PATH,
      mode: 'general',
      result: null,
      mrzResult: null,
    }),
  ]);
  expect.soft(state.scanHistory[0]?.error).toBeUndefined();
}

function expectBSuccess(currentId: string | null) {
  const state = store.getState();
  expect(state.isScanning).toBe(false);
  expect(state.currentScanId).toBe(currentId);
  expect(state.lastScanError).toBeNull();
  expect(state.scanHistory).toEqual([
    expect.objectContaining({ id: currentId, filePath: B_PATH, result: B_RESULT, mrzResult: null }),
  ]);
}

beforeEach(async () => {
  vi.resetModules();
  ipc.mockReset();
  localStorage.clear();
  releaseReplies = [];
  scans = [];
  auth = (await import('./authStore')).useAuthStore;
  store = (await import('./ocrScanStore')).useOcrScanStore;
  auth.getState().completeUnlock({ id: 'rf029-account-a', name: 'Synthetic A' });
});

afterEach(async () => {
  // 即使某个红测断言失败，也释放全部受控 IPC 并等待扫描收尾，不留下悬停任务。
  for (const release of releaseReplies) release();
  await Promise.allSettled(scans);
  store.getState().clearOnVaultLock();
  auth.setState({ currentAccount: null, isAuthenticated: false });
  localStorage.clear();
});

describe('RF029 OCR Store 会话失效后的迟到响应', () => {
  it('A MRZ 清空后返回 None 不发旧 fallback，也不结束仍悬停的 B 扫描', async () => {
    const run = await beginAThenB('mrz');

    run.oldReply.resolve(null);
    await run.oldWork;

    expect.soft(callsFor('ocr_scan_image', A_PATH)).toHaveLength(0);
    expectBPending(run.currentId);
    run.currentReply.resolve(B_RESULT);
    await run.currentWork;
    expectBSuccess(run.currentId);
    expect(callsFor('ocr_scan_image', B_PATH)).toHaveLength(1);
  });

  it('A 的迟到错误不覆盖 B 已完成扫描的真实错误', async () => {
    const run = await beginAThenB('general');
    run.currentReply.reject(new Error(B_ERROR));
    await run.currentWork;
    expect(store.getState().lastScanError).toBe(B_ERROR);
    const before = store.getState();
    const historyBefore = before.scanHistory.map((entry) => ({ ...entry }));

    run.oldReply.reject(new Error(A_ERROR));
    await run.oldWork;

    const after = store.getState();
    expect.soft(after.lastScanError).toBe(B_ERROR);
    expect.soft(after.isScanning).toBe(false);
    expect.soft(after.currentScanId).toBe(run.currentId);
    expect.soft(after.scanHistory).toEqual(historyBefore);
    expect.soft(after.scanHistory).toEqual([
      expect.objectContaining({
        filePath: B_PATH,
        error: B_ERROR,
        result: null,
        mrzResult: null,
      }),
    ]);
  });

  it('A 的迟到成功不进入 B，也不能提前把 B 的扫描标记为完成', async () => {
    const run = await beginAThenB('general');

    run.oldReply.resolve(A_RESULT);
    await run.oldWork;

    expectBPending(run.currentId);
    expect.soft(JSON.stringify(store.getState().scanHistory)).not.toContain(A_RESULT.text);
    run.currentReply.resolve(B_RESULT);
    await run.currentWork;
    expectBSuccess(run.currentId);
    expect(callsFor('ocr_scan_image', A_PATH)).toHaveLength(1);
    expect(callsFor('ocr_scan_image', B_PATH)).toHaveLength(1);
  });
});
