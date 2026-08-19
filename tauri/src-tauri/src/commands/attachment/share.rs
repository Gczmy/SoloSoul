//! 平台分享：macOS 分享面板 / Windows 分享面板（降级 reveal）/ Linux reveal / Android 系统分享（P047 拆分）。

use super::*;

/// 复制附件到共享临时目录 `solosoul_share/`，返回复制后的目标路径。
///
/// 桌面端（macOS/Windows/Linux）分享前统一走此副本逻辑：分享副本而非 vault 原文件，
/// 避免把用户带进隐藏的 vault 目录、也避免用户误改 vault 内文件。
///
/// 将附件复制到指定目录（分享副本），返回最终目标路径。
///
/// - 清理文件名，防止路径遍历（file_name 来自 vault 元数据，不可直接 join）。
/// - `file_name()` 对 "." / ".." 原样返回，显式拒绝避免写入目录之外。
/// - 同名冲突：不同对象的附件可能同名（如对象1/对象2各有 "2"），若直接用
///   文件名作为目标会互相覆盖（后分享的覆盖先分享的，且临时目录不自动清理）。
///   复用下载路径的 `make_unique_dest_path` 去重：已存在同名时生成
///   a(1).pdf / a(2).pdf 序号副本，保证分享副本互不覆盖。
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub(crate) fn copy_into_dir(
    base_dir: &Path,
    path: &Path,
    file_name: &str,
    att_key: &[u8; 32],
) -> Result<PathBuf, String> {
    std::fs::create_dir_all(base_dir).map_err(|e| format!("Failed to prepare directory: {}", e))?;
    // P023: 统一走 solosoul_core::path_util::sanitize_file_name（平台无关拒绝
    // `/` `\\` 分隔符 + 取末段兜底 + 拒绝空/`.`/`..`），修复旧实现不拒反斜杠缺口。
    let safe_name = solosoul_core::path_util::sanitize_file_name(file_name)?;
    let dest = make_unique_dest_path(&base_dir.join(safe_name));
    // P001: vault 内附件加密落盘，分享副本需解密（SOLC 密文自动解密，旧明文直拷）。
    solosoul_core::attachment_crypto::copy_decrypt_file(att_key, path, &dest)
        .map_err(|e| format!("Failed to copy file for sharing: {}", e))?;
    Ok(dest)
}

/// P010: 清理分享临时目录内的旧副本文件（分享面板/reveal 用完后无保留价值，
/// 下次分享前清掉，避免 `temp_dir()/solosoul_share/` 明文残留无限累积）。
/// 目录本身保留（后续 copy_into_dir 会复用）；仅删除文件不递归（分享副本为平铺文件）。
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub(crate) fn cleanup_share_dir(dir: &Path) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() {
                let _ = std::fs::remove_file(&p);
            }
        }
    }
}

/// 分享副本落盘目录（系统临时目录）。P010: 复制前先清理上次分享的旧副本。
#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn copy_to_share_dir(path: &Path, file_name: &str, att_key: &[u8; 32]) -> Result<PathBuf, String> {
    let dir = std::env::temp_dir().join("solosoul_share");
    cleanup_share_dir(&dir);
    copy_into_dir(&dir, path, file_name, att_key)
}

