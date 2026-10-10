//! 更新源偏好：账户内加密保存，登录前使用不含账户信息的本机缓存。
//! 偏好只调整已允许候选的顺序，不能向请求列表注入地址。

use super::settings::{resolve_ui_prefs_path, UI_PREFS_LOCK};
use crate::state::AppState;
use futures::{future::BoxFuture, stream::FuturesUnordered, StreamExt};
use serde::{Deserialize, Serialize};
use solosoul_vault::VaultStore;
use std::{
    path::PathBuf,
    sync::{Arc, RwLock, Weak},
    time::Duration,
};
use tauri::Manager;

const PREF_KEY: &str = "updateSources";
const REPROBE_SECS: i64 = 6 * 60 * 60;
const PREFERRED_GRACE: Duration = Duration::from_millis(900);

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) enum Channel {
    Manifest,
    Release,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PreferredSource {
    url: String,
    probed_at: i64,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct Preferences {
    manifest: Option<PreferredSource>,
    release: Option<PreferredSource>,
    last_channel: Option<Channel>,
}

impl Preferences {
    fn source(&self, channel: Channel) -> Option<&PreferredSource> {
        match channel {
            Channel::Manifest => self.manifest.as_ref(),
            Channel::Release => self.release.as_ref(),
        }
    }

    fn remember(&mut self, channel: Channel, url: &str, full_probe: bool) {
        let slot = match channel {
            Channel::Manifest => &mut self.manifest,
            Channel::Release => &mut self.release,
        };
        // 快速路径不延长探测有效期，避免一直信任某个陈旧镜像。
        if full_probe || slot.as_ref().is_none_or(|source| source.url != url) {
            *slot = Some(PreferredSource {
                url: url.into(),
                probed_at: chrono::Utc::now().timestamp(),
            });
        }
        self.last_channel = Some(channel);
    }
}

struct AccountSnapshot {
    id: String,
    generation: u64,
    vault: Weak<VaultStore>,
}

pub(super) struct SourcePreferences {
    preferences: Preferences,
    cache: Option<PathBuf>,
    account: Option<AccountSnapshot>,
    account_value: Option<serde_json::Value>,
    // 网络等待不保活 Store、目录或活动许可；返回后重新验证原身份。
    service: Weak<RwLock<solosoul_core::VaultService>>,
    owner: Weak<solosoul_vault::root_owner::VaultRootOwner>,
}

fn preference_activity(
    owner: Arc<solosoul_vault::root_owner::VaultRootOwner>,
) -> Result<Arc<solosoul_core::import_activity::RootActivityGuard>, String> {
    let activity = Arc::new(solosoul_core::import_activity::begin_owned_root_activity(
        owner,
    )?);
    #[cfg(all(feature = "native-perf", target_os = "windows"))]
    crate::native_perf::auth_trace::register_activity(
        crate::native_perf::maintenance_trace::ActivityKind::UpdateSourcePreferences,
        &activity,
    );
    Ok(activity)
}

impl SourcePreferences {
    pub fn load(app: &tauri::AppHandle) -> Self {
        let Some(state) = app.try_state::<AppState>() else {
            return Self::empty();
        };
        Self::load_for_service(&state.vault_service, |svc| resolve_ui_prefs_path(app, svc))
    }

    fn empty() -> Self {
        Self {
            preferences: Preferences::default(),
            cache: None,
            account: None,
            account_value: None,
            service: Weak::new(),
            owner: Weak::new(),
        }
    }

    /// 读取期间持许可；维护忙或会话失效时不提供无许可缓存写入退路。
    fn load_for_service(
        service: &Arc<RwLock<solosoul_core::VaultService>>,
        resolve_cache: impl FnOnce(&solosoul_core::VaultService) -> Result<PathBuf, String>,
    ) -> Self {
        let Ok(svc) = service.read() else {
            return Self::empty();
        };
        let owner = svc.root_owner();
        let Ok(_activity) = preference_activity(Arc::clone(&owner)) else {
            return Self::empty();
        };
        let mut result = Self::empty();
        result.cache = resolve_cache(&svc).ok();
        let stored = if let Some(id) = svc.get_current_account() {
            let Ok(session) = svc.capture_session(&id) else {
                return Self::empty();
            };
            let Some(vault) = svc.get_vault_store() else {
                return Self::empty();
            };
            if !std::ptr::eq(vault.as_ref(), session.vault()) {
                return Self::empty();
            }
            let Ok(stored) = svc.with_session(&session, |vault| {
                Ok(vault.load_profile(&id).ok().flatten().and_then(|profile| {
                    serde_json::from_slice::<serde_json::Value>(&profile.data)
                        .ok()?
                        .get("preferences")?
                        .get(PREF_KEY)
                        .cloned()
                }))
            }) else {
                return Self::empty();
            };
            result.account = Some(AccountSnapshot {
                id,
                generation: session.generation(),
                vault: Arc::downgrade(&vault),
            });
            stored
        } else {
            None
        };
        let cached = || {
            let _guard = UI_PREFS_LOCK.lock().ok()?;
            let bytes = std::fs::read(result.cache.as_ref()?).ok()?;
            serde_json::from_slice::<serde_json::Value>(&bytes)
                .ok()?
                .get(PREF_KEY)
                .cloned()
        };
        result.account_value = stored.clone();
        result.preferences = stored
            .or_else(cached)
            .and_then(|value| serde_json::from_value(value).ok())
            .unwrap_or_default();
        result.service = Arc::downgrade(service);
        result.owner = Arc::downgrade(&owner);
        result
    }

    pub fn preferred<'a>(&'a self, channel: Channel, candidates: &[String]) -> Option<&'a str> {
        let source = self.preferences.source(channel)?;
        let age = chrono::Utc::now().timestamp() - source.probed_at;
        ((0..REPROBE_SECS).contains(&age) && candidates.contains(&source.url))
            .then_some(source.url.as_str())
    }

    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    pub fn prefers_release(&self) -> bool {
        self.preferences.last_channel == Some(Channel::Release)
            && self.preferences.release.as_ref().is_some_and(|source| {
                let age = chrono::Utc::now().timestamp() - source.probed_at;
                (0..REPROBE_SECS).contains(&age)
            })
    }

    pub fn remember(&self, channel: Channel, url: &str, full_probe: bool) {
        let (Some(service), Some(owner)) = (self.service.upgrade(), self.owner.upgrade()) else {
            return;
        };
        let Ok(svc) = service.read() else {
            return;
        };
        if !Arc::ptr_eq(&svc.root_owner(), &owner) {
            return;
        }
        let Ok(_activity) = preference_activity(owner) else {
            return;
        };
        if let Some(account) = &self.account {
            let Some(original) = account.vault.upgrade() else {
                return;
            };
            let Ok(session) = svc.capture_session(&account.id) else {
                return;
            };
            if session.generation() != account.generation
                || !std::ptr::eq(session.vault(), original.as_ref())
            {
                return;
            }
            // 核对与实际同步写入在同一会话门闩内；锁定不能插入两者之间。
            if let Err(error) = svc.with_session(&session, |vault| {
                self.persist(Some((vault, &account.id)), channel, url, full_probe);
                Ok(())
            }) {
                tracing::warn!("[updater] 更新源偏好会话已失效: {error}");
            }
        } else {
            // 登录前只更新同一目录的公共缓存，不能写入后来登录的账户。
            self.persist(None, channel, url, full_probe);
        }
    }

    fn persist(
        &self,
        account: Option<(&VaultStore, &str)>,
        channel: Channel,
        url: &str,
        full_probe: bool,
    ) {
        // 两种清单可能并发完成，必须按键合并最新配置，不能覆盖另一通道。
        if let Some((vault, id)) = account {
            let mut next = self
                .account_value
                .clone()
                .unwrap_or_else(|| serde_json::json!({}));
            let unchanged = merge_preference(&mut next, channel, url, full_probe).is_ok()
                && self.account_value.as_ref() == Some(&next);
            // 快速路径复用同一个来源时无需推进 Profile 的版本/HLC。
            if !unchanged {
                if let Err(error) = vault.update_profile_prefs(id, |prefs| {
                    let value = prefs
                        .entry(PREF_KEY)
                        .or_insert_with(|| serde_json::json!({}));
                    merge_preference(value, channel, url, full_probe)?;
                    Ok(())
                }) {
                    tracing::warn!("[updater] 保存账户更新源偏好失败: {error}");
                }
            }
        }
        if let Some(path) = &self.cache {
            let result = (|| -> Result<(), String> {
                let _guard = UI_PREFS_LOCK.lock().map_err(|e| e.to_string())?;
                let mut prefs: serde_json::Value = match std::fs::read(path) {
                    Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| e.to_string())?,
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => serde_json::json!({}),
                    Err(e) => return Err(e.to_string()),
                };
                let prefs_obj = prefs.as_object_mut().ok_or("Invalid UI preferences")?;
                let value = prefs_obj
                    .entry(PREF_KEY)
                    .or_insert_with(|| serde_json::json!({}));
                let previous = value.clone();
                merge_preference(value, channel, url, full_probe)?;
                if *value == previous {
                    return Ok(());
                }
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                let temp = path.with_extension("update-source.tmp");
                std::fs::write(
                    &temp,
                    serde_json::to_vec(&prefs).map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?;
                std::fs::rename(temp, path).map_err(|e| e.to_string())
            })();
            if let Err(error) = result {
                tracing::warn!("[updater] 保存本机更新源偏好失败: {error}");
            }
        }
    }
}

