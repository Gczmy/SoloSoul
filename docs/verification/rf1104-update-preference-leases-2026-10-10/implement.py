from pathlib import Path
import json, hashlib

stage = Path(__file__).parent
root = Path('D:/SoloSoul')
source = root/'tauri/src-tauri/src/commands/update_preferences.rs'
receipt = json.loads((stage/'regression-red-fixed.receipt.json').read_text())
assert receipt['sourceUnchanged']
assert receipt['exitCode'] == 101
red_output = (stage/'regression-red-fixed.stdout.log').read_text(encoding='utf-8', errors='replace')
red_errors = (stage/'regression-red-fixed.stderr.log').read_text(encoding='utf-8', errors='replace')
assert 'IMPORT_OPERATIONS_ACTIVE' in red_errors and '1 failed' in red_output
red_sources = json.loads((stage/'regression-red-fixed.sources.json').read_text())
assert red_sources['tauri/src-tauri/src/commands/update_preferences.rs'] == hashlib.sha256(source.read_bytes()).hexdigest()
before = stage/'before/update_preferences.rs'
before.parent.mkdir(exist_ok=True)
assert not before.exists()
before.write_bytes(source.read_bytes())
text = source.read_text(encoding='utf-8')
text = text.replace('use std::{path::PathBuf, sync::Arc, time::Duration};', '''use std::{
    path::PathBuf,
    sync::{Arc, RwLock, Weak},
    time::Duration,
};''')
start = text.index('pub(super) struct SourcePreferences')
end = text.index("    pub fn preferred<'a>", start)
text = text[:start] + '''struct AccountSnapshot {
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
    let activity = Arc::new(solosoul_core::import_activity::begin_owned_root_activity(owner)?);
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

''' + text[end:]
start = text.index('    pub fn remember(&self,')
old = '    pub fn remember(&self, channel: Channel, url: &str, full_probe: bool) {\n'
replacement = '''    pub fn remember(&self, channel: Channel, url: &str, full_probe: bool) {
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
'''
assert text.count(old) == 1
text = text.replace(old, replacement).replace('if let Some((vault, id)) = &self.account {', 'if let Some((vault, id)) = account {')
source.write_bytes(text.replace('\n', '\r\n').encode('utf-8'))
print('Saved before bytes and applied RF-1104 production changes')
