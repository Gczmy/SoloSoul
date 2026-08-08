import { invokeCommand as invoke } from '@/lib/ipcClient';
import { Channel } from '@tauri-apps/api/core';
import type { ContractRoleBinding } from '@/types/template';

interface RegistryEntry {
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

type PluginParamType = 'string' | 'number' | 'boolean' | 'select';

interface PluginParamOption {
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

interface PluginContractRole {
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

interface PluginLogLine {
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

interface WatermarkResultPayload {
  type: 'watermark_result';
  outputDir: string;
  items: WatermarkResultItem[];
}

interface ExpiryGuardianItem {
  objectId: string;
  objectName: string;
  kind: string;
  expiryDate: string;
  daysRemaining: number;
  urgency: 'expired' | 'critical' | 'warning' | 'notice' | 'safe';
}

interface ExpiryGuardianSummary {
  total: number;
  expired: number;
  critical: number;
  warning: number;
  notice: number;
  safe: number;
}

interface ExpiryGuardianPayload {
  type: 'expiry_guardian';
  title: string;
  locale: string;
  items: ExpiryGuardianItem[];
  summary: ExpiryGuardianSummary;
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
  | WatermarkResultPayload
  | ExpiryGuardianPayload;

interface PluginResult {
  exitCode: number;
  logs: PluginLogLine[];
  results: PluginResultPayload[];
  fuelConsumed: number;
}

type DialogType = 'alert' | 'confirm' | 'radio_list' | 'checkbox_list' | 'input';

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

interface PluginEvent {
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

interface PluginSessionInfo {
  id: string;
  pluginId: string;
  createdAt: string;
  expiresAt: string;
}

interface PluginAuditEntry {
  timestamp: string;
  pluginId: string;
  sessionId?: string;
  action: PluginAuditAction;
}

type PluginAuditAction =
  | { action: 'plugin_installed'; version: string }
  | { action: 'plugin_uninstalled' }
  | { action: 'plugin_run_started' }
  | { action: 'plugin_run_completed'; exitCode: number }
  | { action: 'plugin_run_failed'; reason: string }
  | { action: 'consent_approved'; fieldId: string }
  | { action: 'consent_denied'; fieldId: string };

interface PluginInstallResult {
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

/** 官方水印插件 ID——运行入口与卡片展示共用，避免在通用面板硬编码漂移。 */
export const WATERMARK_PLUGIN_ID = 'com.solosoul.official.watermark';

/**
 * 水印插件运行前置校验：已配置 `selectedAttachments` 但未选择任何附件（空数组/非法值）时返回 false。
 * 未配置（默认全部附件）、解析失败（沿用原行为继续运行）均视为通过。
 */
export function hasUsableWatermarkSelection(
  savedParams: Record<string, string> | undefined,
): boolean {
  const selectedRaw = savedParams?.selectedAttachments;
  if (!selectedRaw) return true;
  try {
    const selected = JSON.parse(selectedRaw);
    return Array.isArray(selected) && selected.length > 0;
  } catch {
    return true; // 解析失败 → 按未配置处理，继续运行
  }
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
