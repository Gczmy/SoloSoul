//! SoloSoul 同步引擎库
//!
//! 提供：
//! - CRDT 数据结构
//! - 网络传输（TCP + Noise）
//! - mDNS 服务发现

pub mod hlc;
pub mod protocol;
pub mod types;

// 共享模块：协议、Noise、传输、Delta、附件同步逻辑在所有平台可用。
pub mod attachments;
pub mod delta;
pub mod identity;
pub mod noise;
pub mod recovery;
pub mod session;
pub mod shared;
pub mod transport;

// 桌面端使用基于 mdns-sd 的 SyncManager；移动端使用基于 Android NSD / iOS Bonjour 的
// 发现层，因此 manager/service 按平台拆分。
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub mod manager;
#[cfg(any(target_os = "android", target_os = "ios"))]
pub mod mobile;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub mod service;

pub use hlc::{Hlc, SyncWatermark};
pub use protocol::{AttachmentInfo, SyncMessage, SyncRecord};
pub use types::{
    ApplyStats, AttachmentSyncStats, ConflictRecord, SyncPeerInfo, SyncSessionResult, TableStats,
};

#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub use manager::SyncManager;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub use service::SyncService;

// 账户哈希计算（mDNS TXT account_hash 广播/比对），供 GUI 发现层过滤复用。
pub use identity::sha256_hex_short;

// 本机客户端类型（macos/windows/...），供 GUI 在 mDNS/NSD TXT 广播中使用。
pub use session::local_client_type;

#[cfg(any(target_os = "android", target_os = "ios"))]
pub use mobile::SyncService;

// NoiseKeys 在 noise 模块中定义；桌面端与移动端统一从此处 re-export，
// 避免 mobile 模块再导出时与 lib.rs 的 re-export 产生冲突。
pub use noise::NoiseKeys;
