import { invoke } from '@tauri-apps/api/core';
import { Channel } from '@tauri-apps/api/core';
import type { ContractRoleBinding } from '@/types/template';

export interface RegistryEntry {
  id: string;
  name: string;
  author: string;
  description: string;
  latestVersion: string;
  minCoreVersion: string;
  wasmHashSha256: string;
  permissions: string[];
  categories: string[];
  params: PluginParam[];
  i18n?: Record<string, { name: string; description: string }>;
  customUi?: string;
}

export type PluginTier = 'p0' | 'p1' | 'p2' | 'p3' | 'p4';

export type PluginParamType = 'string' | 'number' | 'boolean' | 'select';

export interface PluginParamOption {
  value: string;
  label: string;
}

export interface PluginParam {
  id: string;
  label: string;
  type: PluginParamType;
  required: boolean;
  description?: string;
  defaultValue?: string;
  options?: PluginParamOption[];
}

export interface MarketPluginInfo {
  pluginId: string;
  installedVersion?: string;
  hasUpdate: boolean;
  isCompatible: boolean;
  tier: PluginTier;
  category: string;
  registryEntry: RegistryEntry;
}

export interface PluginContractRole {
  roleId: string;
  label?: string;
  required?: boolean;
  defaultPropertyId?: string;
}

export interface PluginContractBinding {
  typeId: string;
  version: number;
  displayName?: string;
  strictContractGate: boolean;
  typeIdAliases: string[];
  roles: PluginContractRole[];
}

export interface PluginManifest {
  id: string;
  name: string;
  version: string;
  description: string;
  author: string;
  homepage?: string;
  permissions: string[];
  requiredCoreVersion: string;
  wasmHashSha256: string;
  dataTtlSeconds: number;
  tier: PluginTier;
  category: string;
  params: PluginParam[];
  /** 插件声明的合约列表（V2 新增字段，可为空/缺省） */
  contracts?: PluginContractBinding[];
  /** 插件国际化文本。key 为 locale，value 为 { name, description, ... } */
  i18n?: Record<string, Record<string, string>>;
  customUi?: string;
}

export interface PluginLogLine {
  id: string;
  level: 'debug' | 'info' | 'warn' | 'error';
  message: string;
  timestamp: number;
}

export interface WatermarkResultItem {
  objectId: string;
  attachmentId: string;
  fileName: string;
  mimeType: string;
  outputPath: string;
}

export interface WatermarkResultPayload {
  type: 'watermark_result';
  outputDir: string;
  items: WatermarkResultItem[];
}

export type PluginResultPayload =
  | { type: 'text'; content: string }
  | {
      type: 'key_value';
      title: string;
      pairs: Array<{ key: string; value: string; tag?: string; tagCode?: string }>;
    }
  | { type: 'table'; headers: string[]; rows: string[][] }
  | { type: 'markdown'; content: string }
  | WatermarkResultPayload;

export interface PluginResult {
  exitCode: number;
  logs: PluginLogLine[];
  results: PluginResultPayload[];
  fuelConsumed: number;
}

export type DialogType = 'alert' | 'confirm' | 'radio_list' | 'checkbox_list' | 'input';

export interface DialogConfig {
  type: DialogType;
  title?: string;
  message?: string;
  items?: Array<{ id: string; label: string }>;
  defaultValue?: string;
  placeholder?: string;
}

export interface DialogRequestEvent {
  eventType: 'dialog_request';
  requestId: string;
  pluginId: string;
  pluginName: string;
  jsonData: string;
}

export interface ConsentRequestEvent {
  eventType: 'consent_request';
  requestId: string;
  pluginId: string;
  pluginName: string;
  fieldId: string;
  fieldLabel: string;
  sensitivityLevel: string;
}

export interface PluginEvent {
  eventType:
    | 'log'
    | 'result'
    | 'consent_request'
    | 'dialog_request'
    | 'completed'
    | 'error'
    | 'custom_event';
  jsonData: string;
  customType?: string;
  requestId?: string;
  pluginId?: string;
  pluginName?: string;
  fieldId?: string;
  fieldLabel?: string;
  sensitivityLevel?: string;
}

