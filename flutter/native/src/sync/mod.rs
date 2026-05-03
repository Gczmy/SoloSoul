//! Sync module - E2EE cloud synchronization
//!
//! Provides:
//! - Encrypted blob upload/download
//! - Sequence number versioning
//! - WebSocket real-time sync
//! - Conflict resolution
//! - CRDT-based profile replication

pub mod crdt;
pub mod engine;
pub mod protocol;
pub mod transport;

pub use engine::*;
pub use protocol::*;
