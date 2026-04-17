//! Crypto module - High-performance encryption primitives
//!
//! Provides:
//! - Argon2id key derivation
//! - AES-256-GCM encryption/decryption
//! - Secure random generation

pub mod argon2;
pub mod aes;
pub mod utils;

pub use argon2::*;
pub use aes::*;
pub use utils::*;
