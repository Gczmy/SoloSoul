//! SoloSoul 共享核心库
//!
//! 本 crate 封装 SoloSoul GUI 与 CLI 共用的无 Tauri 依赖核心逻辑：
//! - Vault 生命周期管理（账户创建、解锁、锁定、密码验证）
//! - 默认模板种子数据导入
//! - 生物识别相关原语（Touch ID / Face ID / Windows Hello）
//! - 认证相关辅助函数
//!
//! 设计目标：任何交互宿主（Tauri GUI、终端 CLI、自动化脚本）都可直接依赖此 crate，
//! 而不必引入整个 Tauri 运行时。

pub mod auth;
pub mod biometric;
pub mod llm;
pub mod ocr;
pub mod process_lock;
pub mod template_service;
pub mod vault_service;

/// Crate version (from Cargo.toml at compile time).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

//  Convenience re-exports for callers that only need the public types.
pub use vault_service::{AccountConfig, AccountSummary, VaultService};

// Re-export vault storage types so CLI/GUI hosts can use them through a single crate.
pub use solosoul_vault::{
    AuditLogEntry, ObjectRecord, ObjectSummary, Profile, PropertyType, TemplateProperty, TrashItem,
    TrashItemSummary, UserTemplate, VaultStats, VaultStore,
};
