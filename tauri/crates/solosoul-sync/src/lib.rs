//! SoloSoul 同步引擎库
//!
//! 提供：
//! - CRDT 数据结构
//! - 网络传输（TCP + Noise）
//! - mDNS 服务发现

pub mod crdt;
pub mod delta;
pub mod discovery;
pub mod hlc;
pub mod manager;
pub mod noise;
pub mod protocol;
pub mod transport;

pub use hlc::{Hlc, SyncWatermark};
pub use manager::{SyncManager, SyncPeerInfo};
pub use noise::NoiseKeys;
pub use protocol::{SyncMessage, SyncRecord};
