import { createSessionRequests, onRequestSessionChange } from '@/lib/sessionRequests';
import { create } from 'zustand';
import {
  ConsentRequestEvent,
  DialogRequestEvent,
  MarketPluginInfo,
  PluginManifest,
  PluginInstallProgress,
  PluginResultPayload,
  PluginTier,
  pluginCommands,
} from '@/lib/plugin';
import { useUiStore } from '@/stores/uiStore';
import { useTemplateStore } from '@/stores/templateStore';
import { logger } from '@/lib/logger';
import i18next from '@/lib/i18n';

export interface PluginLogLine {
  id: string;
  level: 'debug' | 'info' | 'warn' | 'error';
  message: string;
  timestamp: number;
}

/** P215: 单插件运行日志上限——环形截断，避免不可变累积 `[...logs, x]` 的 O(n²) 拷贝与内存膨胀。 */
export const MAX_PLUGIN_LOGS = 200;

/** P215: 单插件结果上限。 */
export const MAX_PLUGIN_RESULTS = 50;

export function isPluginLogLine(value: unknown): value is PluginLogLine {
  if (typeof value !== 'object' || value === null) return false;
  const v = value as Record<string, unknown>;
  return (
    typeof v.id === 'string' &&
    typeof v.message === 'string' &&
    typeof v.timestamp === 'number' &&
    ['debug', 'info', 'warn', 'error'].includes(v.level as string)
  );
}

export function isPluginResultPayload(value: unknown): value is PluginResultPayload {
  if (typeof value !== 'object' || value === null) return false;
  const v = value as Record<string, unknown>;
  switch (v.type) {
    case 'text':
    case 'markdown':
      return typeof v.content === 'string';
    case 'key_value':
      return typeof v.title === 'string' && Array.isArray(v.pairs);
    case 'table':
      return Array.isArray(v.headers) && Array.isArray(v.rows);
    case 'watermark_result':
      return typeof v.outputDir === 'string' && Array.isArray(v.items);
    case 'expiry_guardian':
      return typeof v.title === 'string' && Array.isArray(v.items);
    default:
      return false;
  }
}

export function isConsentRequestEvent(event: unknown): event is ConsentRequestEvent {
  const e = event as Record<string, unknown>;
  return (
    e?.eventType === 'consent_request' && typeof (event as ConsentRequestEvent).fieldId === 'string'
  );
}

export function isDialogRequestEvent(event: unknown): event is DialogRequestEvent {
  const e = event as Record<string, unknown>;
  return (
    e?.eventType === 'dialog_request' && typeof (event as DialogRequestEvent).requestId === 'string'
  );
}

export function isPluginCompletedEvent(value: unknown): value is { exitCode: number } {
  if (typeof value !== 'object' || value === null) return false;
  const v = value as Record<string, unknown>;
  return typeof v.exitCode === 'number';
}

export interface RunningPlugin {
  runId?: string;
  pluginId: string;
  pluginName: string;
  startTime: number;
  logs: PluginLogLine[];
  results: PluginResultPayload[];
  consentRequests: ConsentRequestEvent[];
  dialogRequests: DialogRequestEvent[];
  completed: boolean;
  exitCode?: number;
  error?: string;
  /** 标记 runPlugin 已显示 Toast，供 PluginQuickNotificationListener 去重 */
  toastShown?: boolean;
}

/**
 * P021：插件运行事件 → 运行态对象的纯函数应用（自 runPlugin 的巨型 switch 拆出）。
 * 就地修改传入的 draft 并返回；log/result 按 P215 环形上限截断。
 */
/** 插件运行事件的最小形状（lib/plugin 的 PluginEvent 为内部类型，此处按需声明）。 */
type RunPluginEvent = { eventType: string; jsonData: string };

