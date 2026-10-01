/** 同步展示模型；wire 类型仅从生成契约派生，旧历史允许缺少展示元数据。 */
import type {
  SyncPeer as WirePeer,
  SyncStatus as WireStatus,
  DiscoveredDevice as WireDevice,
  SyncResult as WireResult,
} from './generated/ipcContracts';

export type SyncPeer = Omit<WirePeer, 'customName' | 'lastSeenTs' | 'trustedAt' | 'clientType'> &
  Partial<Pick<WirePeer, 'customName' | 'lastSeenTs' | 'trustedAt' | 'clientType'>> & {
    sasCode?: string;
  };
export type DiscoveredDevice = Omit<WireDevice, 'fingerprint' | 'clientType'> &
  Partial<Pick<WireDevice, 'fingerprint' | 'clientType'>>;
export type SyncStatus = Omit<WireStatus, 'connectedPeers'> & { connectedPeers: SyncPeer[] };
/** 可选的 UI 操作集合；Host 的 strategy 参数仍是 String。 */
export type SyncConflictStrategy = 'keep_local' | 'keep_remote' | 'dismiss';
export interface SyncResult extends WireResult {
  /** 客户端侧标记：true 表示该结果来自入站同步完成事件（sync-completed），
   *  而非本端主动发起的同步。同步页据此跳过通用「同步完成」toast 避免双弹。 */
  inbound?: boolean;
  /** B：响应方（入站会话）本次发回给发起方的记录条数。仅入站结果携带
   *  （发起方结果无此字段），展示完整交换量避免「检查 0 条」误导。 */
  outboundRecords?: number;
  /** 入站事件的冲突数（SyncConflict[] 在入站路径为空数组，事件单独携带计数）。
   *  展示时优先本字段，缺失时回退 conflicts.length。 */
  conflictCount?: number;

  // ── 客户端盖章字段（同步活动面板增强，非后端 DTO，仅前端记录时写入）──

  /** 本地记录时间戳（unix 毫秒）。手动同步在 syncWithDevice 返回时盖章，
   *  入站同步在 sync-completed 事件收到时盖章；旧版历史（重启前已持久化）缺失。 */
  at?: number;
  /** 对端设备名（记录时从 connectedPeers 解析并固化，避免对端之后被忘记导致无法追溯）。 */
  peerName?: string;
  /** 对端客户端类型（macos/windows/linux/android/ios/unknown），面板据此渲染设备图标。 */
  peerClientType?: string;
  /** 对端 node id（手动同步=deviceId，入站=事件 peerNodeId）。 */
  peerNodeId?: string;
  /** 失败条目标记：手动同步失败时写入历史，供同步活动面板展示失败记录（旧版失败仅横幅一闪而过）。 */
  failed?: boolean;
  /** 失败原因（后端原始错误串，展示时经 resolveBackendErrorMessage 本地化）。 */
  errorSummary?: string;
}
