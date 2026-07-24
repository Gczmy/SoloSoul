//! 文件系统抽象模块（Tauri 应用层）

pub mod saf_sync_driver;
pub mod vault_file_system;

/// 规范化路径：移除 `./` 等冗余组件，不依赖文件系统（不会 canonicalize）。
///
/// 当路径包含 `./`（如 `resolve(".", ...)` 产生的结果）时，直接使用该路径
/// 可能在某些 Android 文件系统上触发 ENAMETOOLONG。此函数对路径进行纯计算
/// 级别的归一化，无需文件系统存在或可访问。
pub fn normalize_path(path: &std::path::Path) -> std::path::PathBuf {
    use std::path::Component;
    let mut out = std::path::PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {
                // 跳过 `./`
            }
            Component::RootDir => out.push(component.as_os_str()),
            _ => out.push(component.as_os_str()),
        }
    }
    // 如果路径被完全清空（例如路径仅为 `.`），回退到原始路径
    if out.as_os_str().is_empty() {
        path.to_path_buf()
    } else {
        out
    }
}