/// 转发附件到其他应用。
///
/// - Android：系统分享面板（`ACTION_SEND` + FileProvider），可直发微信等应用。
/// - 桌面端：复制附件到临时目录 `solosoul_share/` 后 `opener::reveal` 在文件管理器中
///   显示，由用户自行拖入目标应用——复制副本而非 vault 原文件，避免把用户带进隐藏的
///   vault 目录、也避免用户误改 vault 内文件。
/// - iOS：不支持（返回明确错误）。
#[tauri::command]
pub async fn attachment_share<R: Runtime>(
    #[allow(unused_variables)] app: AppHandle<R>,
    state: State<'_, AppState>,
    object_id: String,
    attachment_id: String,
) -> Result<(), String> {
    // 块作用域尽早释放 vault_service 读锁：复制/转发期间不占用锁，
    // 且保证 guard 在 spawn_blocking 的 await 点之前已销毁（async 状态机 Send 要求）。
    // P001: 附件密钥一并取出，分享副本需要解密（vault 内附件已加密落盘）。
    let (path, att, att_key) = {
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
        let (p, a) = resolve_verified_attachment_path(&svc, &object_id, &attachment_id)?;
        (p, a, key_arr)
    };

    #[cfg(target_os = "android")]
    {
        // P001: 分享给外部应用前解密到临时明文（FileProvider 无法读取 vault 密文）。
        let temp_dir = std::env::temp_dir().join(format!("solosoul_share_{}", object_id));
        std::fs::create_dir_all(&temp_dir)
            .map_err(|e| format!("Failed to prepare share dir: {}", e))?;
        // P010: 清理该对象目录下上次分享的旧副本明文，避免累积残留。
        if let Ok(entries) = std::fs::read_dir(&temp_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() {
                    let _ = std::fs::remove_file(&p);
                }
            }
        }
        let safe_name = solosoul_core::path_util::sanitize_file_name(&att.file_name)?;
        let temp_path = temp_dir.join(&safe_name);
        solosoul_core::attachment_crypto::copy_decrypt_file(&att_key, &path, &temp_path)
            .map_err(|e| format!("Failed to decrypt for share: {}", e))?;
        let handle = app.state::<AttachmentImportPluginHandle<R>>();
        handle.share_file(OpenFilePayload {
            path: temp_path.to_string_lossy().to_string(),
            mime_type: att.mime_type.clone(),
        })
    }

    #[cfg(target_os = "macos")]
    {
        share_macos(app, path, att.file_name, att_key).await
    }

    #[cfg(target_os = "windows")]
    {
        share_windows(app, path, att.file_name, att_key).await
    }

    #[cfg(all(
        not(target_os = "android"),
        not(target_os = "macos"),
        not(target_os = "windows"),
        not(target_os = "ios")
    ))]
    {
        share_linux(path, att.file_name, att_key).await
    }

    #[cfg(target_os = "ios")]
    {
        // 设计决策：iOS 不做转发（无现成原生分享插件先例），显式返回不支持。
        let _ = (path, att);
        Err("attachment_share is not supported on iOS".to_string())
    }
}

// ── P009：平台分享实现 ─────────────────────────────────────────────
// 拆分子函数消除 attachment_share 内各平台重复的「复制到分享目录 + 主线程调度 + oneshot」
// 骨架，共享 copy_to_share_dir_async / run_on_main_thread_oneshot 两个模板 helper。

/// 复制附件到分享临时目录（分享副本而非 vault 原文件），复制在 spawn_blocking 中执行，
/// 避免大文件复制阻塞 tokio worker。
/// N002：仅桌面三平台使用（macos/windows/linux）；iOS 显式不支持分享、Android 走自有分享链路，
/// 需 cfg 门控否则 Android/iOS 编译报 E0425、Linux CI 报 dead code。
#[cfg(not(any(target_os = "android", target_os = "ios")))]
async fn copy_to_share_dir_async(
    path: PathBuf,
    file_name: String,
    att_key: [u8; 32],
) -> Result<PathBuf, String> {
    tokio::task::spawn_blocking(move || copy_to_share_dir(&path, &file_name, &att_key))
        .await
        .map_err(|e| format!("Share copy task panicked: {}", e))?
}

/// 主线程调度 + oneshot 回传：平台分享 UI（AppKit/WinRT）必须运行在 UI 线程且非 Send，
/// 通过 run_on_main_thread 调度到主线程，闭包结果经 oneshot channel 回传。
/// N002：仅 macOS / Windows 使用（Linux reveal 无需主线程），需 cfg 门控否则 Android/iOS
/// 编译报 E0425、Linux CI 报 dead code。
#[cfg(any(target_os = "macos", target_os = "windows"))]
async fn run_on_main_thread_oneshot<R: Runtime, F>(app: AppHandle<R>, f: F) -> Result<(), String>
where
    F: FnOnce(AppHandle<R>) -> Result<(), String> + Send + 'static,
{
    let (tx, rx) = tokio::sync::oneshot::channel::<Result<(), String>>();
    let app_for_main = app.clone();
    app.run_on_main_thread(move || {
        let result = f(app_for_main);
        let _ = tx.send(result);
    })
    .map_err(|e| format!("Failed to schedule share on main thread: {}", e))?;
    rx.await
        .map_err(|e| format!("Share task failed: {}", e))??;
    Ok(())
}

