import { invokeTypedCommand } from '@/lib/typedIpc';
import { Channel, Resource } from '@tauri-apps/api/core';
import type { ContractRoleBinding } from '@/types/template';
import type {
  MarketPluginInfo,
  PluginManifest,
  PluginEvent,
  PluginResult,
  PluginSession,
  PluginAuditEntry,
  PluginInstallResult,
  PluginInstallProgress,
  PluginTier,
} from '@/lib/generated/ipcContracts';

export type {
  MarketPluginInfo,
  RegistryEntry,
  RegistryVersion,
  PluginManifest,
  PluginParam,
  PluginParamOption,
  PluginParamType,
  PluginContractRole,
  PluginContractBinding,
  PluginFieldBinding,
  PluginNetworkPolicy,
  PluginEvent,
  PluginLogLine,
  PluginResult,
  PluginResultPayload,
  PluginSession,
  PluginAuditEntry,
  PluginAuditAction,
  PluginInstallResult,
  PluginInstallProgress,
  PluginInstallPhase,
  PluginTier,
} from '@/lib/generated/ipcContracts';

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

type InstallRequest =
  | { command: 'plugin_install'; pluginId: string; version: string }
  | { command: 'plugin_update'; pluginId: string };

async function installWithCancellation(
  request: InstallRequest,
  signal?: AbortSignal,
  onProgress?: (progress: PluginInstallProgress) => void,
): Promise<PluginInstallResult> {
  if (signal?.aborted) throw new DOMException('Cancelled', 'AbortError');
  const operation = new Resource(await invokeTypedCommand('create_plugin_install'));
  const progress = new Channel<PluginInstallProgress>();
  let active = true;
  progress.onmessage = (event) => {
    if (active && !signal?.aborted) onProgress?.(event);
  };
  let closing: Promise<void> | undefined;
  const cancel = () => {
    closing ??= operation.close().catch(() => {});
  };
  signal?.addEventListener('abort', cancel, { once: true });
  try {
    if (signal?.aborted) throw new DOMException('Cancelled', 'AbortError');
    if (request.command === 'plugin_install') {
      return await invokeTypedCommand('plugin_install', {
        pluginId: request.pluginId,
        version: request.version,
        operationId: operation.rid,
        onProgress: progress,
      });
    }
    return await invokeTypedCommand('plugin_update', {
      pluginId: request.pluginId,
      operationId: operation.rid,
      onProgress: progress,
    });
  } finally {
    active = false;
    signal?.removeEventListener('abort', cancel);
    await (closing ?? operation.close()).catch(() => {});
  }
}

export const pluginCommands = {
  async listAll(tier?: PluginTier): Promise<MarketPluginInfo[]> {
    return invokeTypedCommand('plugin_list_all', { tier });
  },

  async listInstalled(): Promise<PluginManifest[]> {
    return invokeTypedCommand('plugin_list_installed');
  },

  async install(
    pluginId: string,
    version: string,
    signal?: AbortSignal,
    onProgress?: (progress: PluginInstallProgress) => void,
  ): Promise<PluginInstallResult> {
    return installWithCancellation(
      { command: 'plugin_install', pluginId, version },
      signal,
      onProgress,
    );
  },

  async update(
    pluginId: string,
    signal?: AbortSignal,
    onProgress?: (progress: PluginInstallProgress) => void,
  ): Promise<PluginInstallResult> {
    return installWithCancellation({ command: 'plugin_update', pluginId }, signal, onProgress);
  },

  async uninstall(pluginId: string): Promise<void> {
    await invokeTypedCommand('plugin_uninstall', { pluginId });
  },

  async run(
    pluginId: string,
    params: Record<string, string>,
    onEvent: (event: PluginEvent) => void,
    requestIsCurrent?: () => boolean,
  ): Promise<PluginResult> {
    const channel = new Channel<PluginEvent>();
    channel.onmessage = (event) => {
      if (!requestIsCurrent || requestIsCurrent()) onEvent(event);
    };
    return invokeTypedCommand('plugin_run', { pluginId, params, channel }, { requestIsCurrent });
  },

  async consentResponse(requestId: string, approved: boolean, value?: string): Promise<void> {
    await invokeTypedCommand('plugin_consent_response', { requestId, approved, value });
  },

  async dialogResponse(requestId: string, value?: string): Promise<void> {
    await invokeTypedCommand('plugin_dialog_response', { requestId, value });
  },

  async listSessions(): Promise<PluginSession[]> {
    return invokeTypedCommand('plugin_list_sessions');
  },

  async auditLog(limit?: number): Promise<PluginAuditEntry[]> {
    return invokeTypedCommand('plugin_audit_log', { limit });
  },

  async updateRegistry(): Promise<void> {
    await invokeTypedCommand('plugin_update_registry');
  },
};
