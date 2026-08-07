//! 共享工具函数。

/// 进程级共享 tokio 多线程运行时（首次调用时创建一次）。
///
/// 原先插件/同步/模型命令每次调用各自 `Runtime::new()`（含线程池创建
/// 与销毁），并发执行多个此类命令时资源重复。统一收敛为单例。
///
/// R2-V7：初始化失败返回 Err（而非 `expect` panic）——
/// 系统资源不足时由调用方优雅降级（error_message / 返回错误），不退出 TUI。
pub fn shared_runtime() -> color_eyre::Result<&'static tokio::runtime::Runtime> {
    static RUNTIME: std::sync::OnceLock<tokio::runtime::Runtime> = std::sync::OnceLock::new();
    RUNTIME.get().map(Ok).unwrap_or_else(|| {
        match tokio::runtime::Runtime::new() {
            Ok(rt) => match RUNTIME.set(rt) {
                Ok(()) => Ok(RUNTIME.get().expect("刚 set 成功必然可取回")),
                Err(_) => {
                    // 并发下其他线程已成功 set，直接取回即可。
                    Ok(RUNTIME.get().expect("并发 set 竞争后必存在"))
                }
            },
            Err(e) => Err(color_eyre::eyre::eyre!(
                "创建共享 tokio 运行时失败（系统资源不足）: {e}"
            )),
        }
    })
}

/// 以 0600 权限写入文件——创建时即定权限，避免默认 umask（通常 0644）下的
/// 明文窗口期。审计日志/诊断包等解密明文统一走此入口（P027/P028）。
pub fn write_private_file(path: &std::path::Path, contents: &[u8]) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)?;
        f.write_all(contents)?;
        Ok(())
    }
    #[cfg(not(unix))]
    {
        std::fs::write(path, contents)
    }
}

/// 将字符索引转换为字符串中的字节位置。
pub fn byte_position(s: &str, char_index: usize) -> usize {
    s.char_indices()
        .nth(char_index)
        .map(|(idx, _)| idx)
        .unwrap_or(s.len())
}
