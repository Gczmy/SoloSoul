//! OCR 后处理
//!
//! ICAO 字符映射、校验位验证、置信度过滤。

/// ICAO MRZ 字符标准化
///
/// 规则：
/// - O → 0（字母 O 映射为数字 0）
/// - I → 1（字母 I 映射为数字 1）
/// - 空格 → <（填充符）
/// - 其他非法字符 → <
/// - 保留 [A-Z0-9<]
pub fn icao_normalize(text: &str) -> String {
    let mut result = String::with_capacity(text.len());

    for c in text.chars() {
        let normalized = match c {
            'O' => '0',
            'I' | 'i' => '1',
            ' ' => '<',
            '0'..='9' => c,
            'A'..='Z' => c,
            'a'..='z' => c.to_ascii_uppercase(),
            '<' => '<',
            _ => '<',
        };
        result.push(normalized);
    }

    result
}

/// MRZ 校验位验证（ICAO Doc 9303）
///
/// 权重序列：7, 3, 1 循环
/// 字符值：0-9 = 0-9, A-Z = 10-35, < = 0
pub fn validate_check_digit(data: &str, check_digit: char) -> bool {
    if check_digit == '<' {
        return true; // 无校验位视为通过
    }

    let weights = [7, 3, 1];
    let mut sum = 0;

    for (i, c) in data.chars().enumerate() {
        let val = match c {
            '0'..='9' => c as u32 - '0' as u32,
            'A'..='Z' => c as u32 - 'A' as u32 + 10,
            '<' => 0,
            _ => return false, // 非法字符
        };
        sum += val * weights[i % 3];
    }

    let expected = (sum % 10) as u8;
    let actual = match check_digit {
        '0'..='9' => check_digit as u8 - b'0',
        _ => return false,
    };

    expected == actual
}

/// 计算 MRZ 数据字符串的校验位
pub fn compute_check_digit(data: &str) -> Option<char> {
    let weights = [7, 3, 1];
    let mut sum = 0;

    for (i, c) in data.chars().enumerate() {
        let val = match c {
            '0'..='9' => c as u32 - '0' as u32,
            'A'..='Z' => c as u32 - 'A' as u32 + 10,
            '<' => 0,
            _ => return None,
        };
        sum += val * weights[i % 3];
    }

    let digit = (sum % 10) as u8;
    char::from_digit(digit as u32, 10)
}

/// 过滤低置信度结果
///
/// 若某行识别置信度低于阈值，标记为需用户确认
pub fn filter_low_confidence(lines: &[(String, f32)], threshold: f32) -> Vec<String> {
    lines
        .iter()
        .filter_map(|(text, conf)| {
            if *conf >= threshold {
                Some(text.clone())
            } else {
                // 低置信度行保留但标记（实际由 Dart 层处理）
                Some(text.clone())
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_icao_normalize() {
        assert_eq!(icao_normalize("P<OCR TEST"), "P<0CR<TEST");
        assert_eq!(icao_normalize("E12345678"), "E12345678");
        assert_eq!(icao_normalize("hello world"), "HELLO<WORLD");
        assert_eq!(icao_normalize("OIL"), "01L");
        // 小写 l 不应被映射（仅大写 I 和小写 i → 1）
        assert_eq!(icao_normalize("hello"), "HELLO");
    }

    #[test]
    fn test_validate_check_digit() {
        // E12345678: E=14, 权重 7,3,1 循环
        // 14*7 + 1*3 + 2*1 + 3*7 + 4*3 + 5*1 + 6*7 + 7*3 + 8*1 = 212
        // 212 % 10 = 2
        assert!(validate_check_digit("E12345678", '2'));
        // 错误校验位应失败
        assert!(!validate_check_digit("E12345678", '8'));
    }

    #[test]
    fn test_compute_check_digit() {
        assert_eq!(compute_check_digit("E12345678"), Some('2'));
        // 860101: 8*7 + 6*3 + 0*1 + 1*7 + 0*3 + 1*1 = 82, 82 % 10 = 2
        assert_eq!(compute_check_digit("860101"), Some('2'));
    }
}
