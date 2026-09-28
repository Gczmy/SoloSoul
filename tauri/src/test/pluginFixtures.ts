/** 仅测试使用：默认值遵循真实 Rust Serialize 输出，覆盖参数不会放宽 wire 类型。 */
import type {
  MarketPluginInfo,
  PluginEvent,
  PluginManifest,
  PluginParam,
  PluginResult,
  RegistryEntry,
} from '@/lib/generated/ipcContracts';

export function pluginParam(overrides: Partial<PluginParam> = {}): PluginParam {
  return {
    id: 'synthetic-param',
    label: 'Synthetic parameter',
    type: 'string',
    required: false,
    description: '',
    defaultValue: null,
    options: [],
    ...overrides,
  };
}

export function pluginManifest(overrides: Partial<PluginManifest> = {}): PluginManifest {
  return {
    id: 'synthetic-plugin',
    name: 'Synthetic Plugin',
    version: '1.0.0',
    description: 'synthetic',
    author: null,
    homepage: null,
    requiredCoreVersion: null,
    wasmHashSha256: null,
    permissions: [],
    dataTtlSeconds: 300,
    networkPolicy: { blockAllOutbound: true, allowedDomains: [] },
    requireUserConfirmation: true,
    tier: 'p3',
    category: 'test',
    params: [],
    contracts: [],
    fieldBindings: [],
    ...overrides,
  };
}

export function registryEntry(overrides: Partial<RegistryEntry> = {}): RegistryEntry {
  return {
    name: 'Synthetic Plugin',
    author: null,
    latestVersion: '1.0.0',
    versions: {},
    description: 'synthetic',
    homepage: null,
    i18n: null,
    tier: 'p3',
    category: 'test',
    params: [],
    contracts: [],
    fieldBindings: [],
    ...overrides,
  };
}

export function marketPlugin(overrides: Partial<MarketPluginInfo> = {}): MarketPluginInfo {
  return {
    pluginId: 'synthetic-plugin',
    installedVersion: null,
    hasUpdate: false,
    isCompatible: true,
    tier: 'p3',
    category: 'test',
    registryEntry: registryEntry(),
    ...overrides,
  };
}

export function pluginEvent(overrides: Partial<PluginEvent> = {}): PluginEvent {
  return {
    eventType: 'log',
    jsonData: '{}',
    customType: null,
    requestId: null,
    pluginId: null,
    pluginName: null,
    fieldId: null,
    fieldLabel: null,
    sensitivityLevel: null,
    ...overrides,
  };
}

export function pluginResult(overrides: Partial<PluginResult> = {}): PluginResult {
  return { exitCode: 0, logs: [], results: [], fuelConsumed: 0, ...overrides };
}
