import { create } from 'zustand';
import { persist, createJSONStorage } from 'zustand/middleware';
import {
  ConsentRequestEvent,
  DialogRequestEvent,
  MarketPluginInfo,
  PluginManifest,
  PluginResultPayload,
  PluginTier,
  pluginCommands,
} from '@/lib/plugin';

export interface PluginLogLine {
  id: string;
  level: 'debug' | 'info' | 'warn' | 'error';
  message: string;
  timestamp: number;
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
}

export const DEFAULT_ENABLED_TIERS: PluginTier[] = ['p0', 'p1', 'p2'];

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
  runPlugin: (pluginId: string, pluginName: string, params?: Record<string, string>) => Promise<void>;
  stopPlugin: (pluginId: string) => void;
  clearPluginOutput: (pluginId: string) => void;
  resolveDialog: (pluginId: string, requestId: string, value?: string) => Promise<void>;
  clearError: () => void;
}

export const usePluginStore = create<PluginState>()(
  persist(
    (set, get) => ({
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
        } catch (err) {
          set({ error: String(err) });
        }
      },

      updatePlugin: async (pluginId: string) => {
        try {
          await pluginCommands.update(pluginId);
          await get().loadMarket();
          await get().loadInstalled();
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
          const result = await pluginCommands.run(pluginId, params ?? {}, (event) => {
            set((state) => {
              const next = { ...state.runningPlugins[pluginId] };
              if (!next) return state;
              switch (event.eventType) {
                case 'log':
                  try {
                    const log = JSON.parse(event.jsonData) as PluginLogLine;
                    next.logs = [...next.logs, log];
                  } catch {
                    // ignore malformed log
                  }
                  break;
                case 'result':
                  try {
                    const payload = JSON.parse(event.jsonData) as PluginResultPayload;
                    next.results = [...next.results, payload];
                  } catch {
                    // ignore malformed result
                  }
                  break;
                case 'consent_request':
                  next.consentRequests = [...next.consentRequests, event as ConsentRequestEvent];
                  break;
                case 'dialog_request':
                  next.dialogRequests = [...next.dialogRequests, event as DialogRequestEvent];
                  break;
                case 'completed':
                  next.completed = true;
                  try {
                    const completed = JSON.parse(event.jsonData) as { exitCode: number };
                    next.exitCode = completed.exitCode;
                  } catch {
                    // ignore
                  }
                  break;
                case 'error':
                  next.completed = true;
                  next.error = event.jsonData;
                  break;
              }
              return { runningPlugins: { ...state.runningPlugins, [pluginId]: next } };
            });
          });

          set((state) => {
            const next = { ...state.runningPlugins[pluginId], completed: true };
            next.exitCode = result.exitCode;
            next.results = [...next.results, ...result.results];
            return { runningPlugins: { ...state.runningPlugins, [pluginId]: next } };
          });
        } catch (err) {
          set((state) => {
            const next = { ...state.runningPlugins[pluginId], completed: true, error: String(err) };
            return { runningPlugins: { ...state.runningPlugins, [pluginId]: next } };
          });
        }
      },

      stopPlugin: (pluginId: string) => {
        set((state) => {
          const next = { ...state.runningPlugins[pluginId], completed: true };
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
    }),
    {
      name: 'solosoul-plugin-store',
      storage: createJSONStorage(() => localStorage),
      partialize: (state) => ({ runningPlugins: state.runningPlugins }),
    },
  ),
);
