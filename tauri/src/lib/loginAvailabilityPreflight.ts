// =============================================================================
// 登录可用性预探测（方案 C）
//
// 背景：登录页的指纹/PIN 可用性探测原本在页面挂载后才发起（2 个异步 IPC），
// 首帧只能先渲染主密码。这里把探测提前到应用启动期（账户 id 从
// LAST_ACCOUNT_KEY 同步可知），结果以模块级 promise 缓存——登录页挂载时
// 探测往往已就绪或更快返回，配合 loginMethodCache（方案 A）首帧即可恢复
// 正确登录方式。启动期账户与登录页实际选中账户不一致时，登录页 effect 会用
// 实际账户重新探测（本模块按账户缓存、换账户自动重发）。
// =============================================================================

import { invokeCommand as invoke } from '@/lib/ipcClient';
import { LAST_ACCOUNT_KEY } from '@/stores/authStore';
import { logger } from '@/lib/logger';

/** P038: 受支持的生物识别类型白名单（显示名由 LoginBiometricView 的查表负责）。 */
const BIOMETRIC_INFO: Record<string, string> = {
  faceId: 'faceId',
  touchId: 'touchId',
  windowsHello: 'windowsHello',
};

/** 归一化后端 biometryType 为受支持的白名单值（未知/缺失回退 touchId）。 */
export function normalizeBiometryType(raw?: string): string {
  return raw ? (BIOMETRIC_INFO[raw] ?? 'touchId') : 'touchId';
}

export interface LoginAvailability {
  /** 已配置凭证且（设备可用或系统临时锁定）→ 指纹项保留。 */
  bioAvailable: boolean;
  /** 系统生物识别因失败次数过多被临时锁定（Android canAuthenticate ERROR_LOCKOUT）。 */
  bioLockout: boolean;
  biometryTypeRaw: string;
  pinAvailable: boolean;
}

/** 模块级缓存：同一账户的探测结果复用（换账户自动失效重发）。 */
let preflight: { accountId: string; promise: Promise<LoginAvailability> } | null = null;

/**
 * 探测指定账户的指纹/PIN 可用性。同账户重复调用返回同一 promise（去重）；
 * 换账户或上次探测失败时重新发起。失败会清除缓存，下次调用可重试。
 */
export function preflightLoginAvailability(accountId: string): Promise<LoginAvailability> {
  if (preflight && preflight.accountId === accountId) return preflight.promise;

  const promise = (async (): Promise<LoginAvailability> => {
    const [bio, pin] = await Promise.all([
      invoke<{
        available: boolean;
        configured: boolean;
        biometryType?: string;
        lockout?: boolean;
      }>('biometric_check_availability', { accountId }),
      invoke<{ configured: boolean; locked: boolean }>('pin_check_availability', {
        accountId,
      }),
    ]);
    // lockout 以 !!r.lockout 为准：即使 available 与 lockout 同时成立
    // （Android 插件 status() 在锁定期间可能仍报可用），也正确标记警告。
    const bioLockout = !!bio.lockout;
    return {
      bioAvailable: !!(bio.configured && (bio.available || bioLockout)),
      bioLockout,
      biometryTypeRaw: normalizeBiometryType(bio.biometryType),
      pinAvailable: !!(pin.configured && !pin.locked),
    };
  })();

  preflight = { accountId, promise };
  // 失败即失效缓存，下次调用重新探测（避免一次性失败污染整个会话）
  promise.catch(() => {
    if (preflight?.promise === promise) preflight = null;
  });
  return promise;
}

/**
 * 启动期预探测：从 LAST_ACCOUNT_KEY 读取上次账户并提前发起探测（非阻塞）。
 * main.tsx 在 React 渲染前调用，与 preloadCameraCapability 同模式。
 */
export function preflightForLastAccount(): void {
  let accountId = '';
  try {
    accountId = localStorage.getItem(LAST_ACCOUNT_KEY) || '';
  } catch {
    // localStorage 不可用时保持空串（跳过预探测，登录页挂载时再探测）
  }
  if (!accountId) return;
  preflightLoginAvailability(accountId).catch((err) =>
    logger.warn('[main] Login availability preflight failed:', err),
  );
}

/**
 * 使指定账户的预探测缓存失效（登录方式修改后调用）。
 *
 * 背景：模块级缓存永不过期——设置页开启/关闭 PIN、生物识别后，锁定账户回到
 * 登录页仍读到旧探测结果（新方式不显示 / 已关闭方式仍显示），重启应用才消失。
 * 修改成功后失效缓存，登录页挂载时即可重新探测，立即生效。
 *
 * 缓存为单槽（{ accountId, promise }）：仅当缓存账户与修改账户一致时清除，
 * 其他账户的缓存不受影响（换账户时 preflightLoginAvailability 本就会重发）。
 */
export function invalidateLoginAvailabilityPreflight(accountId: string): void {
  if (preflight?.accountId === accountId) {
    preflight = null;
  }
}

/** 测试专用：清空缓存（模拟新会话/换账户）。 */
export function __resetLoginAvailabilityPreflightForTest(): void {
  preflight = null;
}
