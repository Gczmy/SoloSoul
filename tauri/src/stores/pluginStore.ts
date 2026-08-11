import { create } from 'zustand';
import {
  ConsentRequestEvent,
  DialogRequestEvent,
  MarketPluginInfo,
  PluginManifest,
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

// P031: 仅本文件使用，取消导出（死导出）
const DEFAULT_ENABLED_TIERS: PluginTier[] = ['p0', 'p1', 'p2'];

interface PluginState {
  marketPlugins: MarketPluginInfo[];
  installedPlugins: PluginManifest[];
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
  clearError: () => void;
  refreshRegistry: () => Promise<void>;
}

export const usePluginStore = create<PluginState>()((set, get) => ({
  marketPlugins: [],
  installedPlugins: [],
  runningPlugins: {},
  selectedTier: 'all',
  enabledTiers: DEFAULT_ENABLED_TIERS,
  isLoadingMarket: false,
  isLoadingInstalled: false,
  error: null,

  loadMarket: async () => {
    set({ isLoadingMarket: true, error: null });
    try {
      const list = await pluginCommands.listAll();
      set({ marketPlugins: list, isLoadingMarket: false });
    } catch (err) {
      set({ error: String(err), isLoadingMarket: false });
    }
  },

  setSelectedTier: (tier) => {
    set({ selectedTier: tier });
  },

  clearError: () => {
    set({ error: null });
  },

  loadInstalled: async () => {
    set({ isLoadingInstalled: true, error: null });
    try {
      const list = await pluginCommands.listInstalled();
      set({ installedPlugins: list, isLoadingInstalled: false });
    } catch (err) {
      set({ error: String(err), isLoadingInstalled: false });
    }
  },

  installPlugin: async (pluginId: string, version: string) => {
    try {
      await pluginCommands.install(pluginId, version);
      await get().loadMarket();
      await get().loadInstalled();
      // 触发模板重载，使 seed 模板的 contract_bindings 迁移结果即时反映在 UI
      useTemplateStore
        .getState()
        .loadTemplates()
        .catch((err) => logger.warn('[pluginStore] installPlugin: template reload failed:', err));
    } catch (err) {
      set({ error: String(err) });
    }
  },

  updatePlugin: async (pluginId: string) => {
    try {
      await pluginCommands.update(pluginId);
      await get().loadMarket();
      await get().loadInstalled();
      // 更新可能带来新的合同/role，同样触发模板重载
      useTemplateStore
        .getState()
        .loadTemplates()
        .catch((err) => logger.warn('[pluginStore] updatePlugin: template reload failed:', err));
    } catch (err) {
      set({ error: String(err) });
    }
  },

  uninstallPlugin: async (pluginId: string) => {
    try {
      await pluginCommands.uninstall(pluginId);
      await get().loadMarket();
      await get().loadInstalled();
    } catch (err) {
      set({ error: String(err) });
    }
  },

  runPlugin: async (pluginId: string, pluginName: string, params?: Record<string, string>) => {
    // 注入当前 UI locale，供插件国际化使用
    const mergedParams: Record<string, string> = {
      locale: i18next.language || 'en',
      ...params,
    };
    const startTime = Date.now();
    const running: RunningPlugin = {
      pluginId,
      pluginName,
      startTime,
      logs: [],
      results: [],
      consentRequests: [],
      dialogRequests: [],
      completed: false,
    };
    set((state) => ({
      runningPlugins: { ...state.runningPlugins, [pluginId]: running },
    }));

    try {
      const result = await pluginCommands.run(pluginId, mergedParams, (event) => {
        set((state) => {
          const next = { ...state.runningPlugins[pluginId] };
          if (!next) return state;
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
          return { runningPlugins: { ...state.runningPlugins, [pluginId]: next } };
        });
      });

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
      set((state) => {
        const next = { ...state.runningPlugins[pluginId], completed: true, toastShown: true };
        next.exitCode = result.exitCode;
        // 结果已通过事件通道实时累积，此处不重复添加。
        // 事件通道 + 最终 result.results 是同一份数据，再加会重复。
        return { runningPlugins: { ...state.runningPlugins, [pluginId]: next } };
      });
    } catch (err) {
      set((state) => {
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
      set((state) => {
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
    set((state) => {
      const next = { ...state.runningPlugins[pluginId], completed: true, toastShown: true };
      return { runningPlugins: { ...state.runningPlugins, [pluginId]: next } };
    });
  },

  clearPluginOutput: (pluginId: string) => {
    set((state) => ({
      runningPlugins: Object.fromEntries(
        Object.entries(state.runningPlugins).filter(([id]) => id !== pluginId),
      ),
    }));
  },

  resolveDialog: async (pluginId: string, requestId: string, value?: string) => {
    try {
      await pluginCommands.dialogResponse(requestId, value);
    } catch (err) {
      set({ error: String(err) });
    }
    set((state) => {
      const next = { ...state.runningPlugins[pluginId] };
      if (!next) return state;
      next.dialogRequests = next.dialogRequests.filter((r) => r.requestId !== requestId);
      return { runningPlugins: { ...state.runningPlugins, [pluginId]: next } };
    });
  },

  refreshRegistry: async () => {
    set({ isLoadingMarket: true, error: null });
    try {
      await pluginCommands.updateRegistry();
      await get().loadMarket();
      await get().loadInstalled();
      useUiStore.getState().showToast({
        type: 'success',
        message: i18next.t('plugin:refresh_success', { defaultValue: 'Plugin registry updated' }),
        duration: 3000,
      });
    } catch (err) {
      set({ error: String(err), isLoadingMarket: false });
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
