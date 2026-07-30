/**
 * SoloSoul JS SDK —— 客户端占位实现
 *
 * 该 SDK 封装与 Tauri Rust Commands 的通信，为外部前端提供类型安全的调用接口。
 * 当前版本为 P4 占位实现，实际功能依赖 Tauri v2 运行环境。
 */

export class SoloSoulError extends Error {
  constructor(
    message: string,
    public readonly code?: string,
  ) {
    super(message);
    this.name = 'SoloSoulError';
  }
}

export interface PluginInfo {
  id: string;
  name: string;
  version: string;
  description: string;
  tier: 'p0' | 'p1' | 'p2' | 'p3' | 'p4';
  installed: boolean;
}

export interface PluginRunResult {
  success: boolean;
  exitCode: number;
  results: unknown[];
  logs: { level: string; message: string; timestamp: number }[];
}

/**
 * 动态导入 @tauri-apps/api/core，避免非 Tauri 环境直接报错。
 */
async function getTauriInvoke(): Promise<
  <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>
> {
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke;
  } catch {
    throw new SoloSoulError(
      'SoloSoul JS SDK 必须在 Tauri Webview 或已注入 __TAURI__ 的环境中运行',
      'TAURI_NOT_FOUND',
    );
  }
}

/**
 * SoloSoul 客户端
 */
export class SoloSoulClient {
  /**
   * 解锁本地 Vault
   */
  async unlockVault(password: string): Promise<void> {
    const invoke = await getTauriInvoke();
    await invoke('auth_unlock_vault', { password });
  }

  /**
   * 锁定 Vault
   */
  async lockVault(): Promise<void> {
    const invoke = await getTauriInvoke();
    await invoke('auth_lock_vault');
  }

  /**
   * 检查是否已完成初始化
   */
  async isBootstrapped(): Promise<boolean> {
    const invoke = await getTauriInvoke();
    return invoke<boolean>('auth_is_bootstrapped');
  }

  /**
   * 列出已安装插件
   */
  async listPlugins(): Promise<PluginInfo[]> {
    const invoke = await getTauriInvoke();
    return invoke<PluginInfo[]>('plugin_list_installed');
  }

  /**
   * 安装市场插件
   */
  async installPlugin(pluginId: string): Promise<void> {
    const invoke = await getTauriInvoke();
    await invoke('plugin_install', { plugin_id: pluginId });
  }

  /**
   * 运行插件
   */
  async runPlugin(
    pluginId: string,
    params?: Record<string, string>,
  ): Promise<PluginRunResult> {
    const invoke = await getTauriInvoke();
    return invoke<PluginRunResult>('plugin_run', { plugin_id: pluginId, params: params ?? {} });
  }
}