#[cfg(target_os = "macos")]
async fn share_macos<R: Runtime>(
    app: AppHandle<R>,
    path: PathBuf,
    file_name: String,
    att_key: [u8; 32],
) -> Result<(), String> {
    use objc2::AnyThread;
    use objc2_app_kit::{NSSharingServicePicker, NSWindow};
    use objc2_foundation::{NSArray, NSRect, NSRectEdge, NSString, NSURL};
    use tauri::Manager;

    let dest = copy_to_share_dir_async(path, file_name, att_key).await?;

    // AppKit UI 必须在主线程执行，且 NSSharingServicePicker 不是 Send——
    // 通过 run_on_main_thread 调度到主线程，错误经 oneshot channel 回传。
    run_on_main_thread_oneshot(app, move |app| {
        let window = app
            .get_webview_window("main")
            .ok_or("Main window not found")?;
        let ns_window_ptr = window
            .ns_window()
            .map_err(|e| format!("Failed to get NSWindow: {}", e))?
            as *mut NSWindow;
        if ns_window_ptr.is_null() {
            return Err("NSWindow pointer is null".to_string());
        }
        // SAFETY: ptr 是 Tauri 通过 ns_window() 返回的有效 NSWindow 指针，已做非空检查；
        // Tauri 管理其生命周期，&*ptr 仅是借用引用（同 window.rs set_titlebar_color 模式）。
        let ns_window = unsafe { &*ns_window_ptr };
        let view = ns_window
            .contentView()
            .ok_or("NSWindow has no content view")?;

        let url: objc2::rc::Retained<NSURL> =
            NSURL::fileURLWithPath(&NSString::from_str(&dest.to_string_lossy()));
        let items: objc2::rc::Retained<NSArray> =
            NSArray::from_retained_slice(&[url.into_super().into()]);
        // SAFETY: initWithItems 的 unsafe 约束要求 items 元素类型正确（NSURL 可分享，
        // 符合 NSPasteboardWriting）；这里 items 仅含单个 NSURL，类型正确。
        let picker = unsafe {
            NSSharingServicePicker::initWithItems(NSSharingServicePicker::alloc(), &items)
        };
        picker.showRelativeToRect_ofView_preferredEdge(NSRect::ZERO, &view, NSRectEdge::MinY);
        // picker 必须保持存活直到分享面板关闭，否则面板会立即消失；
        // 泄漏引用（每次转发泄漏一个轻量对象，量级可忽略）。
        Box::leak(Box::new(picker));
        Ok(())
    })
    .await
}

