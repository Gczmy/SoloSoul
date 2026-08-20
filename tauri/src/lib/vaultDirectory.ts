import { invokeCommand as invoke } from '@/lib/ipcClient';
import type { AccountInfo } from '@/lib/ipc';

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
  /** 初始化后检测到的已有账户列表，用于引导页展示账户名称。 */
  accounts?: AccountInfo[];
}

export async function initVaultDirectory(
  safTreeUri: string | null,
): Promise<InitializeVaultResult> {
  return invoke<InitializeVaultResult>('init_vault_directory', {
    payload: { safTreeUri },
  });
}

/**
 * SAF 选择器结果丢失（Tauri Android 框架缺陷的残余场景）。
 *
 * tauri #15798 修复了 Activity 重建后 launcher 失效导致的 invoke 永久挂起；
 * 这里作为前端兜底：选择器打开（页面隐藏）后重新可见但 invoke 仍未返回，
 * 判定结果已丢失并报错，而不是让用户永远卡在"加载中"。
 */
export class VaultDirPickLostError extends Error {
  constructor() {
    super('VAULT_DIR_PICK_RESULT_LOST');
    this.name = 'VaultDirPickLostError';
  }
}

export async function pickVaultDirectory(): Promise<string | null> {
  let settled = false;
  const raw = invoke<{ uri?: string | null }>('vault_pick_directory').finally(() => {
    settled = true;
  });

  let graceTimer: ReturnType<typeof window.setInterval> | undefined;
  let rejectLost: ((e: VaultDirPickLostError) => void) | undefined;

  // 兜底检测：页面隐藏（SAF 选择器打开）→ 重新可见（选择器已关闭）后，
  // 给 invoke 结果投递留 8 秒缓冲；仍无结果则判定丢失。
  // 正常路径下结果在选择器关闭后数百毫秒内即返回，不会误触发。
  let wasHidden = false;
  const onVisibilityChange = () => {
    if (document.visibilityState === 'hidden') {
      wasHidden = true;
    } else if (wasHidden && !settled && !graceTimer) {
      const graceStart = Date.now();
      graceTimer = window.setInterval(() => {
        if (settled) {
          window.clearInterval(graceTimer);
          graceTimer = undefined;
          return;
        }
        if (Date.now() - graceStart >= 8000) {
          window.clearInterval(graceTimer);
          graceTimer = undefined;
          rejectLost?.(new VaultDirPickLostError());
        }
      }, 500);
    }
  };
  document.addEventListener('visibilitychange', onVisibilityChange);

  const lostResult = new Promise<never>((_, reject) => {
    rejectLost = reject;
  });

  try {
    const result = await Promise.race([raw, lostResult]);
    return result.uri ?? null;
  } finally {
    // P020: 无论正常返回/取消/结果丢失，统一移除监听器并清理宽限定时器——
    // 桌面端走系统对话框不触发 visibility 变化，旧实现监听器永久残留逐次累积。
    document.removeEventListener('visibilitychange', onVisibilityChange);
    if (graceTimer) {
      window.clearInterval(graceTimer);
      graceTimer = undefined;
    }
  }
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
