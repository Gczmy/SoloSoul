//! SoloSoul 同步引擎库
//!
//! 提供：
//! - CRDT 数据结构
//! - 网络传输（TCP + Noise）
//! - mDNS 服务发现

pub mod hlc;
pub mod protocol;
pub mod types;

#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub mod attachments;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub mod delta;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub mod manager;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub mod noise;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub mod service;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub mod transport;

#[cfg(any(target_os = "android", target_os = "ios"))]
mod mobile;

pub use hlc::{Hlc, SyncWatermark};
pub use protocol::{AttachmentInfo, SyncMessage, SyncRecord};
pub use types::{
    ApplyStats, AttachmentSyncStats, ConflictRecord, SyncPeerInfo, SyncSessionResult, TableStats,
};

#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub use manager::SyncManager;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub use noise::NoiseKeys;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub use service::SyncService;

#[cfg(any(target_os = "android", target_os = "ios"))]
pub use mobile::{NoiseKeys, SyncService};