export interface PluginSessionInfo {
  id: string;
  pluginId: string;
  createdAt: string;
  expiresAt: string;
}

export interface PluginAuditEntry {
  timestamp: string;
  pluginId: string;
  sessionId?: string;
  action: PluginAuditAction;
}

export type PluginAuditAction =
  | { action: 'plugin_installed'; version: string }
  | { action: 'plugin_uninstalled' }
  | { action: 'plugin_run_started' }
  | { action: 'plugin_run_completed'; exitCode: number }
  | { action: 'plugin_run_failed'; reason: string }
  | { action: 'consent_approved'; fieldId: string }
  | { action: 'consent_denied'; fieldId: string };

export interface PluginInstallResult {
  pluginId: string;
  version: string;
}

/**
 * 运行时推导插件契约角色绑定。
 * 当字段有 contractField: true 但无硬编码 contractBindings 时，
 * 从已安装插件 manifest 的 contracts[].roles[].defaultPropertyId 自动匹配。
 */
export function deriveContractBindings(
  contractTypeId: string | undefined,
  propertyId: string,
  installedPlugins: PluginManifest[],
): ContractRoleBinding[] {
  if (!contractTypeId) return [];

  for (const plugin of installedPlugins) {
    for (const contract of plugin.contracts || []) {
      if (contract.typeId !== contractTypeId) continue;
      for (const role of contract.roles || []) {
        if (role.defaultPropertyId === propertyId) {
          return [{ contractTypeId, roleId: role.roleId }];
        }
      }
    }
  }
  return [];
}

/** 根据当前 locale 解析插件国际化名称；若无匹配则返回插件默认 name。 */
export function resolvePluginName(
  plugin: Pick<PluginManifest, 'name' | 'i18n'>,
  locale: string,
): string {
  const map = plugin.i18n;
  if (!map) return plugin.name;
  const exact = map[locale]?.name;
  if (exact) return exact;
  const lang = locale.split('-')[0];
  const langMatch = map[lang]?.name;
  if (langMatch) return langMatch;
  const en = map['en-US']?.name ?? map['en']?.name;
  if (en) return en;
  return plugin.name;
}

export const pluginCommands = {
  async listAll(tier?: PluginTier): Promise<MarketPluginInfo[]> {
    return invoke('plugin_list_all', { tier });
  },

  async listInstalled(): Promise<PluginManifest[]> {
    return invoke('plugin_list_installed');
  },

  async install(pluginId: string, version: string): Promise<PluginInstallResult> {
    return invoke('plugin_install', { pluginId, version });
  },

  async update(pluginId: string): Promise<PluginInstallResult> {
    return invoke('plugin_update', { pluginId });
  },

  async uninstall(pluginId: string): Promise<void> {
    return invoke('plugin_uninstall', { pluginId });
  },

  async run(
    pluginId: string,
    params: Record<string, string>,
    onEvent: (event: PluginEvent) => void,
  ): Promise<PluginResult> {
    const channel = new Channel<PluginEvent>();
    channel.onmessage = onEvent;
    return invoke('plugin_run', { pluginId, params, channel });
  },

  async consentResponse(requestId: string, approved: boolean, value?: string): Promise<void> {
    return invoke('plugin_consent_response', { requestId, approved, value });
  },

  async dialogResponse(requestId: string, value?: string): Promise<void> {
    return invoke('plugin_dialog_response', { requestId, value });
  },

  async listSessions(): Promise<PluginSessionInfo[]> {
    return invoke('plugin_list_sessions');
  },

  async auditLog(limit?: number): Promise<PluginAuditEntry[]> {
    return invoke('plugin_audit_log', { limit });
  },

  async updateRegistry(): Promise<void> {
    return invoke('plugin_update_registry');
  },
};
