//! Android Keystore 生物识别凭证存储插件桥。
//!
//! 移动端（Android）通过该插件将主密钥交给 Android Keystore 中
//! 受生物识别保护的 AES 密钥加密，并持久化密文与 IV。
//! 密钥生成时启用 setInvalidatedByBiometricEnrollment(true)，
//! 当用户新增/删除指纹或人脸时，旧密钥会永久失效。
//!
//! 与 tauri-plugin-biometric 不同，本插件将生物识别提示与加解密
//! 操作绑定在同一个 CryptoObject 中完成，确保每次使用密钥都必须
//! 经过用户生物识别授权。

use serde::{Deserialize, Serialize};
use tauri::{
    plugin::{Builder, PluginApi, TauriPlugin},
    AppHandle, Manager, Runtime,
};

#[cfg(target_os = "android")]
use tauri::plugin::PluginHandle;

#[cfg(target_os = "android")]
const PLUGIN_IDENTIFIER: &str = "com.solosoul.app";

/// 单槽 Keystore 加密结果。
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeystoreSlot {
    pub iv: String,
    pub ciphertext: String,
}

/// 双槽凭证存储：strong（CryptoObject 授权绑定密钥，Class 3）与
/// weak（免授权密钥 + 普通提示，Class 2）相互独立，可同时存在。
/// 向后兼容：旧版扁平格式 `{iv, ciphertext}` 反序列化为 strong 槽。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeystoreCredentials {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strong: Option<KeystoreSlot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weak: Option<KeystoreSlot>,
}

impl KeystoreCredentials {
    pub fn is_empty(&self) -> bool {
        self.strong.is_none() && self.weak.is_none()
    }
}

impl<'de> Deserialize<'de> for KeystoreCredentials {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let v = serde_json::Value::deserialize(deserializer)?;
        if v.get("iv").is_some() {
            // 旧版扁平格式 → strong 槽
            let slot: KeystoreSlot = serde_json::from_value(v).map_err(serde::de::Error::custom)?;
            Ok(Self {
                strong: Some(slot),
                weak: None,
            })
        } else {
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Helper {
                #[serde(default)]
                strong: Option<KeystoreSlot>,
                #[serde(default)]
                weak: Option<KeystoreSlot>,
            }
            let h = Helper::deserialize(v).map_err(serde::de::Error::custom)?;
            Ok(Self {
                strong: h.strong,
                weak: h.weak,
            })
        }
    }
}

/// 兼容别名：旧代码中的单槽密文类型。
pub type KeystoreCiphertext = KeystoreSlot;

/// Android 生物识别可用性信息。
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailabilityInfo {
    pub strong_available: bool,
    pub weak_available: bool,
    /// 诊断字段（Kotlin 侧返回，排查 Class 2 人脸设备检测问题）
    #[serde(default)]
    pub sdk_int: Option<i64>,
    #[serde(default)]
    pub face_feature: Option<bool>,
    #[serde(default)]
    pub strong_raw: Option<i64>,
    #[serde(default)]
    pub weak_raw: Option<i64>,
}

/// 生物识别提示信息。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BiometricPromptInfo<'a> {
    pub title: &'a str,
    pub subtitle: &'a str,
    pub cancel_title: &'a str,
}

/// 插件句柄包装，便于在 command 中通过 Tauri state 获取。
pub struct KeystorePluginHandle<R: Runtime> {
    #[cfg(target_os = "android")]
    handle: PluginHandle<R>,
    #[cfg(not(target_os = "android"))]
    _phantom: std::marker::PhantomData<fn() -> R>,
}

