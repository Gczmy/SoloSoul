import { createStore } from 'zustand/vanilla';
import { useStore } from 'zustand';
import { invokeTypedCommand } from './typedIpc';
import type { PlatformCapabilities, PlatformCapability } from './generated/ipcContracts';

export type { PlatformCapabilities, PlatformCapability } from './generated/ipcContracts';

const disabled = (reason: string): PlatformCapability => ({
  status: 'unavailable',
  reason,
  implementation: null,
});

/** 启动尚未读取、旧客户端缺命令或桥接失败时均不允许误发更新/扫描。 */
export function unavailableCapabilities(reason: string): PlatformCapabilities {
  return {
    os: 'unknown',
    updateMethod: 'none',
    update: disabled(reason),
    ocr: disabled(reason),
    nativeMaterial: disabled(reason),
    biometric: disabled(reason),
    fileOpen: disabled(reason),
  };
}

function isCapability(value: unknown): value is PlatformCapability {
  if (!value || typeof value !== 'object') return false;
  const c = value as Partial<PlatformCapability>;
  const hasReason = typeof c.reason === 'string' && c.reason.length > 0;
  const hasImplementation = typeof c.implementation === 'string' && c.implementation.length > 0;
  if (c.status === 'supported') return c.reason === null && hasImplementation;
  if (c.status === 'unsupported') return hasReason && c.implementation === null;
  if (c.status === 'unavailable') return hasReason && hasImplementation;
  return false;
}

/** wire 校验只验证契约，不从 OS 推导能力；未来未知平台一律保守关闭。 */
export function readPlatformCapabilities(value: unknown): PlatformCapabilities {
  if (!value || typeof value !== 'object') return unavailableCapabilities('capabilities_invalid');
  const c = value as Partial<PlatformCapabilities>;
  if (
    typeof c.os !== 'string' ||
    !['none', 'tauri', 'android_apk'].includes(c.updateMethod ?? '') ||
    ![c.update, c.ocr, c.nativeMaterial, c.biometric, c.fileOpen].every(isCapability)
  ) {
    return unavailableCapabilities('capabilities_invalid');
  }
  if (!['macos', 'windows', 'linux', 'android', 'ios'].includes(c.os)) {
    return unavailableCapabilities('unknown_platform');
  }
  if (c.updateMethod === 'none' && c.update?.status === 'supported') {
    return unavailableCapabilities('capabilities_invalid');
  }
  return c as PlatformCapabilities;
}

interface PlatformCapabilityState {
  capabilities: PlatformCapabilities;
  loaded: boolean;
  pending: Promise<PlatformCapabilities> | null;
  load: (refresh?: boolean) => Promise<PlatformCapabilities>;
}

/** 单次在途读取与跨页共享快照；失败不缓存为已加载，下次操作可重新探测。 */
export function createPlatformCapabilityStore(request: () => Promise<unknown>) {
  return createStore<PlatformCapabilityState>((set, get) => ({
    capabilities: unavailableCapabilities('capabilities_not_loaded'),
    loaded: false,
    pending: null,
    load: (refresh = false) => {
      const state = get();
      if (state.pending) return state.pending;
      if (state.loaded && !refresh) return Promise.resolve(state.capabilities);
      const task = Promise.resolve()
        .then(request)
        .then((reply) => {
          const capabilities = readPlatformCapabilities(reply);
          set({ capabilities, loaded: capabilities.os !== 'unknown' });
          return capabilities;
        })
        .catch(() => {
          const capabilities = unavailableCapabilities('capabilities_read_failed');
          set({ capabilities, loaded: false });
          return capabilities;
        })
        .finally(() => {
          if (get().pending === task) set({ pending: null });
        });
      set({ pending: task });
      return task;
    },
  }));
}

export const platformCapabilityStore = createPlatformCapabilityStore(() =>
  invokeTypedCommand('get_platform_capabilities'),
);
export const getPlatformCapabilities = (refresh = false) =>
  platformCapabilityStore.getState().load(refresh);
export const getPlatformCapabilitiesSync = () => platformCapabilityStore.getState().capabilities;
export const usePlatformCapabilities = () =>
  useStore(platformCapabilityStore, (state) => state.capabilities);
