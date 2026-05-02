//! Plugin module - Wasm sandbox for plugin execution
//!
//! Provides:
//! - Wasmtime integration
//! - Host functions for secure data access
//! - Plugin handshake verification

#[cfg(feature = "sandbox")]
mod host;
#[cfg(feature = "sandbox")]
mod manifest;
#[cfg(feature = "sandbox")]
mod sandbox;

#[cfg(feature = "sandbox")]
pub use host::*;
#[cfg(feature = "sandbox")]
pub use manifest::*;
#[cfg(feature = "sandbox")]
pub use sandbox::*;
