//! 共享工具函数。

/// 进程级共享 tokio 多线程运行时（首次调用时创建一次）。
///
/// 原先插件/同步/模型命令每次调用各自 `Runtime::new()`（含线程池创建
/// 与销毁），并发执行多个此类命令时资源重复。统一收敛为单例。
pub fn shared_runtime() -> &'static tokio::runtime::Runtime {
    static RUNTIME: std::sync::OnceLock<tokio::runtime::Runtime> = std::sync::OnceLock::new();
    RUNTIME.get_or_init(|| {
        tokio::runtime::Runtime::new().expect("创建共享 tokio 运行时失败（系统资源不足）")
    })
}

/// 将字符索引转换为字符串中的字节位置。
pub fn byte_position(s: &str, char_index: usize) -> usize {
    s.char_indices()
        .nth(char_index)
        .map(|(idx, _)| idx)
        .unwrap_or(s.len())
}