function applyPluginRunEvent(next: RunningPlugin, event: RunPluginEvent): RunningPlugin {
  switch (event.eventType) {
    case 'log':
      try {
        const parsed = JSON.parse(event.jsonData);
        if (isPluginLogLine(parsed)) {
          // P215: 环形截断到上限，每次 append 的拷贝成本从 O(当前长度) 收敛为 O(上限)。
          next.logs = [...next.logs, parsed].slice(-MAX_PLUGIN_LOGS);
        }
      } catch {
        // ignore malformed log
      }
      break;
    case 'result':
      try {
        const parsed = JSON.parse(event.jsonData);
        if (isPluginResultPayload(parsed)) {
          next.results = [...next.results, parsed].slice(-MAX_PLUGIN_RESULTS);
        }
      } catch {
        // ignore malformed result
      }
      break;
    case 'consent_request':
      if (isConsentRequestEvent(event)) {
        next.consentRequests = [...next.consentRequests, event];
      }
      break;
    case 'dialog_request':
      if (isDialogRequestEvent(event)) {
        next.dialogRequests = [...next.dialogRequests, event];
      }
      break;
    case 'completed':
      next.completed = true;
      next.toastShown = true;
      try {
        const parsed = JSON.parse(event.jsonData);
        if (isPluginCompletedEvent(parsed)) {
          next.exitCode = parsed.exitCode;
        }
      } catch {
        // ignore
      }
      break;
    case 'error':
      next.completed = true;
      next.toastShown = true;
      next.error = event.jsonData;
      break;
  }
  return next;
}

// P031: 仅本文件使用，取消导出（死导出）
const DEFAULT_ENABLED_TIERS: PluginTier[] = ['p0', 'p1', 'p2'];

interface PluginInstallTask {
  controller: AbortController;
  progress: PluginInstallProgress;
}

const INITIAL_INSTALL_PROGRESS: PluginInstallProgress = {
  percent: 0,
  phase: 'preparing',
  downloadedBytes: 0,
  totalBytes: null,
};

/** 同一安装任务跨页面共享进度；取消、锁定及旧任务迟到的事件不能覆盖新任务。 */
function installProgressUpdater(pluginId: string, controller: AbortController) {
  return (progress: PluginInstallProgress) => {
    usePluginStore.setState((state) => {
      const task = state.installingPlugins[pluginId];
      if (
        !task ||
        task.controller !== controller ||
        controller.signal.aborted ||
        !Number.isFinite(progress.percent)
      )
        return state;
      const percent = Math.min(100, Math.max(0, progress.percent));
      if (percent < task.progress.percent) return state;
      return {
        installingPlugins: {
          ...state.installingPlugins,
          [pluginId]: { ...task, progress: { ...progress, percent } },
        },
      };
    });
  };
}

interface PluginState {
  marketPlugins: MarketPluginInfo[];
  installedPlugins: PluginManifest[];
  installingPlugins: Record<string, PluginInstallTask>;
  cancelInstall: (pluginId: string) => void;
  runningPlugins: Record<string, RunningPlugin>;
  selectedTier: 'all' | PluginTier;
  enabledTiers: PluginTier[];
  isLoadingMarket: boolean;
  isLoadingInstalled: boolean;
  error: string | null;
  loadMarket: () => Promise<void>;
  loadInstalled: () => Promise<void>;
  setSelectedTier: (tier: 'all' | PluginTier) => void;
  installPlugin: (pluginId: string, version: string) => Promise<void>;
  updatePlugin: (pluginId: string) => Promise<void>;
  uninstallPlugin: (pluginId: string) => Promise<void>;
  runPlugin: (
    pluginId: string,
    pluginName: string,
    params?: Record<string, string>,
  ) => Promise<void>;
  stopPlugin: (pluginId: string) => void;
  clearPluginOutput: (pluginId: string) => void;
  resolveDialog: (pluginId: string, requestId: string, value?: string) => Promise<void>;
  clearOnVaultLock: () => void;
  clearError: () => void;
  refreshRegistry: () => Promise<void>;
}

