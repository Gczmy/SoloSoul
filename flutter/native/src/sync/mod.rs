//! Sync module - E2EE cloud synchronization
//!
//! Provides:
//! - Encrypted blob upload/download
//! - Sequence number versioning
//! - WebSocket real-time sync
//! - Conflict resolution

mod engine;
mod protocol;

pub use engine::*;
pub use protocol::*;
