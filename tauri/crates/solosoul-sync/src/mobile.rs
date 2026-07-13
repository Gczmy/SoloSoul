//! 同步引擎移动端占位实现。
//!
//! 移动端 MVP 暂不支持设备同步，因此本模块提供与桌面端兼容的公共类型签名，
//! 所有实际功能均为空实现或返回错误，确保 crate 在移动平台可编译。

use solosoul_core::vault_service::VaultService;
use std::sync::Arc;
use tokio::sync::Mutex;

/// 长周期 Noise 身份密钥占位。
#[derive(Debug, Clone)]
pub struct NoiseKeys;

impl NoiseKeys {
    /// 生成占位密钥。
    pub fn generate() -> Self {
        Self
    }

    /// 从持久化密钥恢复（占位）。
    pub fn from_secret(_secret: [u8; 32]) -> Self {
        Self
    }

    pub fn secret_key(&self) -> &[u8; 32] {
        &[0u8; 32]
    }

    pub fn public_key(&self) -> &[u8; 32] {
        &[0u8; 32]
    }

    /// 占位指纹。
    pub fn fingerprint(&self) -> String {
        String::new()
    }
}

/// 同步服务占位。
pub struct SyncService {
    _vault_service: Arc<std::sync::RwLock<VaultService>>,
    _manager: Mutex<Option<()>>,
}

impl SyncService {
    pub fn new(vault_service: Arc<std::sync::RwLock<VaultService>>) -> Self {
        Self {
            _vault_service: vault_service,
            _manager: Mutex::new(None),
        }
    }

    /// 启用/禁用同步（占位，返回不支持错误）。
    pub async fn enable(&self, _enable: bool) -> Result<(), String> {
        Err("移动端暂不支持设备同步".to_string())
    }

    /// 是否已启用同步。
    pub async fn is_enabled(&self) -> bool {
        false
    }

    /// 手动同步设备（占位，返回不支持错误）。
    pub async fn sync_with_device(
        &self,
        _device_id_or_addr: String,
    ) -> Result<crate::types::SyncSessionResult, String> {
        Err("移动端暂不支持设备同步".to_string())
    }

    /// 列出已知 peer（占位，返回空列表）。
    pub async fn known_peers(&self) -> Result<Vec<crate::types::SyncPeerInfo>, String> {
        Ok(Vec::new())
    }

    /// 标记 peer 信任状态（占位，返回不支持错误）。
    pub async fn trust_peer(&self, _peer_node_id: String, _trusted: bool) -> Result<(), String> {
        Err("移动端暂不支持设备同步".to_string())
    }

    /// 移除 peer（占位，返回不支持错误）。
    pub async fn forget_peer(&self, _peer_node_id: String) -> Result<(), String> {
        Err("移动端暂不支持设备同步".to_string())
    }

    /// 返回本地指纹（占位，返回空字符串）。
    pub async fn local_fingerprint(&self) -> Result<String, String> {
        Ok(String::new())
    }
}
