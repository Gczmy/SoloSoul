//! 同步引擎公共类型定义。
//!
//! 这些类型被 service.rs 与 manager.rs 共用，也用于移动端占位实现，
//! 因此放在独立模块中，避免条件编译导致类型不可用。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Information about a discovered or known peer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncPeerInfo {
    pub node_id: String,
    pub account_id: String,
    pub name: String,
    pub addr: String,
    pub fingerprint: String,
    pub trusted: bool,
    pub last_seen: String,
    /// 最近一次同步/在线的原始 unix 秒时间戳（未格式化的相对串）。
    /// 前端据此展示精确的「最近同步时间」。
    pub last_seen_ts: Option<i64>,
    /// 最近一次信任该设备的时间（unix 秒）。从未信任/已撤销时为 None。
    pub trusted_at: Option<i64>,
    /// 客户端类型：macos / windows / linux / android / ios / unknown。
    pub client_type: String,
}

/// Attachment synchronization statistics.
#[derive(Debug, Clone, Default)]
pub struct AttachmentSyncStats {
    pub sent: u64,
    pub received: u64,
    pub bytes_transferred: u64,
    pub errors: Vec<String>,
}

/// Per-table sync statistics.
#[derive(Debug, Clone, Default)]
pub struct TableStats {
    pub examined: u64,
    pub applied: u64,
    pub skipped: u64,
}

/// Result of applying a batch of sync records.
#[derive(Debug, Clone, Default)]
pub struct ApplyStats {
    pub examined: u64,
    pub applied: u64,
    pub skipped: u64,
    pub errors: Vec<String>,
    pub conflicts: Vec<ConflictRecord>,
    pub per_table: HashMap<String, TableStats>,
}

/// A single sync conflict.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictRecord {
    pub table: String,
    pub id: String,
    pub local_hlc: crate::hlc::Hlc,
    pub remote_hlc: crate::hlc::Hlc,
    pub winner: String,
}

/// Result of a full sync session, including database deltas and attachment transfers.
#[derive(Debug, Clone, Default)]
pub struct SyncSessionResult {
    pub data: ApplyStats,
    pub attachments: AttachmentSyncStats,
}

/// 新 peer 配对请求信息（响应方在入站 Hello 落库一条新的未信任记录时触发）。
#[derive(Debug, Clone)]
pub struct NewPeerInfo {
    pub node_id: String,
    pub fingerprint: String,
    pub addr: String,
    pub device_name: String,
    /// 发起方客户端类型（macos/windows/linux/android/ios/unknown）。
    pub client_type: String,
    /// 6 位 SAS 配对验证码：响应方从本次握手哈希本地派生，
    /// 与发起方各自派生的值一致，配对卡片两侧展示供用户目视比对。
    pub sas_code: String,
}

/// 新 peer 回调钩子：入站 Hello record_peer 落库一条新的未信任记录时触发。
/// 供 GUI 装配 `sync-pairing-request` 事件推送（B 用户不在同步页也能收到配对请求）。
pub type PeerCallback = Arc<dyn Fn(NewPeerInfo) + Send + Sync>;

/// 入站同步会话的完整结果（含对端身份），供响应方通知前端「对方已完成同步」。
///
/// 响应方（被连接方）此前收不到任何会话结果——`handle_inbound` 的返回值在 accept
/// 循环里被丢弃，用户只能看到发起方一侧的完成提示。携带对端 node_id 后，GUI 层
/// 可推送 `sync-completed` 事件，让两侧同时展示同步完成提醒与具体条数。
#[derive(Debug, Clone)]
pub struct InboundSessionOutcome {
    /// 发起方 peer 的 node_id。
    pub peer_node_id: String,
    /// B：响应方本次会话发回给发起方的记录条数（完整交换量的一侧）。
    /// 响应方完成事件据此展示「发回对端 X 条」，避免只显示入站方向计数。
    pub outbound_records: u64,
    pub result: SyncSessionResult,
}

/// 响应方完成一次入站同步会话后的统计摘要（事件载荷，供前端直接消费）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionCompletedInfo {
    /// 发起方 peer 的 node_id。
    pub peer_node_id: String,
    pub examined: u64,
    pub applied: u64,
    pub skipped: u64,
    /// 本次会话产生的冲突数量（含单侧删除）。
    pub conflicts: u64,
    /// B：响应方本次会话发回给发起方的记录条数（入站方向计数之外的完整交换量）。
    /// 旧版响应方完成事件只含入站方向（examined/applied/skipped），用户看到
    /// 「检查 0 条」误以为没同步；携带本字段后 toast 展示双向完整交换量。
    pub outbound_records: u64,
}

/// 会话完成回调钩子：入站同步会话成功结束时触发（携带对端身份与统计）。
/// 供 GUI 装配 `sync-completed` 事件推送（B 侧任意页面都能收到完成提醒）。
pub type SessionCompletedCallback = Arc<dyn Fn(SessionCompletedInfo) + Send + Sync>;
