import { invoke } from '@tauri-apps/api/core';

export interface VaultDirectoryInfo {
  directoryType: 'local' | 'saf';
  safTreeUri: string | null;
  /** SAF tree URI 是否仍然可访问（授权未被撤销）。本地目录模式恒为 true。 */
  valid: boolean;
}

export interface SetVaultDirectoryResult {
  success: boolean;
  needsRestart: boolean;
  message: string;
}

export async function getVaultDirectory(): Promise<VaultDirectoryInfo> {
  return invoke<VaultDirectoryInfo>('vault_get_directory');
}

export async function setVaultDirectory(
  safTreeUri: string | null,
): Promise<SetVaultDirectoryResult> {
  // 参数名 payload 必须与 Rust 端 #[tauri::command] 的参数名一致
  return invoke<SetVaultDirectoryResult>('vault_set_directory', {
    payload: { safTreeUri },
  });
}

export interface InitializeVaultResult {
  success: boolean;
  needsRestart: boolean;
  message: string;
  /** 初始化后检测到的已有账户数量（0 = 新用户需创建，>0 = 直接登录）。 */
  accountCount?: number;
}

export async function initVaultDirectory(
  safTreeUri: string | null,
): Promise<InitializeVaultResult> {
  return invoke<InitializeVaultResult>('init_vault_directory', {
    payload: { safTreeUri },
  });
}

export async function pickVaultDirectory(): Promise<string | null> {
  const result = await invoke<{ uri?: string | null }>('vault_pick_directory');
  return result.uri ?? null;
}

export async function syncVaultToRemote(): Promise<void> {
  return invoke<void>('vault_sync_to_remote');
}

export async function syncVaultFromRemote(): Promise<void> {
  return invoke<void>('vault_sync_from_remote');
}

/** 检查 SAF 目录是否仍然可访问（授权未被撤销）。返回 true 表示有效。 */
export async function checkVaultDirectory(): Promise<boolean> {
  return invoke<boolean>('vault_check_directory');
}
