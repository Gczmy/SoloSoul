/**
 * 预热调度（Prefetch Runtime，docs/prefetch-runtime-design.md）。
 *
 * 按阶段批量执行注册表的后台预热：'mount'（App 挂载，warmupPolicy='always'）
 * 与 'afterAuth'（登录/解锁完成，warmupPolicy='afterAuth'）。所有 warmup 均为
 * fire-and-forget 且吞错，失败由页面挂载兜底。
 *
 * requestIdleCallback 降级：iOS WKWebView 旧版本缺失时回退 setTimeout(200ms)
 * （沿用 P015 路由预取 FALLBACK_TICK 模式），保证移动端行为一致。
 */
import { prefetchRegistry } from './registry';
import { useTemplateStore } from '@/stores/templateStore';
import { useTrashStore } from '@/stores/trashStore';
import { useAuthStore } from '@/stores/authStore';

export type WarmupPhase = 'mount' | 'afterAuth';

const IDLE_FALLBACK_MS = 200;
const IDLE_TIMEOUT_MS = 2000;

function scheduleIdle(callback: () => void): void {
  const run = () => callback();
  if (typeof requestIdleCallback === 'function') {
    requestIdleCallback(run, { timeout: IDLE_TIMEOUT_MS });
  } else {
    setTimeout(run, IDLE_FALLBACK_MS);
  }
}

/**
 * 已 store 化数据的后台预热任务（templateStore/trashStore 自带缓存与变更刷新，
 * 仅需登录后提前填充，页面零改动）。返回 Promise 供调度吞错。
 */
export const prefetchWarmupTasks: Array<{ phase: WarmupPhase; run: () => Promise<unknown> }> = [
  {
    phase: 'afterAuth',
    run: () => useTemplateStore.getState().loadTemplates(),
  },
  {
    phase: 'afterAuth',
    run: () => {
      const account = useAuthStore.getState().currentAccount;
      return account ? useTrashStore.getState().loadItems(account.id) : Promise.resolve();
    },
  },
];

export function warmupPrefetchRegistry(phase: WarmupPhase): void {
  scheduleIdle(() => {
    for (const store of Object.values(prefetchRegistry)) {
      if (store.options.warmupPolicy === 'always' || store.options.warmupPolicy === phase) {
        store.warmup();
      }
    }
    for (const task of prefetchWarmupTasks) {
      if (task.phase === phase) {
        void task.run().catch(() => {});
      }
    }
  });
}
