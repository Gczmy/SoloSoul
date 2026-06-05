use std::ops::{Deref, DerefMut};
use subtle::ConstantTimeEq;
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

/// 安全字节数组（自动擦除）
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct SecureBytes {
    data: Vec<u8>,
}

impl SecureBytes {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }

    pub fn from_slice(data: &[u8]) -> Self {
        Self {
            data: data.to_vec(),
        }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }
}

impl Deref for SecureBytes {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl DerefMut for SecureBytes {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

/// 安全字符串（自动擦除）
pub type SecureString = Zeroizing<String>;

/// 安全内存擦除工具函数
pub fn secure_wipe<T: Zeroize>(data: &mut T) {
    data.zeroize();
}

/// 安全的内存比较（常量时间）
pub fn secure_compare(a: &[u8], b: &[u8]) -> bool {
    a.ct_eq(b).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secure_bytes() {
        let data = vec![0x42u8; 32];
        let secure = SecureBytes::new(data.clone());
        assert_eq!(secure.as_slice(), data.as_slice());
    }

    #[test]
    fn test_secure_string_drop() {
        let s = SecureString::from("secret_password".to_string());
        assert_eq!(s.as_str(), "secret_password");
        // After drop, memory should be zeroized (verified by ZeroizeOnDrop safety guarantee)
        drop(s);
    }

    #[test]
    fn test_secure_compare() {
        assert!(secure_compare(&[1u8, 2, 3], &[1u8, 2, 3]));
        assert!(!secure_compare(&[1u8, 2, 3], &[1u8, 2, 4]));
    }
}
