// crates/solosoul-sync/src/service.rs
//
// Sync service — bridges the VaultService and the solosoul-sync SyncManager.
//
// P0.5 阶段把原来在 tauri/src-tauri/src/services/sync_service.rs 的实现整体迁到这里,
// 供 CLI 和 GUI 共用. 这一次迁入彻底删掉了此前所有自创的 helper 方法 (disable /
// local_fingerprint / status_snapshot …), 完全按 SyncManager / VaultService 真实公开
// API 重写, 以避免 12 个 E0282 类型推断错误 + 24 个误用 API 名错误.
//
// 与原版唯一的差异: 因为 service.rs 现在位于 solosoul-sync crate 内部,
// `use solosoul_sync::X` 必须改为 `use crate::X`; `mutex guard + match guard.as_ref()`
// 模式保留 (E0282 的根因是不规范的 in-crate import, 不是 match ergonomics).
//
// 模式上, 所有需要访问 manager 的方法统一沿用:
//   let guard = self.manager.lock().await;
//   let manager = guard.as_ref().ok_or("Sync is not enabled")?;
//   // manager: &SyncManager, 调用方法 .await / .x() 全部明确

use crate::manager::SyncManager;
use crate::noise::NoiseKeys;
use crate::types::{SyncPeerInfo, SyncSessionResult};
use std::sync::Arc;
use tokio::sync::Mutex;

use solosoul_core::vault_service::VaultService;

pub struct SyncService {
    vault_service: Arc<std::sync::RwLock<VaultService>>,
    manager: Mutex<Option<Arc<SyncManager>>>,
}

impl SyncService {
    pub fn new(vault_service: Arc<std::sync::RwLock<VaultService>>) -> Self {
        Self {
            vault_service,
            manager: Mutex::new(None),
        }
    }

    /// 启用或关闭后台同步守护进程.
    pub async fn enable(&self, enable: bool) -> Result<(), String> {
        let mut guard = self.manager.lock().await;
        if enable {
            if guard.is_some() {
                return Ok(());
            }
            let (vault, account_id) = {
                let svc = self
                    .vault_service
                    .read()
                    .map_err(|_| "Vault service lock poisoned".to_string())?;
                let vault = svc.get_vault_store().ok_or("Vault is not unlocked")?;
                let account_id = svc.get_current_account().ok_or("No account is unlocked")?;
                (vault, account_id)
            };
            let (node_id, keys) = get_or_create_sync_identity(&vault)?;
            let manager = SyncManager::new(node_id, account_id, keys, vault.clone(), "0.0.0.0:0");
            manager.start().await?;
            audit_log(
                &vault,
                "sync_enabled",
                None,
                Some(&format!("fingerprint={}", manager.fingerprint())),
            );
            *guard = Some(Arc::new(manager));
            Ok(())
        } else {
            let old_manager = guard.take();
            // 先释放 manager 锁，再在 blocking 线程执行 stop()：
            // stop() 可能等待活跃同步会话最多 STOP_GRACE_PERIOD_SECS（30 秒），
            // 若在此处同步调用会阻塞 async 命令线程，并让 sync_get_status 等
            // 需要 manager 锁的命令全部排队，前端表现为“禁用失败、所有按钮卡住”。
            // manager 已从锁中取出，后续 is_enabled()/sync_get_status 立即返回 false。
            drop(guard);
            if let Some(m) = old_manager {
                // 显式 drop JoinHandle 以分离任务（detach），命令立即返回
                std::mem::drop(tokio::task::spawn_blocking(move || m.stop()));
            }
            if let Ok(svc) = self.vault_service.try_read() {
                if let Some(vault) = svc.get_vault_store() {
                    audit_log(&vault, "sync_disabled", None, None);
                }
            }
            Ok(())
        }
    }

    /// 返回当前是否已开启 sync.
    pub async fn is_enabled(&self) -> bool {
        self.manager.lock().await.is_some()
    }