impl<R: Runtime> KeystorePluginHandle<R> {
    /// 通过生物识别提示加密数据。
    /// `authenticator`："weak" 强制 Class 2 路径；None/"strong" 走 Class 3 优先。
    pub fn authenticate_and_save(
        &self,
        alias: &str,
        data: &str,
        prompt: BiometricPromptInfo<'_>,
        authenticator: Option<&str>,
    ) -> Result<KeystoreSlot, String> {
        #[cfg(target_os = "android")]
        {
            #[derive(Debug, Clone, Serialize)]
            struct Payload<'a> {
                alias: &'a str,
                data: &'a str,
                title: &'a str,
                subtitle: &'a str,
                cancel_title: &'a str,
                #[serde(skip_serializing_if = "Option::is_none")]
                authenticator: Option<&'a str>,
            }
            self.handle
                .run_mobile_plugin::<serde_json::Value>(
                    "authenticateAndSave",
                    Payload {
                        alias,
                        data,
                        title: prompt.title,
                        subtitle: prompt.subtitle,
                        cancel_title: prompt.cancel_title,
                        authenticator,
                    },
                )
                .map_err(|e| e.to_string())
                .and_then(|v| serde_json::from_value::<KeystoreSlot>(v).map_err(|e| e.to_string()))
        }
        #[cfg(not(target_os = "android"))]
        {
            let _ = (alias, data, prompt, authenticator);
            Err("Keystore storage is only supported on Android".to_string())
        }
    }

    /// 通过生物识别提示解密数据。
    /// `authenticator`："weak" 仅 Class 2 提示；"any" 指纹/人脸皆可；None/"strong" 走 Class 3。
    pub fn authenticate_and_read(
        &self,
        alias: &str,
        iv: &str,
        ciphertext: &str,
        prompt: BiometricPromptInfo<'_>,
        authenticator: Option<&str>,
    ) -> Result<String, String> {
        #[cfg(target_os = "android")]
        {
            #[derive(Debug, Clone, Serialize)]
            struct Payload<'a> {
                alias: &'a str,
                iv: &'a str,
                ciphertext: &'a str,
                title: &'a str,
                subtitle: &'a str,
                cancel_title: &'a str,
                #[serde(skip_serializing_if = "Option::is_none")]
                authenticator: Option<&'a str>,
            }
            #[derive(Debug, Clone, Deserialize)]
            struct Wrapper {
                data: String,
            }
            self.handle
                .run_mobile_plugin::<serde_json::Value>(
                    "authenticateAndRead",
                    Payload {
                        alias,
                        iv,
                        ciphertext,
                        title: prompt.title,
                        subtitle: prompt.subtitle,
                        cancel_title: prompt.cancel_title,
                        authenticator,
                    },
                )
                .map_err(|e| e.to_string())
                .and_then(|v| serde_json::from_value::<Wrapper>(v).map_err(|e| e.to_string()))
                .map(|w| w.data)
        }
        #[cfg(not(target_os = "android"))]
        {
            let _ = (alias, iv, ciphertext, prompt, authenticator);
            Err("Keystore storage is only supported on Android".to_string())
        }
    }

    /// 检查 Android 设备的生物识别可用性（强 + 弱）。
    pub fn check_biometric_availability(&self) -> Result<AvailabilityInfo, String> {
        #[cfg(target_os = "android")]
        {
            self.handle
                .run_mobile_plugin::<serde_json::Value>(
                    "checkBiometricAvailability",
                    serde_json::json!({}),
                )
                .map_err(|e| e.to_string())
                .and_then(|v| {
                    serde_json::from_value::<AvailabilityInfo>(v).map_err(|e| format!("parse: {e}"))
                })
        }
        #[cfg(not(target_os = "android"))]
        {
            Ok(AvailabilityInfo {
                strong_available: false,
                weak_available: false,
                sdk_int: None,
                face_feature: None,
                strong_raw: None,
                weak_raw: None,
            })
        }
    }

    /// 检查 Keystore 中指定密钥别名是否真实存在。
    /// `authenticator`："weak" 查询 `{alias}_weak`；否则查询主别名。
    /// 用途：卸载/换机后密钥已被系统擦除，但 keystore_data.json 可能
    /// 从 SAF 同步残留，用于识别并清理"幽灵开启"的陈旧凭证。
    pub fn key_exists(&self, alias: &str, authenticator: Option<&str>) -> Result<bool, String> {
        #[cfg(target_os = "android")]
        {
            #[derive(Debug, Clone, Serialize)]
            struct Payload<'a> {
                alias: &'a str,
                #[serde(skip_serializing_if = "Option::is_none")]
                authenticator: Option<&'a str>,
            }
            #[derive(Debug, Clone, Deserialize)]
            struct Wrapper {
                exists: bool,
            }
            self.handle
                .run_mobile_plugin::<serde_json::Value>(
                    "keyExists",
                    Payload {
                        alias,
                        authenticator,
                    },
                )
                .map_err(|e| e.to_string())
                .and_then(|v| serde_json::from_value::<Wrapper>(v).map_err(|e| e.to_string()))
                .map(|w| w.exists)
        }
        #[cfg(not(target_os = "android"))]
        {
            let _ = (alias, authenticator);
            Err("Keystore storage is only supported on Android".to_string())
        }
    }

    /// 删除 Keystore 中的密钥别名。
    /// `authenticator`："weak" 只删 `{alias}_weak`；否则只删主别名。
    pub fn delete(&self, alias: &str, authenticator: Option<&str>) -> Result<(), String> {
        #[cfg(target_os = "android")]
        {
            #[derive(Debug, Clone, Serialize)]
            struct Payload<'a> {
                alias: &'a str,
                #[serde(skip_serializing_if = "Option::is_none")]
                authenticator: Option<&'a str>,
            }
            self.handle
                .run_mobile_plugin::<serde_json::Value>(
                    "delete",
                    Payload {
                        alias,
                        authenticator,
                    },
                )
                .map(|_| ())
                .map_err(|e| e.to_string())
        }
        #[cfg(not(target_os = "android"))]
        {
            let _ = (alias, authenticator);
            Err("Keystore storage is only supported on Android".to_string())
        }
    }
}

/// 初始化插件：注册 Android Kotlin 插件并将句柄存入 state。
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("biometric-keystore")
        .setup(|_app, api| {
            register_plugin::<R>(_app, api)?;
            Ok(())
        })
        .build()
}

#[cfg(target_os = "android")]
fn register_plugin<R: Runtime>(
    app: &AppHandle<R>,
    api: PluginApi<R, ()>,
) -> Result<(), Box<dyn std::error::Error>> {
    let handle = api.register_android_plugin(PLUGIN_IDENTIFIER, "BiometricKeystorePlugin")?;
    app.manage(KeystorePluginHandle { handle });
    Ok(())
}

#[cfg(not(target_os = "android"))]
fn register_plugin<R: Runtime>(
    app: &AppHandle<R>,
    _api: PluginApi<R, ()>,
) -> Result<(), Box<dyn std::error::Error>> {
    app.manage(KeystorePluginHandle {
        _phantom: std::marker::PhantomData::<fn() -> R>,
    });
    Ok(())
}
