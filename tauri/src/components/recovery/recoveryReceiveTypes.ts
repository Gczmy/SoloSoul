/** RecoveryReceiveDialog 的共享类型与常量。 */

export interface RecoveryResultSummary {
  objectCount: number;
  attachmentCount: number;
  /** 恢复包的账户 ID（与旧设备一致）。 */
  accountId: string;
  /** 恢复包的账户名。 */
  accountName: string;
}

export interface RecoveryDiscoveredHost {
  name: string;
  addr: string;
  fingerprint: string;
}

/** 从 `t:"rec"` 二维码解析出的恢复连接信息。 */
export interface ScannedRecoveryQr {
  addr: string;
  pin: string;
  fingerprint: string;
  nonce: string | null;
  /** 二维码中携带的账户 ID（预览用，最终以后端回传为准）。 */
  accountId?: string;
  /** 二维码中携带的账户名（预览用，最终以后端回传为准）。 */
  accountName?: string;
}

export type TabMode = 'scan' | 'manual';
/** 流程阶段：collect=获取连接信息（扫码/手动），account=账户卡+设置主密码，success=成功卡片 */
export type Step = 'collect' | 'account' | 'success';

export const PIN_REGEX = /^\d{6}$/;
export const MDNS_DISCOVER_TIMEOUT_MS = 5000;
