//! PDFium 动态库加载封装
//!
//! 供 OCR、PDF 水印等需要 PDFium 的功能复用。

use pdfium_render::prelude::*;
use std::path::PathBuf;
use std::sync::Mutex;

static PDFIUM: Mutex<Option<&'static Pdfium>> = Mutex::new(None);

fn pdfium_dylib_filename() -> &'static str {
    if cfg!(target_os = "macos") {
        "libpdfium.dylib"
    } else if cfg!(target_os = "windows") {
        "pdfium.dll"
    } else {
        "libpdfium.so"
    }
}

fn try_find_bundled_pdfium() -> Option<PathBuf> {
    // 1. 优先使用调用方通过环境变量显式指定的路径（Tauri 侧通常从 RESOURCE_DIR 设置）。
    if let Ok(path) = std::env::var("PDFIUM_LIBRARY_PATH") {
        let p = PathBuf::from(path);
        if p.exists() {
            return Some(p);
        }
    }

    // 2. 非 Tauri 调用者：尝试从当前可执行文件位置推断打包资源目录。
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            #[cfg(target_os = "macos")]
            {
                let candidate = exe_dir.parent().map(|p| {
                    p.join("Resources")
                        .join("pdfium")
                        .join(pdfium_dylib_filename())
                });
                if candidate.as_ref().map(|p| p.exists()).unwrap_or(false) {
                    return candidate;
                }
            }
            #[cfg(target_os = "windows")]
            {
                let candidate = exe_dir.join("pdfium").join(pdfium_dylib_filename());
                if candidate.exists() {
                    return Some(candidate);
                }
            }
            #[cfg(target_os = "linux")]
            {
                let candidate = exe_dir.join("pdfium").join(pdfium_dylib_filename());
                if candidate.exists() {
                    return Some(candidate);
                }
            }
        }
    }

    // 3. 兼容开发环境：从当前工作目录的 resources/pdfium/ 子目录查找。
    let filename = pdfium_dylib_filename();
    let candidates: [PathBuf; 4] = [
        PathBuf::from("resources/pdfium").join(filename),
        PathBuf::from("resources").join(filename),
        PathBuf::from(filename),
        PathBuf::from("src-tauri/resources/pdfium").join(filename),
    ];
    candidates.iter().find(|p| p.exists()).cloned()
}

fn do_init_pdfium() -> Result<Pdfium, String> {
    let bundled = try_find_bundled_pdfium();
    let bindings = if let Some(path) = bundled {
        Pdfium::bind_to_library(path)
    } else {
        Pdfium::bind_to_system_library()
    }
    .map_err(|e| format!("无法加载 PDFium: {e}"))?;

    Ok(Pdfium::new(bindings))
}

fn store_pdfium(pdfium: Pdfium) -> &'static Pdfium {
    let leaked: &'static Pdfium = Box::leak(Box::new(pdfium));
    leaked
}

/// 初始化 PDFium 绑定。
///
/// 优先加载打包的动态库；未找到时尝试绑定系统库。
/// 同一进程内仅初始化一次，后续调用返回同一实例的静态引用。
pub fn init_pdfium() -> Result<&'static Pdfium, String> {
    let mut guard = PDFIUM.lock().map_err(|_| "PDFium 锁被污染".to_string())?;
    if let Some(pdfium) = *guard {
        return Ok(pdfium);
    }
    let pdfium = do_init_pdfium()?;
    let leaked = store_pdfium(pdfium);
    *guard = Some(leaked);
    Ok(leaked)
}
