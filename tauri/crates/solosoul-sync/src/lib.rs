//! SoloSoul 同步引擎库
//!
//! 提供：
//! - CRDT 数据结构
//! - 网络传输（TCP + Noise）
//! - mDNS 服务发现

pub mod crdt;
pub mod discovery;
pub mod noise;
pub mod transport;
