/**
 * 统一 IPC 调用层（P131）
 *
 * 约定：所有前端 Rust 命令调用一律经 `invokeCommand` 走本层，
 * 禁止裸调 `@tauri-apps/api/core` 的 `invoke`。历史文件通过
 * `import { invokeCommand as invoke } from '@/lib/ipcClient'` 迁移，
 * 调用签名与裸 `invoke` 完全一致，仅统一收口。
 *
 * 本层职责：
 * 1. **统一失败日志** —— 任何命令失败都带命令名记 logger.warn，
 *    消灭「静默失败 / unhandled rejection」类问题（配合各组件 onError toast）。
 * 2. **可选解锁守卫** —— `opts.requireUnlocked` 为 true 时，Vault 未解锁
 *    直接抛出 `No account is currently unlocked`（与后端错误语义一致），
 *    动态 import authStore 避免循环依赖。默认关闭，需按命令显式启用。
 * 3. **错误消息透传** —— 不在此处翻译错误：展示层（useToastError /
 *    translateRustError / resolveBackendErrorMessage）负责翻译，避免
 *    双层翻译破坏既有 translateRustError 消费方。
 *
 * 注意：本模块不 import `@/lib/logger`（其链路 utils → i18n 与本模块被
 * i18n 依赖构成循环）——失败日志内联 dev 守卫，仅依赖 core。
 */
import { invoke } from '@tauri-apps/api/core';

export interface InvokeOptions {
  /** 为 true 时要求 Vault 已解锁（isAuthenticated），否则立即抛错。 */
  requireUnlocked?: boolean;
}

/** dev 守卫日志：与 logger.warn 同语义，但不引入 logger→utils→i18n 的循环依赖。 */
function devWarn(...args: unknown[]): void {
  if (
    import.meta.env.DEV === true ||
    import.meta.env.MODE === 'debug' ||
    import.meta.env.VITE_SOLOSOUL_DEBUG === 'true'
  ) {
    console.warn(...args);
  }
}

/**
 * 统一命令调用入口。签名与 Tauri `invoke` 兼容（args 可选）。
 */
export async function invokeCommand<T>(
  cmd: string,
  args?: Record<string, unknown>,
  opts?: InvokeOptions,
): Promise<T> {
  if (opts?.requireUnlocked) {
    // 动态 import 避免与 authStore 循环依赖
    const { useAuthStore } = await import('@/stores/authStore');
    if (!useAuthStore.getState().isAuthenticated) {
      const err = new Error('No account is currently unlocked');
      devWarn(`[ipc] '${cmd}' blocked: vault not unlocked`);
      throw err;
    }
  }
  try {
    // args 缺省时不再传第二参：既有测试断言 `toHaveBeenCalledWith('cmd')`
    // （单参精确匹配），多传 undefined 会破坏这些断言。
    if (args === undefined) {
      return await invoke<T>(cmd);
    }
    return await invoke<T>(cmd, args);
  } catch (err) {
    const raw = err instanceof Error ? err.message : String(err);
    devWarn(`[ipc] command '${cmd}' failed:`, raw);
    throw err;
  }
}
