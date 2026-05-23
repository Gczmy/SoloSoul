//! Plugin module - Wasm sandbox for plugin execution
//!
//! Provides:
//! - Wasmtime integration
//! - Host functions for secure data access
//! - Plugin handshake verification

#[cfg(feature = "sandbox")]
pub mod field_map;
#[cfg(feature = "sandbox")]
pub mod host;
#[cfg(feature = "sandbox")]
pub mod manifest;
#[cfg(feature = "sandbox")]
pub mod manager;
#[cfg(feature = "sandbox")]
pub mod sandbox;
#[cfg(feature = "sandbox")]
pub mod session;
#[cfg(feature = "sandbox")]
pub mod store;

#[cfg(feature = "sandbox")]
pub use field_map::*;
#[cfg(feature = "sandbox")]
pub use host::*;
#[cfg(feature = "sandbox")]
pub use manifest::*;
#[cfg(feature = "sandbox")]
pub use manager::*;
#[cfg(feature = "sandbox")]
pub use sandbox::*;
#[cfg(feature = "sandbox")]
pub use session::*;
#[cfg(feature = "sandbox")]
pub use store::*;
