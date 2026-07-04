use subtle::ConstantTimeEq;

/// 安全的内存比较（常量时间）
pub fn secure_compare(a: &[u8], b: &[u8]) -> bool {
    a.ct_eq(b).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secure_compare() {
        assert!(secure_compare(&[1u8, 2, 3], &[1u8, 2, 3]));
        assert!(!secure_compare(&[1u8, 2, 3], &[1u8, 2, 4]));
    }
}
