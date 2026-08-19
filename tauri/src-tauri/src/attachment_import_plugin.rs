/**
 * Android content:// URI 附件导入/导出插件。
 *
 * Tauri `plugin-dialog` 在 Android 上返回/接受的是 `content://` URI，而 `plugin-fs` 的
 * `copy_file` 无法直接处理这种 URI。该插件在 Kotlin 端通过 `ContentResolver`
 * 在 Vault 目录与 content URI 之间流式复制文件，避免前端先把大文件读进内存。
 *
 * 前端：
 * - 上传检测到 `content://` 时调用 `attachment_import_content_uri`。
 * - 下载目标为 `content://` 时调用 `attachment_export_content_uri`。
 */
use serde::{Deserialize, Serialize};
use tauri::{
    plugin::{Builder, PluginApi, TauriPlugin},
    AppHandle, Manager, Runtime, State,
};

#[cfg(target_os = "android")]
use tauri::plugin::PluginHandle;

use crate::commands::attachment::{attachment_dir, path_within_base};
use crate::commands::vault_handle;
use crate::state::AppState;

/// Android 插件包名。
#[cfg(target_os = "android")]
const PLUGIN_IDENTIFIER: &str = "com.solosoul.app";

/// 调用 Kotlin 插件时传入的参数。
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportContentUriPayload {
    pub object_id: String,
    pub attachment_id: String,
    pub content_uri: String,
    pub file_name: String,
    pub dest_path: String,
}

