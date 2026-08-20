import type { TFunction } from 'i18next';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { translateRustError } from '@/lib/rustErrors';
import type { AccountInfo } from '@/lib/ipc';

/**
 * 连接失败：把底层错误映射为带诊断引导的友好提示。
 * 纯函数（依赖注入 `t`），便于独立测试。
 */
export function friendlyConnectError(raw: string, t: TFunction): string {
  const lower = raw.toLowerCase();
  if (lower.includes('timed out') || lower.includes('timeout')) {
    // 连接超时：最常见原因是 macOS/Windows 防火墙拦截入站连接，
    // 或两台设备不在同一网络。给出可操作的排查步骤。
    return t('common:recovery_connect_timeout');
  }
  if (lower.includes('unreachable') || lower.includes('no route')) {
    // 目标网络不可达：典型为跨网段/不在同一网络。
    return t('common:recovery_connect_unreachable');
  }
  if (lower.includes('refused') || lower.includes('connection reset')) {
    // 连接被拒：主机端未在监听（会话过期/已取消）或端口不可达。
    return t('common:recovery_connect_refused');
  }
  if (
    lower.includes('read prefix failed') ||
    lower.includes('failed to fill whole buffer') ||
    lower.includes('unexpected eof') ||
    lower.includes('invalid magic prefix')
  ) {
    // 主机在发送任何数据前就关闭了连接（EOF）：会话可能已结束（超时/被取消/已被使用），
    // 或主机在握手前发生了内部错误。引导用户在旧设备上重新生成恢复二维码后重试。
    return t('common:recovery_host_closed_early');
  }
  if (lower.includes('too many failed recovery attempts')) {
    // 全局限流：短时间内失败次数过多，恢复服务暂时拒绝新连接。
    return t('common:recovery_too_many_attempts');
  }
  if (lower.includes('invalid pin') || lower.includes('invalid nonce')) {
    // PIN/二维码随机数不匹配：二维码可能已过期、已被使用，或 PIN 输入有误。
    return t('common:recovery_invalid_pin');
  }
  if (lower.includes('identity verification failed') || lower.includes('possible mitm')) {
    // 指纹校验失败：可能为中间人攻击，或指纹输入与旧设备屏幕不一致。
    return t('common:recovery_mitm');
  }
  if (
    lower.includes('read handshake') ||
    lower.includes('unexpected auth response') ||
    lower.includes('did not provide a static public key')
  ) {
    // 握手中断 / 协议异常：会话可能已中断或失效。
    return t('common:recovery_handshake_failed');
  }
  if (lower.includes('incomplete transfer') || lower.includes('received more data than expected')) {
    // 传输中断：连接在传输过程中断开。
    return t('common:recovery_transfer_failed');
  }
  if (lower.includes('export file too large') || lower.includes('invalid file size')) {
    // 恢复包超过大小限制。
    return t('common:recovery_package_too_large');
  }
  if (lower.includes('recovery task failed')) {
    // spawn_blocking join 失败（任务 panic/abort 时的内部错误）。
    return t('common:recovery_task_failed');
  }
  // 兜底：未命中的已知 Rust 错误先尝试 i18n 映射（如 Account ID already exists），
  // 命中则返回本地化文案，未命中才返回原始错误。
  const translated = translateRustError(raw);
  if (translated) return t(translated);
  return raw;
}

/**
 * 扫描完成后的账户 ID 冲突预检：本设备已存在相同 account_id → 输入密码前即提示覆盖选项。
 * 检查失败视为无冲突，由后端在恢复时兜底提示。
 */
export async function checkRecoveryIdConflict(accountId?: string | null): Promise<boolean> {
  if (!accountId) return false;
  try {
    const accounts = await invoke<AccountInfo[]>('vault_list_accounts');
    return accounts.some((a) => a.id === accountId);
  } catch {
    return false;
  }
}