const requests = createSessionRequests();

export const usePluginStore = create<PluginState>()((set, get) => ({
  marketPlugins: [],
  installedPlugins: [],
  installingPlugins: {},
  cancelInstall: (pluginId) => {
    get().installingPlugins[pluginId]?.controller.abort();
  },
  runningPlugins: {},
  selectedTier: 'all',
  enabledTiers: DEFAULT_ENABLED_TIERS,
  isLoadingMarket: false,
  isLoadingInstalled: false,
  error: null,

  loadMarket: async () => {
    const request = requests.begin('market');
    const setCurrent = request.guardSet<PluginState>(set);
    setCurrent({ isLoadingMarket: true, error: null });
    try {
      request.assertCurrent();
      const list = await pluginCommands.listAll();
      request.assertCurrent();
      setCurrent({ marketPlugins: list, isLoadingMarket: false });
    } catch (err) {
      if (!request.isCurrent()) return;
      setCurrent({ error: String(err), isLoadingMarket: false });
    }
  },

  setSelectedTier: (tier) => {
    set({ selectedTier: tier });
  },

  clearOnVaultLock: () => {
    Object.values(get().installingPlugins).forEach((task) => task.controller.abort());
    set({ installingPlugins: {} });
    requests.invalidate();
    set({ runningPlugins: {}, error: null, isLoadingMarket: false, isLoadingInstalled: false });
  },

  clearError: () => {
    set({ error: null });
  },

  loadInstalled: async () => {
    const request = requests.begin('installed');
    const setCurrent = request.guardSet<PluginState>(set);
    setCurrent({ isLoadingInstalled: true, error: null });
    try {
      request.assertCurrent();
      const list = await pluginCommands.listInstalled();
      request.assertCurrent();
      setCurrent({ installedPlugins: list, isLoadingInstalled: false });
    } catch (err) {
      if (!request.isCurrent()) return;
      setCurrent({ error: String(err), isLoadingInstalled: false });
    }
  },

  installPlugin: async (pluginId: string, version: string) => {
    if (get().installingPlugins[pluginId]) return;
    const controller = new AbortController();
    set((state) => ({
      installingPlugins: {
        ...state.installingPlugins,
        [pluginId]: { controller, progress: INITIAL_INSTALL_PROGRESS },
      },
      error: null,
    }));
    const request = requests.begin();
    const setCurrent = request.guardSet<PluginState>(set);
    try {
      request.assertCurrent();
      const onProgress = installProgressUpdater(pluginId, controller);
      await pluginCommands.install(pluginId, version, controller.signal, onProgress);
      request.assertCurrent();
      // 100% 来自命令成功；短暂保留满环反馈，同时刷新列表，不延长真实安装过程。
      onProgress({ ...INITIAL_INSTALL_PROGRESS, percent: 100, phase: 'completed' });
      const completedFeedback = new Promise<void>((resolve) => setTimeout(resolve, 250));
      await get().loadMarket();
      request.assertCurrent();
      await get().loadInstalled();
      request.assertCurrent();
      await completedFeedback;
      request.assertCurrent();
      // 触发模板重载，使 seed 模板的 contract_bindings 迁移结果即时反映在 UI
      useTemplateStore
        .getState()
        .loadTemplates()
        .catch((err) => logger.warn('[pluginStore] installPlugin: template reload failed:', err));
    } catch (err) {
      if (!request.isCurrent()) return;
      if (!controller.signal.aborted && !String(err).includes('PLUGIN_INSTALL_CANCELLED'))
        setCurrent({ error: String(err) });
    } finally {
      if (get().installingPlugins[pluginId]?.controller === controller)
        set((state) => {
          const installingPlugins = { ...state.installingPlugins };
          delete installingPlugins[pluginId];
          return { installingPlugins };
        });
    }
  },

  updatePlugin: async (pluginId: string) => {
    if (get().installingPlugins[pluginId]) return;
    const controller = new AbortController();
    set((state) => ({
      installingPlugins: {
        ...state.installingPlugins,
        [pluginId]: { controller, progress: INITIAL_INSTALL_PROGRESS },
      },
      error: null,
    }));
    const request = requests.begin();
    const setCurrent = request.guardSet<PluginState>(set);
    try {
      request.assertCurrent();
      const onProgress = installProgressUpdater(pluginId, controller);
      await pluginCommands.update(pluginId, controller.signal, onProgress);
      request.assertCurrent();
      onProgress({ ...INITIAL_INSTALL_PROGRESS, percent: 100, phase: 'completed' });
      const completedFeedback = new Promise<void>((resolve) => setTimeout(resolve, 250));
      await get().loadMarket();
      request.assertCurrent();
      await get().loadInstalled();
      request.assertCurrent();
      await completedFeedback;
      request.assertCurrent();
      // 更新可能带来新的合同/role，同样触发模板重载
      useTemplateStore
        .getState()
        .loadTemplates()
        .catch((err) => logger.warn('[pluginStore] updatePlugin: template reload failed:', err));
    } catch (err) {
      if (!request.isCurrent()) return;
      if (!controller.signal.aborted && !String(err).includes('PLUGIN_INSTALL_CANCELLED'))
        setCurrent({ error: String(err) });
    } finally {
      if (get().installingPlugins[pluginId]?.controller === controller)
        set((state) => {
          const installingPlugins = { ...state.installingPlugins };
          delete installingPlugins[pluginId];
          return { installingPlugins };
        });
    }
  },

  uninstallPlugin: async (pluginId: string) => {
    const request = requests.begin();
    const setCurrent = request.guardSet<PluginState>(set);
    try {
      request.assertCurrent();
      await pluginCommands.uninstall(pluginId);
      request.assertCurrent();
      await get().loadMarket();
      request.assertCurrent();
      await get().loadInstalled();
      request.assertCurrent();
    } catch (err) {
      if (!request.isCurrent()) return;
      setCurrent({ error: String(err) });
    }
  },

  runPlugin: async (pluginId: string, pluginName: string, params?: Record<string, string>) => {
    const request = requests.begin(`run:${pluginId}`);
    const setCurrent = request.guardSet<PluginState>(set);
    // 注入当前 UI locale，供插件国际化使用
    const mergedParams: Record<string, string> = {
      locale: i18next.language || 'en',
      ...params,
    };
    const startTime = Date.now();
    const running: RunningPlugin = {
      runId: crypto.randomUUID(),
      pluginId,
      pluginName,
      startTime,
      logs: [],
      results: [],
      consentRequests: [],
      dialogRequests: [],
      completed: false,
    };
    setCurrent((state) => ({
      runningPlugins: { ...state.runningPlugins, [pluginId]: running },
    }));

    try {
      // P021：事件分发收敛至模块级纯函数 applyPluginRunEvent
      request.assertCurrent();
      const result = await pluginCommands.run(
        pluginId,
        mergedParams,
        (event) => {
          setCurrent((state) => {
            const draft = state.runningPlugins[pluginId];
            if (!draft) return state;
            const next = applyPluginRunEvent({ ...draft }, event);
            return { runningPlugins: { ...state.runningPlugins, [pluginId]: next } };
          });
        },
        request.isCurrent,
      );
      request.assertCurrent();

      // 根据运行结果决定 Toast 类型（在同一个 set 中同时标记 completed + toastShown，
      // 避免 PluginQuickNotificationListener 在两次 set 之间误触发重复 Toast）
      const finalPlugin = get().runningPlugins[pluginId];
      const hasError = result.exitCode !== 0 || !!finalPlugin?.error;
      if (hasError) {
        useUiStore.getState().showToast({
          type: 'error',
          message: i18next.t('plugin:run_failed', {
            pluginName,
            defaultValue: `「${pluginName}」plugin run failed`,
          }),
          duration: 5000,
        });
      } else {
        useUiStore.getState().showToast({
          type: 'success',
          message: i18next.t('plugin:run_complete', {
            pluginName,
            defaultValue: `「${pluginName}」plugin run completed`,
          }),
          duration: 3000,
        });
      }
      setCurrent((state) => {
        const next = { ...state.runningPlugins[pluginId], completed: true, toastShown: true };
        next.exitCode = result.exitCode;
        // 结果已通过事件通道实时累积，此处不重复添加。
        // 事件通道 + 最终 result.results 是同一份数据，再加会重复。
        return { runningPlugins: { ...state.runningPlugins, [pluginId]: next } };
      });
    } catch (err) {
      if (!request.isCurrent()) return;
      setCurrent((state) => {
        const next = { ...state.runningPlugins[pluginId], completed: true, error: String(err) };
        return { runningPlugins: { ...state.runningPlugins, [pluginId]: next } };
      });
      useUiStore.getState().showToast({
        type: 'error',
        message: i18next.t('plugin:run_error', {
          pluginName,
          defaultValue: `「${pluginName}」plugin run error`,
        }),
        duration: 5000,
      });
      setCurrent((state) => {
        const next = {
          ...state.runningPlugins[pluginId],
          completed: true,
          toastShown: true,
          error: String(err),
        };
        return { runningPlugins: { ...state.runningPlugins, [pluginId]: next } };
      });
    }
  },

  stopPlugin: (pluginId: string) => {
    requests.invalidate(`run:${pluginId}`);
    set((state) => {
      const next = { ...state.runningPlugins[pluginId], completed: true, toastShown: true };
      return { runningPlugins: { ...state.runningPlugins, [pluginId]: next } };
    });
  },

  clearPluginOutput: (pluginId: string) => {
    requests.invalidate(`run:${pluginId}`);
    set((state) => ({
      runningPlugins: Object.fromEntries(
        Object.entries(state.runningPlugins).filter(([id]) => id !== pluginId),
      ),
    }));
  },

  resolveDialog: async (pluginId: string, requestId: string, value?: string) => {
    const request = requests.begin();
    const setCurrent = request.guardSet<PluginState>(set);
    const runId = get().runningPlugins[pluginId]?.runId;
    try {
      request.assertCurrent();
      await pluginCommands.dialogResponse(requestId, value);
      request.assertCurrent();
    } catch (err) {
      if (!request.isCurrent()) return;
      setCurrent({ error: String(err) });
    }
    setCurrent((state) => {
      const current = state.runningPlugins[pluginId];
      if (!current || current.runId !== runId) return state;
      const next = { ...current };
      next.dialogRequests = next.dialogRequests.filter((r) => r.requestId !== requestId);
      return { runningPlugins: { ...state.runningPlugins, [pluginId]: next } };
    });
  },

  refreshRegistry: async () => {
    const request = requests.begin();
    const setCurrent = request.guardSet<PluginState>(set);
    setCurrent({ isLoadingMarket: true, error: null });
    try {
      request.assertCurrent();
      await pluginCommands.updateRegistry();
      request.assertCurrent();
      await get().loadMarket();
      request.assertCurrent();
      await get().loadInstalled();
      request.assertCurrent();
      useUiStore.getState().showToast({
        type: 'success',
        message: i18next.t('plugin:refresh_success', { defaultValue: 'Plugin registry updated' }),
        duration: 3000,
      });
    } catch (err) {
      if (!request.isCurrent()) return;
      setCurrent({ error: String(err), isLoadingMarket: false });
      useUiStore.getState().showToast({
        type: 'error',
        message: i18next.t('plugin:refresh_failed', {
          defaultValue: 'Failed to refresh plugin registry',
        }),
        duration: 5000,
      });
    }
  },
}));

onRequestSessionChange(() => usePluginStore.getState().clearOnVaultLock());
