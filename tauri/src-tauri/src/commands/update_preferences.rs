//! 更新源偏好：账户内加密保存，登录前使用不含账户信息的本机缓存。
//! 偏好只调整已允许候选的顺序，不能向请求列表注入地址。

use super::settings::{resolve_ui_prefs_path, UI_PREFS_LOCK};
use crate::state::AppState;
use futures::{future::BoxFuture, stream::FuturesUnordered, StreamExt};
use serde::{Deserialize, Serialize};
use solosoul_vault::VaultStore;
use std::{path::PathBuf, sync::Arc, time::Duration};
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

pub(super) struct SourcePreferences {
    preferences: Preferences,
    cache: Option<PathBuf>,
    // 请求开始时绑定账户；网络等待期间切换账户，不能把结果写入新账户。
    account: Option<(Arc<VaultStore>, String)>,
    account_value: Option<serde_json::Value>,
}

impl SourcePreferences {
    pub fn load(app: &tauri::AppHandle) -> Self {
        let mut result = Self {
            preferences: Preferences::default(),
            cache: None,
            account: None,
            account_value: None,
        };
        let Some(state) = app.try_state::<AppState>() else {
            return result;
        };
        let Ok(svc) = state.vault_service.read() else {
            return result;
        };
        result.cache = resolve_ui_prefs_path(app, &svc).ok();
        result.account = svc.get_vault_store().zip(svc.get_current_account());
        let stored = result.account.as_ref().and_then(|(vault, id)| {
            vault.load_profile(id).ok().flatten().and_then(|profile| {
                serde_json::from_slice::<serde_json::Value>(&profile.data)
                    .ok()?
                    .get("preferences")?
                    .get(PREF_KEY)
                    .cloned()
            })
        });
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
        // 两种清单可能并发完成，必须按键合并最新配置，不能覆盖另一通道。
        if let Some((vault, id)) = &self.account {
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
        let mut prefs = SourcePreferences {
            preferences: Preferences::default(),
            cache: None,
            account: None,
            account_value: None,
        };
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
        let cache = dir.path().join("ui_preferences.json");
        std::fs::write(&cache, r#"{"theme":"dark"}"#).unwrap();
        let config = solosoul_vault::VaultConfig::new("account-a", dir.path().to_path_buf())
            .with_data_key([7; 32]);
        let vault = Arc::new(VaultStore::open(config.clone()).unwrap());
        let prefs = SourcePreferences {
            preferences: Preferences::default(),
            cache: Some(cache.clone()),
            account: Some((vault.clone(), "account-a".into())),
            account_value: None,
        };
        prefs.remember(
            Channel::Manifest,
            "https://mirror.example/latest.json",
            true,
        );
        prefs.remember(Channel::Release, "https://other.example/release.json", true);
        let before = vault.load_profile("account-a").unwrap().unwrap();
        let data: serde_json::Value = serde_json::from_slice(&before.data).unwrap();
        let value = data["preferences"][PREF_KEY].clone();
        let unchanged = SourcePreferences {
            preferences: serde_json::from_value(value.clone()).unwrap(),
            cache: Some(cache.clone()),
            account: Some((vault.clone(), "account-a".into())),
            account_value: Some(value),
        };
        unchanged.remember(
            Channel::Release,
            "https://other.example/release.json",
            false,
        );
        assert_eq!(
            vault.load_profile("account-a").unwrap().unwrap().version,
            before.version
        );
        drop(unchanged);
        drop(prefs);
        drop(vault);
        let vault = VaultStore::open(config).unwrap();
        let profile = vault.load_profile("account-a").unwrap().unwrap();
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
        assert!(!cached.to_string().contains("account-a"));
    }
}
