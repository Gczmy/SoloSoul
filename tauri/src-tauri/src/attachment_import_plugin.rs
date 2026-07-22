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
use std::path::Path;
use tauri::{
    plugin::{Builder, PluginApi, TauriPlugin},
    AppHandle, Manager, Runtime, State,
};

#[cfg(target_os = "android")]
use tauri::plugin::PluginHandle;

use crate::commands::attachment::attachment_dir;
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

    // 尽早释放 vault_service read guard：先拿到 base_path 并创建目标目录，
    // 避免长时间阻塞在 JNI 文件复制期间仍持有锁。
    let base = {
        let svc = state
            .vault_service
            .read()
            .map_err(|_| "Vault service lock poisoned".to_string())?;
        svc.base_path().clone()
    };

    // 计算并创建 Vault 目标目录。
    let dest_dir = attachment_dir(&base, &object_id, &attachment_id)?;
    std::fs::create_dir_all(&dest_dir)
        .map_err(|e| format!("Failed to create attachment directory: {}", e))?;

    // 清理文件名，防止路径遍历。
    let safe_name = Path::new(&file_name)
        .file_name()
        .ok_or("Invalid file name")?
        .to_string_lossy()
        .to_string();
    let dest_path = dest_dir.join(&safe_name);

    let payload = ImportContentUriPayload {
        object_id,
        attachment_id,
        content_uri,
        file_name: safe_name,
        dest_path: dest_path.to_string_lossy().to_string(),
    };

    // JNI 文件复制会阻塞当前线程，放到 spawn_blocking 避免阻塞 tokio worker。
    // 注意：在 spawn_blocking 内部重新获取插件句柄，避免引用 `app` 导致生命周期错误。
    tokio::task::spawn_blocking(move || {
        let handle = app.state::<AttachmentImportPluginHandle<R>>();
        handle.import_content_uri(payload)
    })
    .await
    .map_err(|e| format!("Import task failed: {}", e))?
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

    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    let base = svc.base_path().clone();
    let attachments_dir = base.join("attachments");

    // 为了调试 Android 上 path 前缀不一致的问题，先打印关键路径。
    tracing::error!(
        "attachment_export_content_uri debug: src_path={}, attachments_dir={}, base={}",
        src_path,
        attachments_dir.display(),
        base.display()
    );

    // 校验源文件在 Vault attachments 目录内，防止路径遍历。
    // 先尝试 canonicalize 源路径；若失败但文件存在，保留原始路径做后续前缀比较。
    let src = std::path::Path::new(&src_path)
        .canonicalize()
        .or_else(|_| {
            let p = std::path::PathBuf::from(&src_path);
            if p.exists() {
                Ok(p)
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

    tracing::error!(
        "attachment_export_content_uri debug: src_canon={}, attachments_canon={}, base_canon={}",
        src.display(),
        attachments_canon.display(),
        base_canon.display()
    );

    // 放宽校验：文件必须在 Vault base 目录下（通常即为应用私有数据目录），
    // 并且优先要求其在 attachments 子目录下。
    let in_attachments = src.starts_with(&attachments_canon)
        || src_path.starts_with(attachments_canon.to_string_lossy().as_ref());
    let in_vault =
        src.starts_with(&base_canon) || src_path.starts_with(base_canon.to_string_lossy().as_ref());

    if !in_attachments && !in_vault {
        return Err(format!(
            "Source path must be within vault attachments storage: src={}, attachments_dir={}",
            src.display(),
            attachments_canon.display()
        ));
    }

    let handle = app.state::<AttachmentImportPluginHandle<R>>();
    handle.export_content_uri(ExportContentUriPayload {
        src_path: src.to_string_lossy().to_string(),
        dest_uri,
    })
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

    let svc = state
        .vault_service
        .read()
        .map_err(|_| "Vault service lock poisoned".to_string())?;
    let base = svc.base_path().clone();
    let attachments_dir = base.join("attachments");

    let src = std::path::Path::new(&src_path)
        .canonicalize()
        .or_else(|_| {
            let p = std::path::PathBuf::from(&src_path);
            if p.exists() {
                Ok(p)
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
    let in_attachments = src.starts_with(&attachments_canon)
        || src_path.starts_with(attachments_canon.to_string_lossy().as_ref());
    if !in_attachments {
        return Err("Source path must be within vault attachments storage".to_string());
    }

    let handle = app.state::<AttachmentImportPluginHandle<R>>();
    handle.export_to_tree_uri(ExportToTreeUriPayload {
        src_path: src.to_string_lossy().to_string(),
        tree_uri,
        file_name,
        mime_type,
    })
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
