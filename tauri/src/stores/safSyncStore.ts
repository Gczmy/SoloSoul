import { create } from 'zustand';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { logger } from '@/lib/logger';

/** 同步进度事件 payload（与 Rust 侧 `emit("sync-progress", ...)` 一致）。 */
export interface SyncProgressPayload {
  phase: 'sync_start' | 'sync_complete' | 'error' | 'sync_to_remote' | 'sync_from_remote' | 'migrate' | 'auto_sync';
  current?: number;
  total?: number;
  message?: string;
}

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

export const useSafSyncStore = create<SafSyncState>((set, get) => ({
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

    const pending = listen<SyncProgressPayload>('sync-progress', (event) => {
      const { phase, current, total, message } = event.payload;

      switch (phase) {
        case 'sync_start':
        case 'sync_to_remote':
        case 'sync_from_remote':
        case 'migrate':
          set({
            status: 'syncing',
            phase,
            progress: current != null && total != null ? { current, total } : null,
            error: null,
          });
          break;

        case 'sync_complete':
          set({
            status: 'completed',
            phase,
            progress: current != null && total != null ? { current, total } : null,
            lastSyncedAt: Date.now(),
            error: null,
          });
          // 3 秒后自动恢复到 idle
          setTimeout(() => {
            const s = get();
            if (s.status === 'completed') {
              set({ status: 'idle', phase: null, progress: null });
            }
          }, 3000);
          break;

        case 'error':
          set({
            status: 'error',
            phase,
            error: message ?? '同步失败',
          });
          // 5 秒后自动恢复到 idle
          setTimeout(() => {
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
        set({ _unlisten: unlistenFn, _unlistenPromise: null });
      })
      .catch((err) => {
        logger.error('[safSyncStore] Failed to register sync-progress listener:', err);
        set({ _unlistenPromise: null });
      });
  },

  stopListening: () => {
    const state = get();
    state._unlisten?.();
    if (state._unlistenPromise) {
      state._unlistenPromise
        .then((fn) => fn())
        .catch((err) => logger.warn('[safSyncStore] Failed to clean up listener:', err));
    }
    set({ _unlisten: null, _unlistenPromise: null, status: 'idle', phase: null, progress: null, error: null });
  },

  reset: () => {
    set({ status: 'idle', phase: null, progress: null, error: null, lastSyncedAt: null });
  },
}));
