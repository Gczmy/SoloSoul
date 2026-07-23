import { invoke } from '@tauri-apps/api/core';

export interface VaultDirectoryInfo {
  directoryType: 'local' | 'saf';
  safTreeUri: string | null;
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
  return invoke<SetVaultDirectoryResult>('vault_set_directory', { safTreeUri });
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