#[cfg(target_os = "windows")]
async fn share_windows<R: Runtime>(
    app: AppHandle<R>,
    path: PathBuf,
    file_name: String,
    att_key: [u8; 32],
) -> Result<(), String> {
    use tauri::Manager;
    use windows::core::{Interface, Ref, HSTRING};
    use windows::ApplicationModel::DataTransfer::{
        DataPackage, DataRequestedEventArgs, DataTransferManager,
    };
    use windows::Foundation::TypedEventHandler;
    use windows::Storage::{IStorageItem, StorageFile};
    use windows::Win32::System::WinRT::RoGetActivationFactory;
    use windows::Win32::UI::Shell::IDataTransferManagerInterop;
    use windows_collections::IIterable;

    let dest = copy_to_share_dir_async(path, file_name, att_key).await?;

    // Windows 10 1809 以下不支持系统分享面板（ShowShareUIForWindow），降级为文件管理器显示
    if !DataTransferManager::IsSupported().unwrap_or(false) {
        opener::reveal(&dest).map_err(|e| format!("Failed to reveal file: {}", e))?;
        return Ok(());
    }

    // WinRT 分享面板：DataTransferManager 绑定 UI 线程且非 Send——
    // 通过 run_on_main_thread 调度，错误经 oneshot channel 回传。
    let dest_str = dest.to_string_lossy().to_string();
    run_on_main_thread_oneshot(app, move |app| {
        let window = app
            .get_webview_window("main")
            .ok_or("Main window not found")?;
        let hwnd = window
            .hwnd()
            .map_err(|e| format!("Failed to get HWND: {}", e))?;

        // SAFETY: RoGetActivationFactory 返回 Windows 内建激活工厂的
        // IDataTransferManagerInterop 接口（DataTransferManager 的 COM 工厂）。
        let interop: IDataTransferManagerInterop = unsafe {
            RoGetActivationFactory(&HSTRING::from(
                "Windows.ApplicationModel.DataTransfer.DataTransferManager",
            ))
        }
        .map_err(|e| format!("Failed to get DataTransferManager interop: {}", e))?;

        // SAFETY: GetForWindow 为指定窗口返回绑定的 DataTransferManager 实例，
        // 返回对象由 COM 管理生命周期。
        let dtm: DataTransferManager = unsafe { interop.GetForWindow(hwnd) }
            .map_err(|e| format!("GetForWindow failed: {}", e))?;

        // 注册 DataRequested 事件：用户在选择目标应用后，系统调用该处理器请求分享数据
        // 注意：windows-collections 的 IIterable 需通过 From<Vec<T::Default>> 构建，
        // 接口元素类型为 Option<IStorageItem>（T::Default = Option<T>）。
        let path_for_handler = dest_str.clone();
        let token = dtm
            .DataRequested(&TypedEventHandler::new(
                move |_sender: Ref<'_, DataTransferManager>,
                      args: Ref<'_, DataRequestedEventArgs>| {
                    let request = args.ok()?.Request()?;
                    let package = DataPackage::new()?;
                    let file =
                        StorageFile::GetFileFromPathAsync(&HSTRING::from(&path_for_handler))?
                            .get()?;
                    // StorageFile → IStorageItem（SetStorageItems 需要 IIterable<IStorageItem>）
                    let items: IIterable<IStorageItem> =
                        vec![Some(file.cast::<IStorageItem>()?)].into();
                    package.SetStorageItems(&items, false)?;
                    request.SetData(&package)?;
                    Ok(())
                },
            ))
            .map_err(|e| format!("DataRequested registration failed: {}", e))?;

        // 显示系统分享面板（Share Contract）
        // SAFETY: hwnd 是 Tauri 提供的有效窗口句柄，生命周期由 Tauri 管理。
        if let Err(e) = unsafe { interop.ShowShareUIForWindow(hwnd) } {
            // 与 IsSupported 降级路径一致：面板启动失败（Win10 特殊环境/分享组件异常）
            // 时降级为文件管理器显示，而不是直接报错中断用户操作。
            // token 是 i64（Copy），无需显式 drop；dtm 需要释放（不再泄漏）
            let _ = token;
            drop(dtm);
            tracing::warn!("ShowShareUIForWindow failed, falling back to reveal: {}", e);
            return opener::reveal(Path::new(&dest_str))
                .map_err(|r| format!("Failed to reveal file: {}", r));
        }

        // 保持 DataTransferManager 存活（分享面板触发 DataRequested 时事件源需要）；
        // token 只是注册句柄（i64），无需保留。泄漏引用——每次转发泄漏一个轻量 COM
        // 对象，量级可忽略（同 macOS 分支的 picker 泄漏策略）。
        Box::leak(Box::new(dtm));
        Ok(())
    })
    .await
}

#[cfg(all(
    not(target_os = "android"),
    not(target_os = "macos"),
    not(target_os = "windows"),
    not(target_os = "ios")
))]
async fn share_linux(path: PathBuf, file_name: String, att_key: [u8; 32]) -> Result<(), String> {
    // Linux：复制到临时目录后 reveal 在文件管理器中显示，避免把用户带进隐藏的
    // vault 目录、也避免误改 vault 内文件。复制走 copy_to_share_dir_async（spawn_blocking），
    // reveal 在 async 上下文直接调用（文件管理器调用本身非阻塞、无 spawn_blocking 需求）。
    let dest = copy_to_share_dir_async(path, file_name, att_key).await?;
    opener::reveal(&dest).map_err(|e| format!("Failed to reveal file: {}", e))
}
