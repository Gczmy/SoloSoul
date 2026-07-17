//! Android Keystore 生物识别凭证存储插件桥。
//!
//! 移动端（Android）通过该插件将主密钥交给 Android Keystore 中
//! 受生物识别保护的 AES 密钥加密，并持久化密文与 IV。
//! 密钥生成时启用 setInvalidatedByBiometricEnrollment(true)，
//! 当用户新增/删除指纹或人脸时，旧密钥会永久失效。

use serde::{Deserialize, Serialize};
use tauri::{
    plugin::{Builder, PluginApi, TauriPlugin},
    AppHandle, Manager, Runtime,
};

#[cfg(target_os = "android")]
use tauri::plugin::PluginHandle;

#[cfg(target_os = "android")]
const PLUGIN_IDENTIFIER: &str = "com.solosoul.app";

/// Keystore 加密结果。
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeystoreCiphertext {
    pub iv: String,
    pub ciphertext: String,
}

/// 插件句柄包装，便于在 command 中通过 Tauri state 获取。
pub struct KeystorePluginHandle<R: Runtime> {
    #[cfg(target_os = "android")]
    handle: PluginHandle<R>,
    #[cfg(not(target_os = "android"))]
    _phantom: std::marker::PhantomData<fn() -> R>,
}

impl<R: Runtime> KeystorePluginHandle<R> {
    /// 使用 Android Keystore 加密数据。
    pub fn save(&self, alias: &str, data: &str) -> Result<KeystoreCiphertext, String> {
        #[cfg(target_os = "android")]
        {
            #[derive(Debug, Clone, Serialize)]
            struct Payload<'a> {
                alias: &'a str,
                data: &'a str,
            }
            self.handle
                .run_mobile_plugin::<serde_json::Value>("save", Payload { alias, data })
                .map_err(|e| e.to_string())
                .and_then(|v| {
                    serde_json::from_value::<KeystoreCiphertext>(v).map_err(|e| e.to_string())
                })
        }
        #[cfg(not(target_os = "android"))]
        {
            let _ = (alias, data);
            Err("Keystore storage is only supported on Android".to_string())
        }
    }

    /// 使用 Android Keystore 解密数据。
    pub fn read(&self, alias: &str, iv: &str, ciphertext: &str) -> Result<String, String> {
        #[cfg(target_os = "android")]
        {
            #[derive(Debug, Clone, Serialize)]
            struct Payload<'a> {
                alias: &'a str,
                iv: &'a str,
                ciphertext: &'a str,
            }
            #[derive(Debug, Clone, Deserialize)]
            struct Wrapper {
                data: String,
            }
            self.handle
                .run_mobile_plugin::<serde_json::Value>(
                    "read",
                    Payload {
                        alias,
                        iv,
                        ciphertext,
                    },
                )
                .map_err(|e| e.to_string())
                .and_then(|v| serde_json::from_value::<Wrapper>(v).map_err(|e| e.to_string()))
                .map(|w| w.data)
        }
        #[cfg(not(target_os = "android"))]
        {
            let _ = (alias, iv, ciphertext);
            Err("Keystore storage is only supported on Android".to_string())
        }
    }

    /// 删除 Keystore 中的密钥别名。
    pub fn delete(&self, alias: &str) -> Result<(), String> {
        #[cfg(target_os = "android")]
        {
            #[derive(Debug, Clone, Serialize)]
            struct Payload<'a> {
                alias: &'a str,
            }
            self.handle
                .run_mobile_plugin::<serde_json::Value>("delete", Payload { alias })
                .map(|_| ())
                .map_err(|e| e.to_string())
        }
        #[cfg(not(target_os = "android"))]
        {
            let _ = alias;
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
