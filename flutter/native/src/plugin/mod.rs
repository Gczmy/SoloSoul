//! Plugin module - Wasm sandbox for plugin execution
//!
//! Provides:
//! - Wasmtime integration
//! - Host functions for secure data access
//! - Plugin handshake verification

mod sandbox;
mod host;
mod manifest;

pub use sandbox::*;
pub use host::*;
pub use manifest::*;