/// Kotlin 插件返回的结果。
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportContentUriResult {
    pub vault_path: String,
    pub size_bytes: u64,
    /// Android 端通过 ContentResolver 查询到的真实显示名称。
    /// 当 content URI 的路径段不是真实文件名时，该字段提供正确的附件名。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

/// 调用 Kotlin 导出命令时传入的参数。
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportContentUriPayload {
    pub src_path: String,
    pub dest_uri: String,
}

/// 调用 Kotlin 通用复制命令时传入的参数。
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyContentUriPayload {
    pub content_uri: String,
    pub dest_path: String,
}

/// Kotlin 通用复制命令的返回结果。
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyContentUriResult {
    pub local_path: String,
    pub size_bytes: u64,
}

/// 调用 Kotlin 打开文件命令时传入的参数。
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenFilePayload {
    pub path: String,
    pub mime_type: String,
}

/// 调用 Kotlin 导出到 tree URI 命令时传入的参数。
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportToTreeUriPayload {
    pub src_path: String,
    pub tree_uri: String,
    pub file_name: String,
    pub mime_type: String,
}

/// pickTreeUri 命令的返回结果。
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PickTreeUriResult {
    /// 用户选择的 SAF tree URI，取消时为 None。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
}

/// SAF 目录同步命令的参数。
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncDirPayload {
    pub local_dir: String,
    pub tree_uri: String,
}

/// WorkManager 后台同步兜底调度参数。
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleFallbackSyncPayload {
    pub local_dir: String,
    pub tree_uri: String,
}

/// 插件句柄包装，便于在 command 中通过 Tauri state 获取。
pub struct AttachmentImportPluginHandle<R: Runtime> {
    #[cfg(target_os = "android")]
    handle: PluginHandle<R>,
    #[cfg(not(target_os = "android"))]
    _phantom: std::marker::PhantomData<fn() -> R>,
}

impl<R: Runtime> AttachmentImportPluginHandle<R> {
    /// 在 Android 端通过 ContentResolver 把 content:// URI 复制到 Vault 目录。
    /// 非 Android 平台直接返回不支持错误。
    pub fn import_content_uri(
        &self,
        payload: ImportContentUriPayload,
    ) -> Result<ImportContentUriResult, String> {
        #[cfg(target_os = "android")]
        {
            self.handle
                .run_mobile_plugin("importContentUri", payload)
                .map_err(|e| e.to_string())
                .and_then(|v| {
                    serde_json::from_value::<ImportContentUriResult>(v).map_err(|e| e.to_string())
                })
        }
        #[cfg(not(target_os = "android"))]
        {
            let _ = payload;
            Err("attachment_import_content_uri is only supported on Android".to_string())
        }
    }

    /// 在 Android 端通过 FileProvider 用系统默认应用打开本地文件。
    /// 非 Android 平台直接返回不支持错误。
    pub fn open_file(&self, payload: OpenFilePayload) -> Result<(), String> {
        #[cfg(target_os = "android")]
        {
            self.handle
                .run_mobile_plugin::<serde_json::Value>("openFile", payload)
                .map(|_| ())
                .map_err(|e| e.to_string())
        }
        #[cfg(not(target_os = "android"))]
        {
            let _ = payload;
            Err("attachment_open_file is only supported on Android".to_string())
        }
    }

    /// 在 Android 端通过系统分享面板（ACTION_SEND + FileProvider）转发本地文件。
    /// 非 Android 平台直接返回不支持错误。
    pub fn share_file(&self, payload: OpenFilePayload) -> Result<(), String> {
        #[cfg(target_os = "android")]
        {
            self.handle
                .run_mobile_plugin::<serde_json::Value>("shareFile", payload)
                .map(|_| ())
                .map_err(|e| e.to_string())
        }
        #[cfg(not(target_os = "android"))]
        {
            let _ = payload;
            Err("attachment_share is only supported on Android".to_string())
        }
    }

    /// 在 Android 端把 Vault 中的本地文件复制到 content:// URI。
    /// 非 Android 平台直接返回不支持错误。
    pub fn export_content_uri(&self, payload: ExportContentUriPayload) -> Result<(), String> {
        #[cfg(target_os = "android")]
        {
            self.handle
                .run_mobile_plugin::<serde_json::Value>("exportContentUri", payload)
                .map(|_| ())
                .map_err(|e| e.to_string())
        }
        #[cfg(not(target_os = "android"))]
        {
            let _ = payload;
            Err("attachment_export_content_uri is only supported on Android".to_string())
        }
    }

    /// 在 Android 端通过 ContentResolver 把 content:// URI 复制到任意本地路径。
    /// 非 Android 平台直接返回不支持错误。
    pub fn copy_content_uri_to_file(
        &self,
        payload: CopyContentUriPayload,
    ) -> Result<CopyContentUriResult, String> {
        #[cfg(target_os = "android")]
        {
            self.handle
                .run_mobile_plugin("copyContentUriToFile", payload)
                .map_err(|e| e.to_string())
                .and_then(|v| {
                    serde_json::from_value::<CopyContentUriResult>(v).map_err(|e| e.to_string())
                })
        }
        #[cfg(not(target_os = "android"))]
        {
            let _ = payload;
            Err("copy_content_uri_to_path is only supported on Android".to_string())
        }
    }

    /// 在 Android 端启动系统 SAF 目录选择器（Intent.ACTION_OPEN_DOCUMENT_TREE）。
    /// 返回用户选择的 tree URI（取消时为 None）。
    /// 非 Android 平台直接返回不支持错误。
    pub fn pick_tree_uri(&self) -> Result<PickTreeUriResult, String> {
        #[cfg(target_os = "android")]
        {
            // pickTreeUri 命令不读取 payload，() 序列化为 null 不影响 Kotlin 端
            self.handle
                .run_mobile_plugin::<PickTreeUriResult>("pickTreeUri", ())
                .map_err(|e| e.to_string())
        }
        #[cfg(not(target_os = "android"))]
        {
            Err("attachment_pick_tree_uri is only supported on Android".to_string())
        }
    }

    /// 在 Android 端检查 SAF tree URI 是否仍然可访问（授权未被撤销）。
    /// 返回 `{ accessible: bool }`。
    /// 非 Android 平台直接返回 false（无法验证）。
    pub fn check_vault_dir_access(&self, tree_uri: &str) -> Result<bool, String> {
        #[cfg(target_os = "android")]
        {
            let payload = serde_json::json!({ "treeUri": tree_uri });
            self.handle
                .run_mobile_plugin::<serde_json::Value>("checkVaultDirAccess", payload)
                .map_err(|e| e.to_string())
                .map(|v| {
                    v.get("accessible")
                        .and_then(|a| a.as_bool())
                        .unwrap_or(false)
                })
        }
        #[cfg(not(target_os = "android"))]
        {
            let _ = tree_uri;
            Ok(false)
        }
    }

    /// 在 Android 端启动 Vault 目录选择器（ACTION_OPEN_DOCUMENT_TREE），
    /// 返回用户选择的 tree URI（取消时为 None）。
    pub fn pick_vault_dir(&self) -> Result<PickTreeUriResult, String> {
        #[cfg(target_os = "android")]
        {
            self.handle
                .run_mobile_plugin::<PickTreeUriResult>("pickVaultDir", ())
                .map_err(|e| e.to_string())
        }
        #[cfg(not(target_os = "android"))]
        {
            Err("vault_pick_directory is only supported on Android".to_string())
        }
    }

    /// 在 Android 端把本地目录递归同步到 SAF tree URI 目录。
    /// 非 Android 平台直接返回不支持错误。
    pub fn sync_dir_to_remote(&self, local_dir: &str, tree_uri: &str) -> Result<(), String> {
        #[cfg(target_os = "android")]
        {
            let payload = SyncDirPayload {
                local_dir: local_dir.to_string(),
                tree_uri: tree_uri.to_string(),
            };
            self.handle
                .run_mobile_plugin::<serde_json::Value>("syncDirToRemote", payload)
                .map(|_| ())
                .map_err(|e| e.to_string())
        }
        #[cfg(not(target_os = "android"))]
        {
            let _ = local_dir;
            let _ = tree_uri;
            Err("sync_dir_to_remote is only supported on Android".to_string())
        }
    }

    /// 在 Android 端从 SAF tree URI 目录递归同步到本地目录。
    /// 非 Android 平台直接返回不支持错误。
    pub fn sync_dir_from_remote(&self, local_dir: &str, tree_uri: &str) -> Result<(), String> {
        #[cfg(target_os = "android")]
        {
            let payload = SyncDirPayload {
                local_dir: local_dir.to_string(),
                tree_uri: tree_uri.to_string(),
            };
            self.handle
                .run_mobile_plugin::<serde_json::Value>("syncDirFromRemote", payload)
                .map(|_| ())
                .map_err(|e| e.to_string())
        }
        #[cfg(not(target_os = "android"))]
        {
            let _ = local_dir;
            let _ = tree_uri;
            Err("sync_dir_from_remote is only supported on Android".to_string())
        }
    }

    /// 在 Android 端调度 WorkManager 周期性后台同步兜底任务。
    /// 非 Android 平台直接返回不支持错误。
    pub fn schedule_fallback_sync(&self, local_dir: &str, tree_uri: &str) -> Result<(), String> {
        #[cfg(target_os = "android")]
        {
            let payload = ScheduleFallbackSyncPayload {
                local_dir: local_dir.to_string(),
                tree_uri: tree_uri.to_string(),
            };
            self.handle
                .run_mobile_plugin::<serde_json::Value>("scheduleFallbackSync", payload)
                .map(|_| ())
                .map_err(|e| e.to_string())
        }
        #[cfg(not(target_os = "android"))]
        {
            let _ = local_dir;
            let _ = tree_uri;
            Err("schedule_fallback_sync is only supported on Android".to_string())
        }
    }

    /// 在 Android 端取消 WorkManager 周期性后台同步兜底任务。
    /// 非 Android 平台直接返回不支持错误。
    pub fn cancel_fallback_sync(&self) -> Result<(), String> {
        #[cfg(target_os = "android")]
        {
            self.handle
                .run_mobile_plugin::<serde_json::Value>("cancelFallbackSync", ())
                .map(|_| ())
                .map_err(|e| e.to_string())
        }
        #[cfg(not(target_os = "android"))]
        {
            Err("cancel_fallback_sync is only supported on Android".to_string())
        }
    }

    /// 在 Android 端把 Vault 中的本地文件导出到 SAF tree URI 目录。
    /// 非 Android 平台直接返回不支持错误。
    pub fn export_to_tree_uri(&self, payload: ExportToTreeUriPayload) -> Result<(), String> {
        #[cfg(target_os = "android")]
        {
            self.handle
                .run_mobile_plugin::<serde_json::Value>("exportToTreeUri", payload)
                .map(|_| ())
                .map_err(|e| e.to_string())
        }
        #[cfg(not(target_os = "android"))]
        {
            let _ = payload;
            Err("attachment_export_to_tree_uri is only supported on Android".to_string())
        }
    }
}

/// 初始化插件：注册 Android Kotlin 插件并将句柄存入 state。
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("attachment-import")
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
    let handle = api.register_android_plugin(PLUGIN_IDENTIFIER, "AttachmentImportPlugin")?;
    app.manage(AttachmentImportPluginHandle { handle });
    Ok(())
}

