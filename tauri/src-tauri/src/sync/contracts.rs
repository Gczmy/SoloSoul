//! 同步、发现与恢复的 IPC wire DTO。展示字段由前端派生。
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 同步冲突载荷 DTO：桌面端与移动端**共用**的序列化形状（P001）。
///
/// 底层 `Hlc.node_id: [u8; 16]` 在桌面端会被 serde 序列化为 `number[]`，
/// 而移动端旧实现用本地复刻 `MobileHlc`（`node_id: String`）——同一载荷在
/// 两个平台形状不同，Android 上前端任何读取 `node_id` 的逻辑都会拿到 string。
/// 统一经 hex 编码为字符串，并删除移动端复刻结构。
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct SyncConflictDto {
    pub table: String,
    pub id: String,
    pub local_hlc: ConflictHlc,
    pub remote_hlc: ConflictHlc,
    pub winner: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncPeer {
    pub id: String,
    pub name: String,
    pub custom_name: Option<String>,
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
    pub is_discovering: bool,
    pub sync_enabled: bool,
    pub auto_sync_enabled: bool,
    pub local_fingerprint: String,
    pub connected_peers: Vec<SyncPeer>,
}

#[derive(Serialize)]
pub struct SyncResult {
    pub summary: String,
    pub examined: u64,
    pub applied: u64,
    pub skipped: u64,
    pub conflicts: Vec<SyncConflictDto>,
    pub per_table: Vec<TableResult>,
}

#[derive(Serialize)]
pub struct TableResult {
    pub table: String,
    pub examined: u64,
    pub applied: u64,
    pub skipped: u64,
}

/// 同步冲突摘要，前端列表使用。
#[derive(Serialize)]
pub struct ConflictSummary {
    pub id: String,
    pub table: String,
    pub record_id: String,
    pub local_hlc: ConflictHlc,
    pub remote_hlc: ConflictHlc,
    pub winner: String,
    pub created_at: String,
}

/// 同步冲突 HLC（统一 DTO，桌面/移动共用）。
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct ConflictHlc {
    pub wall_time_ms: u64,
    pub counter: u64,
    pub node_id: String,
}

/// 同步冲突详情，前端 Diff 使用。
#[derive(Serialize)]
pub struct ConflictDetail {
    pub id: String,
    pub table: String,
    pub record_id: String,
    pub local_hlc: ConflictHlc,
    pub remote_hlc: ConflictHlc,
    pub local_data: serde_json::Value,
    pub remote_data: serde_json::Value,
    pub remote_deleted: bool,
    pub winner: String,
    pub created_at: String,
}

/// 通过 mDNS 发现的恢复主机信息。
///
/// 安全约束：mDNS TXT 广播**不携带** PIN 与 nonce（二者仅经 QR 码/手动输入
/// 带外传递）。此前将 PIN+nonce 写入明文 TXT，局域网内任意主机浏览
/// `_solosoul_recovery._tcp.local.` 即可直接通过认证下载恢复包（完整 Vault
/// 失陷）。发现到主机后，PIN 由用户从主机屏幕/QR 手动输入。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryDiscoveredHost {
    /// 主机显示名称（由主机指纹截取生成）。
    pub name: String,
    /// 连接地址（host:port）。
    pub addr: String,
    /// 主机公钥指纹（用于 MITM 验证）。
    pub fingerprint: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredDevice {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub addresses: Vec<String>,
    /// 对端公钥指纹（mDNS TXT 广播，用于前端「已发现设备」详情与已知设备匹配）。
    /// 旧版对端/未解析时为空串。
    #[serde(default)]
    pub fingerprint: String,
    /// 对端客户端类型（macos/windows/linux/android/ios/unknown）。
    /// 优先来自 TXT 广播；旧版对端回退按 node_id 查本机 vault peer 记录；
    /// 从未同步过的设备为 unknown（前端兜底显示通用图标）。
    pub client_type: String,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryHostInfo {
    pub display_addr: String,
    pub bind_addr: String,
    pub pin: String,
    pub nonce: String,
    pub fingerprint: String,
    /// 供前端生成 QR 码的 JSON 字符串。
    pub qr_payload: String,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResultSummary {
    #[serde(flatten)]
    pub outcome: crate::commands::export_import::contracts::ImportResult,
    /// 恢复包的账户 ID（与旧设备一致，用于在卡片上展示）。
    pub account_id: String,
    /// 恢复包的账户名。
    pub account_name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudSyncConfigPayload {
    pub account_id: String,
    pub connector_type: String,
    pub config_json: Value,
    pub enabled: bool,
    pub interval_secs: u64,
    pub wifi_only: bool,
    pub retention: serde_json::Value,
    /// 快照包加密口令（与主密码独立；保存时经主密码验证后入 Vault）。
    #[serde(default)]
    pub snapshot_password: String,
    /// 自动导入云端新快照。
    #[serde(default)]
    pub auto_import: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncPairingRequest {
    pub node_id: String,
    pub fingerprint: String,
    pub addr: String,
    pub device_name: String,
    pub sas_code: String,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncCompleted {
    pub peer_node_id: String,
    pub examined: u64,
    pub applied: u64,
    pub skipped: u64,
    pub conflicts: u64,
    pub outbound_records: u64,
}
#[derive(Debug, Clone, Serialize)]
pub struct SyncConflictsUpdated {
    pub count: u64,
}
#[derive(Debug, Clone, Serialize)]
pub struct SyncNsdFailed {
    pub error: String,
}

/// 多个实际发送方共享的开放 phase 协议；省略键继续省略，不发送 null。
#[derive(Debug, Clone, Default, Serialize)]
pub struct SyncProgress {
    pub phase: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub silent: Option<bool>,
}
impl SyncProgress {
    pub fn counters(phase: &str, current: u64, total: u64) -> Self {
        Self {
            phase: phase.into(),
            current: Some(current),
            total: Some(total),
            ..Default::default()
        }
    }
}
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "phase")]
pub enum DeviceSyncAutoStatus {
    #[serde(rename = "sync_start")]
    Start { source: String, peer_count: usize },
    #[serde(rename = "sync_complete")]
    Complete {
        source: String,
        message: Option<String>,
    },
    #[serde(rename = "error")]
    Error {
        source: String,
        message: Option<String>,
    },
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudSyncStatus {
    pub account_id: String,
    pub session_generation: u64,
    pub phase: String,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudSyncIncoming {
    pub account_id: String,
    pub session_generation: u64,
    pub files: Vec<String>,
    pub hint: String,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryProgress {
    pub phase: String,
    pub percent: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,
}
#[cfg(test)]
mod tests;