fn merge_preference(
    value: &mut serde_json::Value,
    channel: Channel,
    url: &str,
    full_probe: bool,
) -> Result<(), String> {
    let mut prefs: Preferences = serde_json::from_value(value.clone()).unwrap_or_default();
    prefs.remember(channel, url, full_probe);
    *value = serde_json::to_value(prefs).map_err(|e| e.to_string())?;
    Ok(())
}

pub(super) struct Metadata<T> {
    pub value: T,
    pub version: semver::Version,
}

pub(super) struct Selected<T> {
    pub metadata: Metadata<T>,
    pub url: String,
    pub full_probe: bool,
}

type PendingSource<'a, T> = BoxFuture<'a, (&'a str, Result<Metadata<T>, String>)>;

/// 优先源有 900ms 独占窗口；失败立即换源，超时保留在途请求并错峰探测其他源。
/// 完整探测沿用「发现新版本即返回」，无新版本时选最高版本，避免旧镜像覆盖新响应。
pub(super) async fn select_source<'a, T: Send + 'a>(
    candidates: &'a [String],
    preferred: Option<&str>,
    current: &semver::Version,
    fetch: impl Fn(&'a str) -> BoxFuture<'a, Result<Metadata<T>, String>>,
) -> Result<Selected<T>, String> {
    let mut pending: FuturesUnordered<PendingSource<'a, T>> = FuturesUnordered::new();
    let mut fallback: Option<Selected<T>> = None;
    let mut last_error = "未配置更新源".to_string();
    let preferred = preferred.and_then(|saved| candidates.iter().find(|url| url.as_str() == saved));
    if let Some(url) = preferred {
        let host = url::Url::parse(url)
            .ok()
            .and_then(|url| url.host_str().map(str::to_owned))
            .unwrap_or_default();
        tracing::info!("[updater] 优先使用上次成功的更新源：{}", host);
        let mut first = fetch(url);
        match tokio::time::timeout(PREFERRED_GRACE, &mut first).await {
            Ok(Ok(metadata)) if metadata.version >= *current => {
                return Ok(Selected {
                    metadata,
                    url: url.clone(),
                    full_probe: false,
                });
            }
            Ok(Ok(metadata)) => {
                fallback = Some(Selected {
                    metadata,
                    url: url.clone(),
                    full_probe: true,
                })
            }
            Ok(Err(error)) => last_error = error,
            Err(_) => pending.push(Box::pin(async move { (url.as_str(), first.await) })),
        }
        tracing::info!("[updater] 优先源未及时提供可用版本，正在探测其他更新源");
    }
    for (index, url) in candidates
        .iter()
        .filter(|url| Some(*url) != preferred)
        .enumerate()
    {
        let future = fetch(url);
        pending.push(Box::pin(async move {
            tokio::time::sleep(Duration::from_millis(index.min(8) as u64 * 350)).await;
            (url.as_str(), future.await)
        }));
    }
    while let Some((url, result)) = pending.next().await {
        match result {
            Ok(metadata) => {
                let selected = Selected {
                    metadata,
                    url: url.into(),
                    full_probe: true,
                };
                if selected.metadata.version > *current {
                    return Ok(selected);
                }
                if fallback
                    .as_ref()
                    .is_none_or(|old| selected.metadata.version > old.metadata.version)
                {
                    fallback = Some(selected);
                }
            }
            Err(error) => last_error = error,
        }
    }
    fallback.ok_or_else(|| format!("所有更新源均不可用: {last_error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[tokio::test]
    async fn rf1104_pending_source_selection_does_not_block_password_reunlock() {
        let dir = tempfile::tempdir().unwrap();
        let service = Arc::new(RwLock::new(
            solosoul_core::VaultService::try_with_base_path(dir.path().join("vault")).unwrap(),
        ));
        let account = "acc_rf1104_pending";
        service
            .read()
            .unwrap()
            .create_account_with_id(account, "Test", "password123", None)
            .unwrap();
        let owner = service.read().unwrap().root_owner();
        let cache = dir.path().join("ui_preferences.json");
        let before = br#"{"theme":"dark"}"#;
        std::fs::write(&cache, before).unwrap();
        let prefs = SourcePreferences::load_for_service(&service, |_| Ok(cache.clone()));
        // 原 Store 仍存活时也必须拒绝同账户的新会话，不能只依赖弱引用过期。
        let original = service.read().unwrap().get_vault_store().unwrap();
        let entered = Arc::new(tokio::sync::Notify::new());
        let release = Arc::new(tokio::sync::Notify::new());
        let observed = entered.clone();
        let response = release.clone();
        let writer = tokio::spawn(async move {
            let urls = vec!["https://mirror.example/latest.json".into()];
            let selected = select_source(&urls, None, &"2.13.1".parse().unwrap(), |_| {
                observed.notify_one();
                let response = response.clone();
                Box::pin(async move {
                    response.notified().await;
                    Ok(metadata("2.13.2"))
                })
            })
            .await
            .unwrap();
            prefs.remember(Channel::Manifest, &selected.url, selected.full_probe);
        });
        entered.notified().await;
        service.read().unwrap().lock();
        let maintenance = solosoul_core::import_activity::begin_owned_root_maintenance(owner);
        assert!(
            maintenance.is_ok(),
            "network-only source selection must not retain the root activity permit: {:?}",
            maintenance.err()
        );
        let maintenance = maintenance.unwrap();
        service
            .read()
            .unwrap()
            .unlock_secure_with_maintenance(
                account,
                &zeroize::Zeroizing::new("password123".to_owned()),
                &maintenance,
            )
            .unwrap();
        drop(maintenance);
        release.notify_one();
        writer.await.unwrap();
        assert!(!Arc::ptr_eq(
            &original,
            &service.read().unwrap().get_vault_store().unwrap()
        ));
        assert_eq!(std::fs::read(cache).unwrap(), before);
        let vault = service.read().unwrap().get_vault_store().unwrap();
        assert!(vault
            .load_profile(account)
            .unwrap()
            .and_then(|profile| serde_json::from_slice::<serde_json::Value>(&profile.data).ok())
            .is_none_or(|value| value["preferences"][PREF_KEY].is_null()));
    }

    fn metadata(version: &str) -> Metadata<()> {
        Metadata {
            value: (),
            version: version.parse().unwrap(),
        }
    }

    #[tokio::test]
    async fn successful_preferred_source_skips_other_requests() {
        let urls = vec!["direct".into(), "mirror".into()];
        let calls = Mutex::new(Vec::new());
        let selected = select_source(&urls, Some("mirror"), &"2.13.1".parse().unwrap(), |url| {
            calls.lock().unwrap().push(url);
            Box::pin(async { Ok(metadata("2.13.1")) })
        })
        .await
        .unwrap();
        assert_eq!(selected.url, "mirror");
        assert!(!selected.full_probe);
        assert_eq!(*calls.lock().unwrap(), vec!["mirror"]);
    }

    #[tokio::test]
    async fn failed_or_stale_preferred_source_cannot_block_new_version() {
        for stale in [false, true] {
            let urls = vec!["direct".into(), "mirror".into()];
            let selected =
                select_source(&urls, Some("mirror"), &"2.13.1".parse().unwrap(), |url| {
                    Box::pin(async move {
                        if url == "direct" {
                            Ok(metadata("2.13.2"))
                        } else if stale {
                            Ok(metadata("2.12.0"))
                        } else {
                            Err("offline".into())
                        }
                    })
                })
                .await
                .unwrap();
            assert_eq!(selected.url, "direct");
            assert!(selected.full_probe);
        }
    }

    #[tokio::test]
    async fn stalled_preferred_source_has_bounded_head_start() {
        let urls = vec!["direct".into(), "mirror".into()];
        let selected = tokio::time::timeout(
            Duration::from_secs(2),
            select_source(&urls, Some("mirror"), &"2.13.1".parse().unwrap(), |url| {
                Box::pin(async move {
                    if url == "mirror" {
                        std::future::pending().await
                    } else {
                        Ok(metadata("2.13.2"))
                    }
                })
            }),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(selected.url, "direct");
    }

    #[test]
    fn expired_and_disallowed_sources_are_not_prioritized() {
        let mut prefs = SourcePreferences::empty();
        prefs.preferences.remember(
            Channel::Manifest,
            "https://mirror.example/latest.json",
            true,
        );
        let allowed = vec!["https://mirror.example/latest.json".into()];
        assert!(prefs.preferred(Channel::Manifest, &allowed).is_some());
        assert!(prefs
            .preferred(
                Channel::Manifest,
                &["https://direct.example/latest.json".into()]
            )
            .is_none());
        let source = prefs.preferences.manifest.as_mut().unwrap();
        source.probed_at -= REPROBE_SECS + 1;
        let old_timestamp = source.probed_at;
        prefs
            .preferences
            .remember(Channel::Manifest, &allowed[0], false);
        assert_eq!(
            prefs.preferences.manifest.as_ref().unwrap().probed_at,
            old_timestamp
        );
        assert!(prefs.preferred(Channel::Manifest, &allowed).is_none());
    }

    #[test]
    fn preferences_survive_reopen_and_preserve_other_settings_and_account_isolation() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("vault");
        let cache = dir.path().join("ui_preferences.json");
        std::fs::write(&cache, r#"{"theme":"dark"}"#).unwrap();
        let service = Arc::new(RwLock::new(
            solosoul_core::VaultService::try_with_base_path(root.clone()).unwrap(),
        ));
        let account = "acc_prefs_reopen";
        service
            .read()
            .unwrap()
            .create_account_with_id(account, "Test", "password123", None)
            .unwrap();
        let prefs = SourcePreferences::load_for_service(&service, |_| Ok(cache.clone()));
        prefs.remember(
            Channel::Manifest,
            "https://mirror.example/latest.json",
            true,
        );
        prefs.remember(Channel::Release, "https://other.example/release.json", true);
        let vault = service.read().unwrap().get_vault_store().unwrap();
        let before = vault.load_profile(account).unwrap().unwrap();
        let unchanged = SourcePreferences::load_for_service(&service, |_| Ok(cache.clone()));
        unchanged.remember(
            Channel::Release,
            "https://other.example/release.json",
            false,
        );
        assert_eq!(
            vault.load_profile(account).unwrap().unwrap().version,
            before.version
        );
        drop(vault);
        service.read().unwrap().lock();
        drop(service);
        let reopened = solosoul_core::VaultService::try_with_base_path(root).unwrap();
        reopened.unlock(account, "password123").unwrap();
        let vault = reopened.get_vault_store().unwrap();
        let profile = vault.load_profile(account).unwrap().unwrap();
        let data: serde_json::Value = serde_json::from_slice(&profile.data).unwrap();
        assert_eq!(
            data["preferences"][PREF_KEY]["manifest"]["url"],
            "https://mirror.example/latest.json"
        );
        assert_eq!(
            data["preferences"][PREF_KEY]["release"]["url"],
            "https://other.example/release.json"
        );
        assert!(vault.load_profile("account-b").unwrap().is_none());
        let cached: serde_json::Value =
            serde_json::from_slice(&std::fs::read(cache).unwrap()).unwrap();
        assert_eq!(cached["theme"], "dark");
        assert!(!cached.to_string().contains(account));
    }

    #[test]
    fn rf905_locked_source_preferences_preserve_global_cache_with_owned_admission() {
        let dir = tempfile::tempdir().unwrap();
        let service = Arc::new(RwLock::new(
            solosoul_core::VaultService::try_with_base_path(dir.path().join("vault")).unwrap(),
        ));
        let owner = service.read().unwrap().root_owner();
        let cache = dir.path().join("ui_preferences.json");
        std::fs::write(&cache, br#"{"theme":"dark"}"#).unwrap();
        let prefs = SourcePreferences::load_for_service(&service, |_| Ok(cache.clone()));
        assert!(prefs.account.is_none());
        prefs.remember(
            Channel::Manifest,
            "https://mirror.example/latest.json",
            true,
        );
        let saved: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&cache).unwrap()).unwrap();
        assert_eq!(saved["theme"], "dark");
        assert_eq!(
            saved[PREF_KEY]["manifest"]["url"],
            "https://mirror.example/latest.json"
        );
        let during_network =
            solosoul_core::import_activity::begin_owned_root_maintenance(Arc::clone(&owner))
                .unwrap();
        drop(during_network);
        drop(prefs);
        let maintenance =
            solosoul_core::import_activity::begin_owned_root_maintenance(owner).unwrap();
        drop(maintenance);
    }

    #[test]
    fn rf905_busy_preferences_never_fallback_to_unowned_cache_or_old_store() {
        let dir = tempfile::tempdir().unwrap();
        let service = Arc::new(RwLock::new(
            solosoul_core::VaultService::try_with_base_path(dir.path().join("vault")).unwrap(),
        ));
        service
            .read()
            .unwrap()
            .create_account_with_id("acc_rf905_prefs", "Test", "password123", None)
            .unwrap();
        let owner = service.read().unwrap().root_owner();
        let cache = dir.path().join("ui_preferences.json");
        let before = br#"{"theme":"dark"}"#;
        std::fs::write(&cache, before).unwrap();
        let maintenance =
            solosoul_core::import_activity::begin_owned_root_maintenance(owner).unwrap();
        let prefs = SourcePreferences::load_for_service(&service, |_| {
            panic!("must reject before resolving fallback cache")
        });
        assert!(prefs.account.is_none());
        assert!(prefs.cache.is_none());
        prefs.remember(
            Channel::Manifest,
            "https://mirror.example/latest.json",
            true,
        );
        assert_eq!(std::fs::read(cache).unwrap(), before);
        let vault = service.read().unwrap().get_vault_store().unwrap();
        assert!(vault
            .load_profile("acc_rf905_prefs")
            .unwrap()
            .and_then(|profile| serde_json::from_slice::<serde_json::Value>(&profile.data).ok())
            .is_none_or(|value| value["preferences"][PREF_KEY].is_null()));
        drop(vault);
        drop(maintenance);
    }

    #[tokio::test]
    async fn rf905_network_delayed_preferences_reacquire_original_store_for_real_write() {
        let dir = tempfile::tempdir().unwrap();
        let service = Arc::new(RwLock::new(
            solosoul_core::VaultService::try_with_base_path(dir.path().join("vault")).unwrap(),
        ));
        service
            .read()
            .unwrap()
            .create_account_with_id("acc_rf905_prefs", "Test", "password123", None)
            .unwrap();
        let owner = service.read().unwrap().root_owner();
        let cache = dir.path().join("ui_preferences.json");
        let prefs = SourcePreferences::load_for_service(&service, |_| Ok(cache.clone()));
        let (go_tx, go_rx) = tokio::sync::oneshot::channel();
        let writer = tokio::spawn(async move {
            go_rx.await.unwrap();
            prefs.remember(
                Channel::Release,
                "https://mirror.example/release.json",
                true,
            );
        });
        let during_network =
            solosoul_core::import_activity::begin_owned_root_maintenance(Arc::clone(&owner))
                .unwrap();
        drop(during_network);
        go_tx.send(()).unwrap();
        writer.await.unwrap();
        let maintenance =
            solosoul_core::import_activity::begin_owned_root_maintenance(owner).unwrap();
        let vault = service.read().unwrap().get_vault_store().unwrap();
        let profile = vault.load_profile("acc_rf905_prefs").unwrap().unwrap();
        let stored: serde_json::Value = serde_json::from_slice(&profile.data).unwrap();
        assert_eq!(
            stored["preferences"][PREF_KEY]["release"]["url"],
            "https://mirror.example/release.json"
        );
        let cached: serde_json::Value =
            serde_json::from_slice(&std::fs::read(cache).unwrap()).unwrap();
        assert_eq!(
            cached[PREF_KEY]["release"]["url"],
            "https://mirror.example/release.json"
        );
        drop(vault);
        drop(maintenance);
    }
    fn locked_service(root: PathBuf) -> Arc<RwLock<solosoul_core::VaultService>> {
        Arc::new(RwLock::new(
            solosoul_core::VaultService::try_with_base_path(root).unwrap(),
        ))
    }

    #[test]
    fn rf1104_pending_cache_does_not_keep_service_or_directory_owned() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("vault");
        let cache = dir.path().join("ui_preferences.json");
        let service = locked_service(root.clone());
        let owner = Arc::downgrade(&service.read().unwrap().root_owner());
        let prefs = SourcePreferences::load_for_service(&service, |_| Ok(cache.clone()));
        drop(service);
        assert!(owner.upgrade().is_none());
        let reopened = solosoul_vault::root_owner::VaultRootOwner::acquire(&root).unwrap();
        prefs.remember(
            Channel::Manifest,
            "https://mirror.example/latest.json",
            true,
        );
        assert!(!cache.exists());
        drop(reopened);
    }

    #[test]
    fn rf1104_late_cache_write_rejects_maintenance_busy_and_replaced_root() {
        let dir = tempfile::tempdir().unwrap();
        let service = locked_service(dir.path().join("vault"));
        let owner = service.read().unwrap().root_owner();
        let cache = dir.path().join("ui_preferences.json");
        let before = br#"{"theme":"dark"}"#;
        std::fs::write(&cache, before).unwrap();
        let prefs = SourcePreferences::load_for_service(&service, |_| Ok(cache.clone()));
        let maintenance =
            solosoul_core::import_activity::begin_owned_root_maintenance(owner.clone()).unwrap();
        prefs.remember(
            Channel::Manifest,
            "https://mirror.example/latest.json",
            true,
        );
        assert_eq!(std::fs::read(&cache).unwrap(), before);
        drop(maintenance);
        *service.write().unwrap() =
            solosoul_core::VaultService::try_with_base_path(dir.path().join("other")).unwrap();
        // 保留原 owner，确保拒绝来自身份比较而非弱引用提前过期。
        assert!(prefs.owner.upgrade().is_some());
        prefs.remember(
            Channel::Manifest,
            "https://mirror.example/latest.json",
            true,
        );
        assert_eq!(std::fs::read(cache).unwrap(), before);
        drop(owner);
    }

    #[test]
    fn rf1104_same_path_replacement_cannot_revive_old_cache_request() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("vault");
        let service = locked_service(root.clone());
        let cache = dir.path().join("ui_preferences.json");
        let prefs = SourcePreferences::load_for_service(&service, |_| Ok(cache.clone()));
        *service.write().unwrap() =
            solosoul_core::VaultService::try_with_base_path(dir.path().join("other")).unwrap();
        *service.write().unwrap() = solosoul_core::VaultService::try_with_base_path(root).unwrap();
        prefs.remember(
            Channel::Manifest,
            "https://mirror.example/latest.json",
            true,
        );
        assert!(!cache.exists());
    }

    #[test]
    fn rf1104_account_switch_rejects_late_account_and_cache_write() {
        let dir = tempfile::tempdir().unwrap();
        let service = locked_service(dir.path().join("vault"));
        let cache = dir.path().join("ui_preferences.json");
        let before = br#"{"theme":"dark"}"#;
        std::fs::write(&cache, before).unwrap();
        service
            .read()
            .unwrap()
            .create_account_with_id("acc_prefs_a", "A", "password123", None)
            .unwrap();
        let prefs = SourcePreferences::load_for_service(&service, |_| Ok(cache.clone()));
        let before_a = service
            .read()
            .unwrap()
            .get_vault_store()
            .unwrap()
            .load_profile("acc_prefs_a")
            .unwrap()
            .map(|profile| (profile.version, profile.data));
        service
            .read()
            .unwrap()
            .create_account_with_id("acc_prefs_b", "B", "password123", None)
            .unwrap();
        let before_b = service
            .read()
            .unwrap()
            .get_vault_store()
            .unwrap()
            .load_profile("acc_prefs_b")
            .unwrap()
            .map(|profile| (profile.version, profile.data));
        prefs.remember(
            Channel::Manifest,
            "https://mirror.example/latest.json",
            true,
        );
        assert_eq!(std::fs::read(cache).unwrap(), before);
        for (account, expected) in [("acc_prefs_b", before_b), ("acc_prefs_a", before_a)] {
            service.read().unwrap().lock();
            service
                .read()
                .unwrap()
                .unlock(account, "password123")
                .unwrap();
            let vault = service.read().unwrap().get_vault_store().unwrap();
            assert_eq!(
                vault
                    .load_profile(account)
                    .unwrap()
                    .map(|profile| (profile.version, profile.data)),
                expected
            );
        }
    }

    #[test]
    fn rf1104_concurrent_source_channels_preserve_account_and_cache_preferences() {
        let dir = tempfile::tempdir().unwrap();
        let service = locked_service(dir.path().join("vault"));
        let cache = dir.path().join("ui_preferences.json");
        std::fs::write(&cache, br#"{"theme":"dark"}"#).unwrap();
        let account = "acc_prefs_concurrent";
        service
            .read()
            .unwrap()
            .create_account_with_id(account, "Test", "password123", None)
            .unwrap();
        let manifest = SourcePreferences::load_for_service(&service, |_| Ok(cache.clone()));
        let release = SourcePreferences::load_for_service(&service, |_| Ok(cache.clone()));
        let ready = Arc::new(std::sync::Barrier::new(3));
        let writers = [
            (
                manifest,
                Channel::Manifest,
                "https://mirror.example/latest.json",
            ),
            (
                release,
                Channel::Release,
                "https://mirror.example/release.json",
            ),
        ]
        .into_iter()
        .map(|(prefs, channel, url)| {
            let ready = ready.clone();
            std::thread::spawn(move || {
                ready.wait();
                prefs.remember(channel, url, true);
            })
        })
        .collect::<Vec<_>>();
        ready.wait();
        for writer in writers {
            writer.join().unwrap();
        }
        let vault = service.read().unwrap().get_vault_store().unwrap();
        let profile = vault.load_profile(account).unwrap().unwrap();
        let stored: serde_json::Value = serde_json::from_slice(&profile.data).unwrap();
        let cached: serde_json::Value =
            serde_json::from_slice(&std::fs::read(cache).unwrap()).unwrap();
        for value in [&stored["preferences"], &cached] {
            assert_eq!(
                value[PREF_KEY]["manifest"]["url"],
                "https://mirror.example/latest.json"
            );
            assert_eq!(
                value[PREF_KEY]["release"]["url"],
                "https://mirror.example/release.json"
            );
        }
        assert_eq!(cached["theme"], "dark");
        assert!(!cached.to_string().contains(account));
    }

    #[test]
    fn rf1104_actual_cache_write_retains_activity_until_file_work_finishes() {
        let dir = tempfile::tempdir().unwrap();
        let service = locked_service(dir.path().join("vault"));
        let owner = service.read().unwrap().root_owner();
        let cache = dir.path().join("ui_preferences.json");
        let before = br#"{"theme":"dark"}"#;
        std::fs::write(&cache, before).unwrap();
        let prefs = SourcePreferences::load_for_service(&service, |_| Ok(cache.clone()));
        drop(solosoul_core::import_activity::begin_owned_root_maintenance(owner.clone()).unwrap());
        let file_lock = UI_PREFS_LOCK.lock().unwrap();
        let writer = std::thread::spawn(move || {
            prefs.remember(
                Channel::Manifest,
                "https://mirror.example/latest.json",
                true,
            );
        });
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        // 只读观察实际 worker 保活，避免轮询维护许可抢在 worker 前占用准入。
        while Arc::strong_count(&owner) == 2 && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        std::thread::sleep(Duration::from_millis(10));
        let blocked = solosoul_core::import_activity::begin_owned_root_maintenance(owner.clone())
            .err()
            .as_deref()
            == Some("IMPORT_OPERATIONS_ACTIVE");
        let while_blocked = std::fs::read(&cache).unwrap();
        drop(file_lock);
        writer.join().unwrap();
        assert!(
            blocked,
            "actual file work must retain its own root activity permit"
        );
        assert_eq!(while_blocked, before);
        let saved: serde_json::Value =
            serde_json::from_slice(&std::fs::read(cache).unwrap()).unwrap();
        assert_eq!(saved["theme"], "dark");
        assert_eq!(
            saved[PREF_KEY]["manifest"]["url"],
            "https://mirror.example/latest.json"
        );
        drop(solosoul_core::import_activity::begin_owned_root_maintenance(owner).unwrap());
    }
}