    /// 手动同步一个已发现的 peer (按 node id) 或一个 `host:port` 地址.
    ///
    /// 锁内仅克隆 `Arc<SyncManager>` 后立即释放锁，再执行会话：整个会话可能耗时
    /// 数十秒（10s 连接超时 + 数据交换），若持锁等待会让 `enable(false)` /
    /// `sync_get_status` 等命令全部排队，前端表现为“禁用失败、按钮卡住”。
    pub async fn sync_with_device(
        &self,
        device_id_or_addr: String,
    ) -> Result<SyncSessionResult, String> {
        let manager = {
            let guard = self.manager.lock().await;
            guard.as_ref().cloned().ok_or("Sync is not enabled")?
        };
        let result = manager.sync_with_peer(&device_id_or_addr).await?;
        let table_summary = result
            .data
            .per_table
            .iter()
            .map(|(table, s)| format!("{}:{}+{}/{}", table, s.applied, s.skipped, s.examined))
            .collect::<Vec<_>>()
            .join(", ");
        let summary = format!(
            "examined={}, applied={}, skipped={}; conflicts={}; tables=[{}]; attachments: sent={}, received={}, bytes={}",
            result.data.examined,
            result.data.applied,
            result.data.skipped,
            result.data.conflicts.len(),
            table_summary,
            result.attachments.sent,
            result.attachments.received,
            result.attachments.bytes_transferred
        );
        if let Ok(svc) = self.vault_service.try_read() {
            if let Some(vault) = svc.get_vault_store() {
                audit_log(
                    &vault,
                    "sync_with_device",
                    Some(&device_id_or_addr),
                    Some(&summary),
                );
            }
        }
        Ok(result)
    }

    /// 列出已发现并已持久化的 peers; 即便 manager 未启动也会回退到 vault 持久化列表.
    pub async fn known_peers(&self) -> Result<Vec<SyncPeerInfo>, String> {
        let guard = self.manager.lock().await;
        match guard.as_ref() {
            Some(m) => m.known_peers(),
            None => {
                let svc = self
                    .vault_service
                    .read()
                    .map_err(|_| "Vault service lock poisoned".to_string())?;
                let vault = svc.get_vault_store().ok_or("Vault is not unlocked")?;
                let account_id = svc.get_current_account().unwrap_or_default();
                let persisted = vault.list_peers()?;
                Ok(persisted
                    .into_iter()
                    .map(|p| SyncPeerInfo {
                        node_id: p.peer_node_id.clone(),
                        account_id: account_id.clone(),
                        name: p
                            .peer_name
                            .clone()
                            .unwrap_or_else(|| p.peer_node_id.clone()),
                        addr: String::new(),
                        fingerprint: p.public_key_fingerprint.clone().unwrap_or_default(),
                        trusted: p.trusted,
                        last_seen: p
                            .last_seen
                            .map(|ts| format!("{}s ago", chrono::Utc::now().timestamp() - ts))
                            .unwrap_or_default(),
                    })
                    .collect())
            }
        }
    }

    /// 把一个 peer 标记为信任/不信任; 即便 manager 未启动也会回写到 vault.
    pub async fn trust_peer(&self, peer_node_id: String, trusted: bool) -> Result<(), String> {
        let guard = self.manager.lock().await;
        let result = match guard.as_ref() {
            Some(m) => m.trust_peer(&peer_node_id, trusted),
            None => {
                let svc = self
                    .vault_service
                    .read()
                    .map_err(|_| "Vault service lock poisoned".to_string())?;
                let vault = svc.get_vault_store().ok_or("Vault is not unlocked")?;
                let now = chrono::Utc::now().to_rfc3339();
                let mut peer = vault.load_peer_state(&peer_node_id)?.unwrap_or_else(|| {
                    solosoul_vault::PeerSyncState {
                        peer_node_id: peer_node_id.clone(),
                        peer_name: None,
                        trusted: false,
                        public_key_fingerprint: None,
                        last_seen: None,
                        created_at: now.clone(),
                        updated_at: now.clone(),
                    }
                });
                peer.trusted = trusted;
                peer.updated_at = now;
                vault.save_peer_state(&peer)
            }
        };
        if result.is_ok() {
            if let Ok(svc) = self.vault_service.try_read() {
                if let Some(vault) = svc.get_vault_store() {
                    audit_log(
                        &vault,
                        if trusted {
                            "sync_peer_trusted"
                        } else {
                            "sync_peer_revoked"
                        },
                        Some(&peer_node_id),
                        None,
                    );
                }
            }
        }
        result
    }