#[cfg(not(target_os = "android"))]
fn register_plugin<R: Runtime>(
    app: &AppHandle<R>,
    _api: PluginApi<R, ()>,
) -> Result<(), Box<dyn std::error::Error>> {
    app.manage(AttachmentImportPluginHandle {
        _phantom: std::marker::PhantomData::<fn() -> R>,
    });
    Ok(())
}

/// 从 Android content:// URI 直接导入附件到 Vault。
///
/// 流程：
/// 1. 校验 Vault 已解锁。
/// 2. 计算目标 Vault 路径并创建目录。
/// 3. 调用 Kotlin 插件通过 ContentResolver 流式复制文件。
/// 4. 返回 vault_path 与 size_bytes，供前端调用 `attachment_save` 写入元数据。
#[tauri::command]
pub async fn attachment_import_content_uri<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    object_id: String,
    attachment_id: String,
    content_uri: String,
    file_name: String,
) -> Result<ImportContentUriResult, String> {
    // Vault 必须处于解锁状态。
    let _vault = vault_handle(&state)?;

    // 尽早释放 vault_service read guard：先拿到 base_path + 附件密钥并创建目标目录，
    // 避免长时间阻塞在 JNI 文件复制期间仍持有锁。
    // P001: 附件密钥用于复制完成后就地加密（vault 内附件加密落盘）。
    let (base, att_key) = {
        let svc = state
            .vault_service
            .read()
            .map_err(|_| "Vault service lock poisoned".to_string())?;
        let key = svc
            .attachment_encryption_key()
            .map_err(|e| format!("无法获取附件密钥: {}", e))?;
        let key_arr: [u8; 32] = key
            .as_slice()
            .try_into()
            .map_err(|_| "附件密钥长度错误".to_string())?;
        (svc.base_path().clone(), key_arr)
    };

    // 计算并创建 Vault 目标目录。
    let dest_dir = attachment_dir(&base, &object_id, &attachment_id)?;
    std::fs::create_dir_all(&dest_dir)
        .map_err(|e| format!("Failed to create attachment directory: {}", e))?;

    // 清理文件名，防止路径遍历（P023：统一走共享实现，平台无关拒绝 `/` `\\`）。
    let safe_name = solosoul_core::path_util::sanitize_file_name(&file_name)?;
    let dest_path = dest_dir.join(&safe_name);

    let payload = ImportContentUriPayload {
        object_id,
        attachment_id,
        content_uri,
        file_name: safe_name.clone(),
        dest_path: dest_path.to_string_lossy().to_string(),
    };

    // JNI 文件复制会阻塞当前线程，放到 spawn_blocking 避免阻塞 tokio worker。
    // 注意：在 spawn_blocking 内部重新获取插件句柄，避免引用 `app` 导致生命周期错误。
    let result = tokio::task::spawn_blocking(move || {
        let handle = app.state::<AttachmentImportPluginHandle<R>>();
        handle.import_content_uri(payload)
    })
    .await
    .map_err(|e| format!("Import task failed: {}", e))??;

    // P001: Kotlin 复制到 dest_path 的是明文——立即就地加密（读明文 → 写密文临时 → 原子替换），
    // 消除明文落盘窗口。旧数据为密文时跳过（幂等）。
    if !solosoul_core::attachment_crypto::is_encrypted_file(&dest_path) {
        let tmp_path = dest_dir.join(format!("{}.enc.tmp", safe_name));
        solosoul_core::attachment_crypto::encrypt_file_stream(&att_key, &dest_path, &tmp_path)
            .map_err(|e| format!("附件落盘加密失败: {}", e))?;
        std::fs::rename(&tmp_path, &dest_path).map_err(|e| format!("附件加密替换失败: {}", e))?;
    }

    Ok(result)
}

