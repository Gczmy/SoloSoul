//! 设备同步偏好和设备备注统一放入账户 Profile 的加密 preferences。
//! 备注独立于 sync_peers 的握手信息，重新发现/配对不会覆盖用户名称。

use super::VaultStore;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DeviceSyncPreferences {
    pub auto_sync_enabled: bool,
    pub ui_prefs_sync_enabled: bool,
}

impl Default for DeviceSyncPreferences {
    fn default() -> Self {
        Self {
            auto_sync_enabled: false,
            ui_prefs_sync_enabled: true,
        }
    }
}

impl VaultStore {
    fn account_preference(&self, key: &str) -> Result<Option<serde_json::Value>, String> {
        let Some(profile) = self.load_profile(&self.config.account_id)? else {
            return Ok(None);
        };
        // 历史 Profile 允许非 JSON 数据；无 preferences 的旧记录不能阻断解锁/换钥恢复。
        // load_profile 仍传播解密失败，不能把损坏密文当成空配置。
        let Ok(data) = serde_json::from_slice::<serde_json::Value>(&profile.data) else {
            return Ok(None);
        };
        Ok(data.get("preferences").and_then(|p| p.get(key)).cloned())
    }

    pub fn device_sync_preferences(&self) -> Result<Option<DeviceSyncPreferences>, String> {
        self.account_preference("deviceSync")?
            .map(serde_json::from_value)
            .transpose()
            .map_err(|e| e.to_string())
    }

    pub fn update_device_sync_preferences(
        &self,
        update: impl FnOnce(&mut DeviceSyncPreferences),
    ) -> Result<DeviceSyncPreferences, String> {
        let mut result = DeviceSyncPreferences::default();
        self.update_profile_prefs(&self.config.account_id, |prefs| {
            result = prefs
                .get("deviceSync")
                .cloned()
                .map(serde_json::from_value)
                .transpose()
                .map_err(|e| e.to_string())?
                .unwrap_or_default();
            update(&mut result);
            prefs.insert(
                "deviceSync".into(),
                serde_json::to_value(&result).map_err(|e| e.to_string())?,
            );
            Ok(())
        })?;
        self.set_ui_prefs_sync_enabled(result.ui_prefs_sync_enabled);
        Ok(result)
    }

    pub fn device_names(&self) -> Result<HashMap<String, String>, String> {
        Ok(self
            .account_preference("deviceNames")?
            .map(serde_json::from_value)
            .transpose()
            .map_err(|e| e.to_string())?
            .unwrap_or_default())
    }

    /// 空名称移除备注，恢复自动名称；不更改对端身份和信任记录。
    pub fn set_device_name(&self, peer_id: &str, name: &str) -> Result<Option<String>, String> {
        let name = name.trim();
        if name.chars().count() > 64 || name.chars().any(char::is_control) {
            return Err("__SYNC_ERR__:invalid_device_name".into());
        }
        if self.load_peer_state(peer_id)?.is_none() {
            return Err("__SYNC_ERR__:unknown_device".into());
        }
        self.update_profile_prefs(&self.config.account_id, |prefs| {
            let names = prefs
                .entry("deviceNames")
                .or_insert_with(|| serde_json::json!({}))
                .as_object_mut()
                .ok_or("Invalid device names")?;
            if name.is_empty() {
                names.remove(peer_id);
            } else {
                names.insert(peer_id.into(), name.into());
            }
            Ok(())
        })?;
        Ok((!name.is_empty()).then(|| name.to_string()))
    }
}