    /// 从持久化状态中移除一个 peer (manager 启动时同步, 否则仅清持久化).
    pub async fn forget_peer(&self, peer_node_id: String) -> Result<(), String> {
        let guard = self.manager.lock().await;
        match guard.as_ref() {
            Some(m) => m.forget_peer(&peer_node_id),
            None => {
                let svc = self
                    .vault_service
                    .read()
                    .map_err(|_| "Vault service lock poisoned".to_string())?;
                let vault = svc.get_vault_store().ok_or("Vault is not unlocked")?;
                vault.delete_peer(&peer_node_id)
            }
        }
    }
    /// 返回当前同步服务的监听端口。
    ///
    /// Manager 未启动时返回 0，用于前端判断当前是否正在监听。
    pub async fn listen_port(&self) -> u16 {
        let guard = self.manager.lock().await;
        guard.as_ref().map(|m| m.listen_port()).unwrap_or(0)
    }

    /// 返回本地节点的 Noise 公钥指纹, GUI 状态栏 / pairing 二维码都会用它.
    pub async fn local_fingerprint(&self) -> Result<String, String> {
        let guard = self.manager.lock().await;
        match guard.as_ref() {
            Some(m) => Ok(m.fingerprint()),
            None => {
                // Manager 未启动时, 从 vault 中取出 Noise 静态密钥直接推导指纹.
                let svc = self
                    .vault_service
                    .read()
                    .map_err(|_| "Vault service lock poisoned".to_string())?;
                let vault = svc.get_vault_store().ok_or("Vault is not unlocked")?;
                let (_node_id, keys) = get_or_create_sync_identity(&vault)?;
                Ok(keys.fingerprint())
            }
        }
    }
}

// -----------------------------------------------------------------------------
// 私有辅助函数: 审计日志 + 同步身份生成.
// -----------------------------------------------------------------------------

fn audit_log(
    vault: &solosoul_vault::VaultStore,
    action: &str,
    entity_id: Option<&str>,
    details: Option<&str>,
) {
    let _ = vault.log_structured(action, "sync", entity_id, None, "user", details);
}

fn get_or_create_sync_identity(
    vault: &solosoul_vault::VaultStore,
) -> Result<(String, NoiseKeys), String> {
    let node_id = match vault.get_sync_node_id()? {
        Some(id) => id,
        None => {
            let id = format!("node_{}", uuid::Uuid::new_v4().to_string().replace("-", ""));
            vault.set_sync_node_id(&id)?;
            id
        }
    };
    let keys = match vault.get_sync_secret_key()? {
        Some(secret) => NoiseKeys::from_secret(secret),
        None => {
            let keys = NoiseKeys::generate();
            vault.set_sync_secret_key(keys.secret_key())?;
            keys
        }
    };
    Ok((node_id, keys))
}

