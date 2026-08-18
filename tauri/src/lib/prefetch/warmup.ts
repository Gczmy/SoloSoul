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
import { usePluginStore } from '@/stores/pluginStore';
import { useSyncStore } from '@/stores/syncStore';

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
  {
    phase: 'afterAuth',
    run: async () => {
      // 插件市场与已装清单均为本地 IPC（plugin_list_all / plugin_list_installed，
      // 网络刷新是独立的 plugin_update_registry），登录后预热，插件页/模板页
      // 挂载时直接渲染（页面零改动）。
      const store = usePluginStore.getState();
      await Promise.all([store.loadMarket(), store.loadInstalled()]);
    },
  },
  {
    phase: 'afterAuth',
    run: async () => {
      // LAN 设备同步状态：AppShell/GlobalSyncIndicator 均不预载（仅事件监听），
      // 首次进入同步页时 store 全默认值，挂载 5 个 IPC 后才填充（状态卡先显示
      // 「禁用/空设备」再跳变）。登录后后台预载，进入同步页直接渲染真实状态。
      // 不注册 registry 缓存：连接/发现/冲突是实时性数据，syncStore 的 15s 轮询
      // + 事件 + 操作后刷新已保证新鲜，TTL 缓存反而引入陈旧窗口。
      const s = useSyncStore.getState();
      await Promise.all([
        s.loadStatus(),
        s.loadListenAddr(),
        s.loadAutoSyncStatus(),
        s.loadUiPrefsSync(),
        s.loadConflicts(),
      ]);
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

/**
 * 重置全部预取缓存（事件失效）：Vault 锁定/登出时调用，与 objectStore /
 * settingsStore 的 clearOnVaultLock 同语义——解密数据（日志/统计/备份列表等）
 * 不残留内存。store 自带缓存，页面重新进入时自动现场拉取兜底。
 */
export function resetPrefetchRegistry(): void {
  for (const store of Object.values(prefetchRegistry)) {
    store.reset();
  }
}
