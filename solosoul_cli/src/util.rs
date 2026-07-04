//! 共享工具函数。

/// 将字符索引转换为字符串中的字节位置。
pub fn byte_position(s: &str, char_index: usize) -> usize {
    s.char_indices()
        .nth(char_index)
        .map(|(idx, _)| idx)
        .unwrap_or(s.len())
}