// -----------------------------------------------------------------------------
// 单元测试: 锁定 vault 的最小 happy-path + not-running 错误回退.
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn fresh_service() -> (SyncService, tempfile::TempDir) {
        let dir = tempdir().expect("tempdir");
        let vault = VaultService::new();
        let svc = SyncService::new(Arc::new(std::sync::RwLock::new(vault)));
        (svc, dir)
    }

    #[tokio::test]
    async fn fresh_service_is_not_enabled() {
        let (svc, _dir) = fresh_service();
        assert!(!svc.is_enabled().await);
    }

    #[tokio::test]
    async fn fresh_service_sync_with_device_returns_err() {
        let (svc, _dir) = fresh_service();
        let r = svc.sync_with_device("127.0.0.1:12345".to_string()).await;
        assert!(r.is_err());
    }

    /// 回归测试：`enable(false)` 不应在 `stop()` 等待活跃会话时阻塞 async 命令线程。
    /// 修复前 stop() 会在持有 manager 锁的情况下最多同步等待 30s，
    /// 导致前端禁用同步超时、所有按钮卡住。
    #[tokio::test]
    async fn disable_returns_promptly_even_with_active_sessions() {
        use solosoul_vault::{VaultConfig, VaultStore};
        use std::sync::atomic::Ordering;

        let dir = tempdir().expect("tempdir");
        let vault = Arc::new(
            VaultStore::open(VaultConfig {
                path: dir.path().to_path_buf(),
                account_id: "acct".to_string(),
                data_key: Some([0u8; 32]),
            })
            .expect("open vault"),
        );
        let keys = NoiseKeys::generate();
        let manager = SyncManager::new(
            "node_test".to_string(),
            "acct".to_string(),
            keys,
            vault,
            "0.0.0.0:0",
        );
        // 模拟存在活跃同步会话：修复前的 stop() 会同步等待其结束（最长 30s）
        let active_sessions = manager.active_sessions_counter();
        manager.set_active_sessions_for_test(1);

        let (svc, _dir2) = fresh_service();
        {
            let mut guard = svc.manager.lock().await;
            *guard = Some(Arc::new(manager));
        }

        let start = std::time::Instant::now();
        let result = svc.enable(false).await;
        let elapsed = start.elapsed();

        assert!(result.is_ok(), "enable(false) 应成功: {:?}", result.err());
        // 修复前此调用会阻塞最多 30s；修复后应立即返回
        assert!(
            elapsed.as_secs() < 5,
            "enable(false) 阻塞了 {:.1}s，manager 锁未及时释放",
            elapsed.as_secs_f32()
        );
        assert!(!svc.is_enabled().await, "禁用后 is_enabled 应为 false");

        // 复位活跃会话计数，让后台 stop() 尽快结束，避免测试收尾等待
        active_sessions.store(0, Ordering::SeqCst);
    }

    /// 回归测试：`sync_with_device` 会话进行中不应持有 manager 锁，
    /// `is_enabled()` / `enable(false)` 应立即返回（与移动端 round 2 修复对齐）。
    /// 修复前 guard 会跨整个会话（连接超时 10s + 数据交换）持有，
    /// 导致同步进行中点“禁用”时前端 15s 超时失败。
    #[tokio::test]
    async fn sync_with_device_does_not_hold_manager_lock() {
        use solosoul_vault::{VaultConfig, VaultStore};

        let dir = tempdir().expect("tempdir");
        let vault = Arc::new(
            VaultStore::open(VaultConfig {
                path: dir.path().to_path_buf(),
                account_id: "acct".to_string(),
                data_key: Some([0u8; 32]),
            })
            .expect("open vault"),
        );
        let manager = SyncManager::new(
            "node_test".to_string(),
            "acct".to_string(),
            NoiseKeys::generate(),
            vault,
            "0.0.0.0:0",
        );

        let (svc, _dir2) = fresh_service();
        let svc = Arc::new(svc);
        {
            let mut guard = svc.manager.lock().await;
            *guard = Some(Arc::new(manager));
        }

        // 会话目标不可达（TEST-NET-1，RFC 5737），连接最长 10s 后才失败，
        // 期间若持有 manager 锁，下面的调用会被阻塞。
        let svc2 = svc.clone();
        let sync_task =
            tokio::spawn(async move { svc2.sync_with_device("192.0.2.1:9".to_string()).await });

        // 给会话一点时间进入 connect 阶段
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        let start = std::time::Instant::now();
        assert!(svc.is_enabled().await);
        assert!(
            start.elapsed().as_secs() < 5,
            "is_enabled() 被会话阻塞了 {:.1}s",
            start.elapsed().as_secs_f32()
        );

        let start = std::time::Instant::now();
        svc.enable(false).await.expect("enable(false) 应成功");
        assert!(
            start.elapsed().as_secs() < 5,
            "enable(false) 被会话阻塞了 {:.1}s",
            start.elapsed().as_secs()
        );

        sync_task.abort();
    }
}
