import { OCR_UNSUPPORTED_PLATFORM, supportsOcrScanSync } from './ocrCapabilities';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { invokeCommand } from '@/lib/ipcClient';
import { createSessionRequests } from '@/lib/sessionRequests';
import type { MrzResult, OcrResult } from '@/lib/ipc';

export type OcrJobState =
  | 'queued'
  | 'running'
  | 'cancelRequested'
  | 'completed'
  | 'cancelled'
  | 'failed'
  | 'stale';
export type OcrScanProgress = 'queued' | 'running' | 'cancelRequested';
export interface OcrJobEvent {
  taskId: string;
  accountId: string;
  sessionGeneration: number;
  state: OcrJobState;
  sequence: number;
}
export type OcrScanOutcome =
  | {
      status: 'completed';
      result: OcrResult | null;
      mrzResult: MrzResult | null;
      usedFallback: boolean;
    }
  | { status: 'cancelled' | 'stale' }
  | { status: 'failed'; error: string };

export function ocrErrorText(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

/** 原生状态只控制进度；正文和最终结果始终由 invoke 的 settle 决定。 */
export function createOcrScanOperation(options: {
  accountId?: string;
  isCurrent?: () => boolean;
  onState?: (state: OcrScanProgress) => void;
}) {
  const ticket = createSessionRequests().begin(undefined, options.accountId);
  let disposed = false;
  let finished = false;
  let started = false;
  let cancelRequested = false;
  let unlisten: UnlistenFn | undefined;
  let active: {
    id: string;
    sequence: number;
    generation?: number;
    settled: boolean;
    terminalSeen: boolean;
  } | null = null;
  const isCurrent = () => !disposed && ticket.isCurrent() && (options.isCurrent?.() ?? true);
  const publish = (state: OcrScanProgress) => {
    if (isCurrent() && !finished) options.onState?.(state);
  };
  const requestCancel = () => {
    const task = active;
    if (!task || task.settled || !ticket.isCurrent()) return;
    // false 表示尚未登记或已结束，不能据此释放 UI；queued/running 会补发取消意图。
    void invokeCommand<boolean>(
      'ocr_cancel_scan',
      { taskId: task.id },
      {
        requestIsCurrent: ticket.isCurrent,
      },
    ).catch(() => {
      // 取消为 best effort；原 invoke 和 Host 会话校验负责最终收尾。
    });
  };
  const cancel = () => {
    if (finished || disposed) return;
    cancelRequested = true;
    publish('cancelRequested');
    requestCancel();
  };
  const dispose = () => {
    // 卸载仍可取消同一会话的旧 task；会话已失效时不借用新账户发取消。
    if (!finished) {
      cancelRequested = true;
      requestCancel();
    }
    disposed = true;
  };
  const runTask = async <T>(command: string, filePath: string): Promise<T> => {
    if (!isCurrent()) throw new Error('__OCR_SESSION_STALE__');
    if (cancelRequested) throw new Error('__OCR_CANCELLED__');
    const task = {
      id: crypto.randomUUID(),
      sequence: -1,
      generation: undefined as number | undefined,
      settled: false,
      terminalSeen: false,
    };
    active = task;
    publish('queued');
    if (!isCurrent()) throw new Error('__OCR_SESSION_STALE__');
    if (cancelRequested) throw new Error('__OCR_CANCELLED__');
    try {
      return await ticket.invoke<T>(command, { filePath, taskId: task.id });
    } finally {
      task.settled = true;
    }
  };
  const run = async (filePath: string, mode: 'general' | 'mrz'): Promise<OcrScanOutcome> => {
    if (started) return { status: 'failed', error: '__OCR_DUPLICATE_TASK__' };
    started = true;
    try {
      if (!isCurrent()) return { status: 'stale' };
      if (!supportsOcrScanSync()) return { status: 'failed', error: OCR_UNSUPPORTED_PLATFORM };
      // 先安装监听再发送任何扫描，避免丢掉登记事件与取消补发机会。
      unlisten = await listen<OcrJobEvent>('ocr-job-state', ({ payload }) => {
        const task = active;
        // 卸载只抑制 UI；原会话仍有效时保留同一 task 的登记后取消补发。
        if (!ticket.isCurrent() || finished || !task || task.settled || task.terminalSeen) return;
        if (payload.taskId !== task.id) return;
        if (options.accountId && payload.accountId !== options.accountId) return;
        if (!Number.isSafeInteger(payload.sequence) || payload.sequence <= task.sequence) return;
        if (!Number.isSafeInteger(payload.sessionGeneration)) return;
        if (task.generation !== undefined && payload.sessionGeneration !== task.generation) return;
        const states: OcrJobState[] = [
          'queued',
          'running',
          'cancelRequested',
          'completed',
          'cancelled',
          'failed',
          'stale',
        ];
        if (!states.includes(payload.state)) return;
        task.generation = payload.sessionGeneration;
        task.sequence = payload.sequence;
        if (payload.state === 'queued' || payload.state === 'running') {
          publish(cancelRequested ? 'cancelRequested' : payload.state);
          if (cancelRequested) requestCancel();
        } else if (payload.state === 'cancelRequested') {
          cancelRequested = true;
          publish('cancelRequested');
        } else {
          task.terminalSeen = true;
        }
      });
      if (!isCurrent()) return { status: 'stale' };
      if (cancelRequested) return { status: 'cancelled' };
      if (mode === 'mrz') {
        const mrzResult = await runTask<MrzResult | null>('ocr_scan_mrz', filePath);
        if (!isCurrent()) return { status: 'stale' };
        if (mrzResult) return { status: 'completed', result: null, mrzResult, usedFallback: false };
        // MRZ 完成只是本用户操作的中间步骤；新任务沿用取消意图和原会话票据。
        if (cancelRequested) return { status: 'cancelled' };
      }
      const result = await runTask<OcrResult>('ocr_scan_image', filePath);
      if (!isCurrent()) return { status: 'stale' };
      // Host 已决定 completed 时，刚点取消不能把成功改写为 cancelled。
      return { status: 'completed', result, mrzResult: null, usedFallback: mode === 'mrz' };
    } catch (error) {
      if (!isCurrent()) return { status: 'stale' };
      const message = ocrErrorText(error);
      if (message === '__OCR_CANCELLED__') return { status: 'cancelled' };
      if (message === '__OCR_SESSION_STALE__') return { status: 'stale' };
      return { status: 'failed', error: message };
    } finally {
      finished = true;
      try {
        unlisten?.();
      } catch {
        // 监听清理失败不能改变已确定的扫描结果。
      }
    }
  };
  return { run, cancel, dispose, isCurrent };
}

export type OcrScanOperation = ReturnType<typeof createOcrScanOperation>;