/// 把 Vault 中的附件文件直接导出到 Android content:// URI。
///
/// 流程：
/// 1. 校验 Vault 已解锁。
/// 2. 校验源文件位于 Vault attachments 目录内。
/// 3. 调用 Kotlin 插件通过 ContentResolver 流式复制到目标 URI。
#[tauri::command]
pub async fn attachment_export_content_uri<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    src_path: String,
    dest_uri: String,
) -> Result<(), String> {
    // Vault 必须处于解锁状态。
    let _vault = vault_handle(&state)?;

    let (base, att_key) = {
        let svc = state
            .vault_service
            .read()
            .map_err(|_| "Vault service lock poisoned".to_string())?;
        let key = svc
            .attachment_encryption_key()
            .map_err(|e| format!("无法获取附件密钥: {}", e))?;
        let key_arr: [u8; 32] = key
            .as_slice()
            .try_into()
            .map_err(|_| "附件密钥长度错误".to_string())?;
        (svc.base_path().clone(), key_arr)
    };
    let attachments_dir = base.join("attachments");

    // 校验源文件在 Vault attachments 目录内，防止路径遍历。
    // 先尝试 canonicalize 源路径；若失败但文件存在，降级使用原始路径（Android symlink 兜底）。
    let (src, src_canonicalized) = std::path::Path::new(&src_path)
        .canonicalize()
        .map(|p| (p, true))
        .or_else(|_| {
            let p = std::path::PathBuf::from(&src_path);
            if p.exists() {
                Ok((p, false))
            } else {
                Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "source path does not exist",
                ))
            }
        })
        .map_err(|e| format!("Invalid source path: {}", e))?;

    // canonicalize attachments 目录；若失败则回退到原始路径。
    let attachments_canon = attachments_dir
        .canonicalize()
        .unwrap_or_else(|_| attachments_dir.clone());
    let base_canon = base.canonicalize().unwrap_or_else(|_| base.clone());

    // P003: 组件级比较（复用 path_within_base），杜绝字符串前缀匹配绕过——
    // 字面路径（如 `.../attachments_x/../../secret`）仅在 canonicalize 失败时参与判定，
    // 且同样按组件比较，防止共享前缀的兄弟目录（`attachments_evil`）通过校验。
    let src_raw = std::path::Path::new(&src_path);
    let in_attachments = path_within_base(
        &src,
        src_raw,
        src_canonicalized,
        &attachments_canon,
        &attachments_dir,
    );
    let in_vault = path_within_base(&src, src_raw, src_canonicalized, &base_canon, &base);

    if !in_attachments && !in_vault {
        // P019：错误消息不携带完整文件路径（防用户路径泄露进插件/日志）。
        return Err("Source path must be within vault attachments storage".to_string());
    }

    // P001: Kotlin ContentResolver 只能读明文——先把 vault 密文解密到临时明文再交给插件。
    let temp_dir = std::env::temp_dir().join("solosoul_export");
    std::fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("Failed to prepare export dir: {}", e))?;
    let temp_src = temp_dir.join(format!("{}.tmp", uuid::Uuid::new_v4()));
    solosoul_core::attachment_crypto::copy_decrypt_file(&att_key, &src, &temp_src)
        .map_err(|e| format!("Failed to decrypt for export: {}", e))?;

    let handle = app.state::<AttachmentImportPluginHandle<R>>();
    let result = handle.export_content_uri(ExportContentUriPayload {
        src_path: temp_src.to_string_lossy().to_string(),
        dest_uri,
    });
    // 清理临时明文（成功/失败都清理）。
    let _ = std::fs::remove_file(&temp_src);
    result
}

