//! Vault service - Tauri-side re-export of `solosoul_core::vault_service`.
//!
//! The actual implementation was moved to `crates/solosoul-core` so that the
//! terminal CLI can share the same account and vault lifecycle logic without
//! depending on the Tauri runtime. This file exists only to keep existing
//! `crate::services::vault_service::*` imports working.

pub use solosoul_core::vault_service::*;
