import type { IpcEvents } from '@/lib/generated/ipcContracts';
import { create } from 'zustand';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { logger } from '@/lib/logger';
import { normalizeSyncError, backendErrorLogDetails } from '@/lib/backendErrorWire';

export type SyncProgressPayload = IpcEvents['sync-progress'];

/** 简化的同步状态。 */
export type SyncStatus = 'idle' | 'syncing' | 'completed' | 'error';

interface SafSyncState {
  /** 当前同步状态。 */
  status: SyncStatus;
  /** 当前阶段（用于 progress bar 等扩展操作）。 */
  phase: string | null;
  /** 完成百分比（null 表示不确定）。 */
  progress: { current: number; total: number } | null;
  /** 最后一次同步完成的时间戳。 */
  lastSyncedAt: number | null;
  /** 错误消息。 */
  error: string | null;
  /** 清理函数。 */
  _unlisten: UnlistenFn | null;
  _unlistenPromise: Promise<UnlistenFn> | null;

  /** 开始监听 sync-progress 事件。 */
  startListening: () => void;
  /** 停止监听并重置。 */
  stopListening: () => void;
  /** 手动重置为闲置状态。 */
  reset: () => void;
}

export const useSafSyncStore = create<SafSyncState>((set, get) => {
  let listenerGeneration = 0;
  let resetTimer: ReturnType<typeof setTimeout> | null = null;
  const clearResetTimer = () => {
    if (resetTimer !== null) clearTimeout(resetTimer);
    resetTimer = null;
  };

  return {
    status: 'idle',
    phase: null,
    progress: null,
    lastSyncedAt: null,
    error: null,
    _unlisten: null,
    _unlistenPromise: null,

    startListening: () => {
      const state = get();
      // 避免重复注册
      if (state._unlisten || state._unlistenPromise) return;
      const generation = ++listenerGeneration;

      const pending = listen<SyncProgressPayload>('sync-progress', (event) => {
        if (generation !== listenerGeneration) return;
        const { phase, current, total, message, silent } = event.payload;

        // 周期性兜底同步标记为 silent，前端不显示任何提示。
        if (silent) {
          return;
        }

        switch (phase) {
          case 'sync_start':
          case 'sync_to_remote':
          case 'sync_from_remote':
          case 'migrate':
            clearResetTimer();
            set({
              status: 'syncing',
              phase,
              progress: current != null && total != null ? { current, total } : null,
              error: null,
            });
            break;

          case 'sync_complete':
            clearResetTimer();
            set({
              status: 'completed',
              phase,
              progress: current != null && total != null ? { current, total } : null,
              lastSyncedAt: Date.now(),
              error: null,
            });
            // 3 秒后自动恢复到 idle
            resetTimer = setTimeout(() => {
              resetTimer = null;
              if (generation !== listenerGeneration) return;
              const s = get();
              if (s.status === 'completed') {
                set({ status: 'idle', phase: null, progress: null });
              }
            }, 3000);
            break;

          case 'error':
            clearResetTimer();
            set({
              status: 'error',
              phase,
              error: normalizeSyncError(message, 'SYNC_WRITE_FAILED').code,
            });
            // 5 秒后自动恢复到 idle
            resetTimer = setTimeout(() => {
              resetTimer = null;
              if (generation !== listenerGeneration) return;
              const s = get();
              if (s.status === 'error') {
                set({ status: 'idle', phase: null, progress: null, error: null });
              }
            }, 5000);
            break;

          case 'auto_sync':
            // auto_sync 阶段不改变状态，仅更新进度信息
            set({
              phase,
              progress: current != null && total != null ? { current, total } : null,
            });
            // 如果当前是 idle，设置为 syncing
            if (get().status === 'idle') {
              set({ status: 'syncing' });
            }
            break;
        }
      });

      set({ _unlistenPromise: pending });
      pending
        .then((unlistenFn) => {
          // StrictMode 或退出登录可先停止再重新注册；旧 Promise 不能覆盖新句柄。
          if (get()._unlistenPromise !== pending) {
            unlistenFn();
            return;
          }
          set({ _unlisten: unlistenFn, _unlistenPromise: null });
        })
        .catch((err) => {
          logger.error(
            '[safSyncStore] Failed to register sync-progress listener:',
            backendErrorLogDetails(err),
          );
          if (get()._unlistenPromise === pending) set({ _unlistenPromise: null });
        });
    },

    stopListening: () => {
      listenerGeneration++;
      clearResetTimer();
      const state = get();
      state._unlisten?.();
      // 清空句柄使待完成的注册在上方 then 中自行释放，避免重复退订。
      set({
        _unlisten: null,
        _unlistenPromise: null,
        status: 'idle',
        phase: null,
        progress: null,
        error: null,
      });
    },

    reset: () => {
      clearResetTimer();
      set({ status: 'idle', phase: null, progress: null, error: null, lastSyncedAt: null });
    },
  };
});