/// 把 Vault 中的附件文件导出到 Android SAF tree URI 目录。
///
/// 流程：
/// 1. 校验 Vault 已解锁。
/// 2. 校验源文件位于 Vault attachments 目录内。
/// 3. 调用 Kotlin 插件通过 ContentResolver 在目标目录下创建文件并流式复制。
#[tauri::command]
pub async fn attachment_export_tree_uri<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    src_path: String,
    tree_uri: String,
    file_name: String,
    mime_type: String,
) -> Result<(), String> {
    // Vault 必须处于解锁状态。
    let _vault = vault_handle(&state)?;

    let (base, att_key) = {
        let svc = state
            .vault_service
            .read()
            .map_err(|_| "Vault service lock poisoned".to_string())?;
        let key = svc
            .attachment_encryption_key()
            .map_err(|e| format!("无法获取附件密钥: {}", e))?;
        let key_arr: [u8; 32] = key
            .as_slice()
            .try_into()
            .map_err(|_| "附件密钥长度错误".to_string())?;
        (svc.base_path().clone(), key_arr)
    };
    let attachments_dir = base.join("attachments");

    let (src, src_canonicalized) = std::path::Path::new(&src_path)
        .canonicalize()
        .map(|p| (p, true))
        .or_else(|_| {
            let p = std::path::PathBuf::from(&src_path);
            if p.exists() {
                Ok((p, false))
            } else {
                Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "source path does not exist",
                ))
            }
        })
        .map_err(|e| format!("Invalid source path: {}", e))?;

    let attachments_canon = attachments_dir
        .canonicalize()
        .unwrap_or_else(|_| attachments_dir.clone());
    // P003: 组件级比较，杜绝字符串前缀匹配绕过。
    let src_raw = std::path::Path::new(&src_path);
    let in_attachments = path_within_base(
        &src,
        src_raw,
        src_canonicalized,
        &attachments_canon,
        &attachments_dir,
    );
    if !in_attachments {
        return Err("Source path must be within vault attachments storage".to_string());
    }

    // P001: Kotlin ContentResolver 只能读明文——先把 vault 密文解密到临时明文再交给插件。
    let temp_dir = std::env::temp_dir().join("solosoul_export");
    std::fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("Failed to prepare export dir: {}", e))?;
    let temp_src = temp_dir.join(format!("{}.tmp", uuid::Uuid::new_v4()));
    solosoul_core::attachment_crypto::copy_decrypt_file(&att_key, &src, &temp_src)
        .map_err(|e| format!("Failed to decrypt for export: {}", e))?;

    let handle = app.state::<AttachmentImportPluginHandle<R>>();
    let result = handle.export_to_tree_uri(ExportToTreeUriPayload {
        src_path: temp_src.to_string_lossy().to_string(),
        tree_uri,
        file_name,
        mime_type,
    });
    // 清理临时明文（成功/失败都清理）。
    let _ = std::fs::remove_file(&temp_src);
    result
}

