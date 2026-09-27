//! PDFium 动态库加载封装
//!
//! 供 OCR、PDF 水印等需要 PDFium 的功能复用。

use pdfium_render::prelude::*;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

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

/// 一次 PDFium 原生操作的独占访问权。
///
/// PDFium 的原生 API 不支持并发调用，锁必须覆盖文档、页面、文本、位图等对象
/// 的使用和析构。调用方应在同步作用域内先取得 guard，再创建这些原生对象；
/// 不得在持有 guard 时再次初始化，也不得跨越 await 保留 guard。
pub struct PdfiumGuard {
    guard: MutexGuard<'static, Option<&'static Pdfium>>,
}

impl Deref for PdfiumGuard {
    type Target = Pdfium;

    fn deref(&self) -> &Self::Target {
        // 仅 init_pdfium 能构造 guard，且返回前已经完成初始化。
        self.guard
            .as_ref()
            .expect("PDFium guard must be initialized")
    }
}

/// 初始化 PDFium 绑定并取得本次操作的独占访问权。
///
/// 优先加载打包的动态库；未找到时尝试绑定系统库。
/// 同一进程内仅初始化一次，后续调用复用同一实例并等待前一次操作释放 guard。
pub fn init_pdfium() -> Result<PdfiumGuard, String> {
    let mut guard = PDFIUM.lock().map_err(|_| "PDFium 锁被污染".to_string())?;
    if guard.is_none() {
        *guard = Some(store_pdfium(do_init_pdfium()?));
    }
    Ok(PdfiumGuard { guard })
}

#[cfg(test)]
mod rf907_tests;