/// 把 Android content:// URI 复制到应用缓存目录下的本地路径（通用中转，不绑定 Vault）。
///
/// 用于导入包中转等场景：前端先基于 appCacheDir 生成目标路径，
/// 再由该命令通过 ContentResolver 流式复制。安全校验：目标路径必须位于应用缓存目录内。
#[tauri::command]
pub async fn copy_content_uri_to_path<R: Runtime>(
    app: AppHandle<R>,
    content_uri: String,
    dest_path: String,
) -> Result<CopyContentUriResult, String> {
    let cache_dir = app
        .path()
        .app_cache_dir()
        .map_err(|e| format!("Failed to resolve app cache dir: {}", e))?;

    // 防御性前缀校验：dest_path 应始终由前端基于 appCacheDir 生成，
    // 但命令可被任意前端代码调用，故限制写入范围在缓存目录内。
    // 目标文件尚不存在，无法 canonicalize，比较其父目录的规范化路径。
    let dest = std::path::PathBuf::from(&dest_path);
    let parent = dest.parent().ok_or("Invalid dest path")?;
    let parent_canon = parent
        .canonicalize()
        .unwrap_or_else(|_| parent.to_path_buf());
    let cache_canon = cache_dir
        .canonicalize()
        .unwrap_or_else(|_| cache_dir.clone());
    if !parent_canon.starts_with(&cache_canon) {
        return Err("Destination path must be within app cache directory".to_string());
    }

    let payload = CopyContentUriPayload {
        content_uri,
        dest_path,
    };

    // JNI 文件复制会阻塞当前线程，放到 spawn_blocking 避免阻塞 tokio worker。
    tokio::task::spawn_blocking(move || {
        let handle = app.state::<AttachmentImportPluginHandle<R>>();
        handle.copy_content_uri_to_file(payload)
    })
    .await
    .map_err(|e| format!("Copy task failed: {}", e))?
}

/// 启动 Android SAF 目录选择器（Intent.ACTION_OPEN_DOCUMENT_TREE）。
///
/// 返回用户选择的 tree URI（取消时 uri 字段为 None）。
/// 桌面端/iOS 返回不支持错误。
#[tauri::command]
pub async fn attachment_pick_tree_uri<R: Runtime>(
    app: AppHandle<R>,
) -> Result<PickTreeUriResult, String> {
    let handle = app.state::<AttachmentImportPluginHandle<R>>();
    handle.pick_tree_uri()
}

/// 启动 Android SAF Vault 目录选择器。
///
/// 返回用户选择的 tree URI（取消时 uri 字段为 None）。
/// 桌面端/iOS 返回不支持错误。
#[tauri::command]
pub async fn vault_pick_directory<R: Runtime>(
    app: AppHandle<R>,
) -> Result<PickTreeUriResult, String> {
    let handle = app.state::<AttachmentImportPluginHandle<R>>();
    handle.pick_vault_dir()
}
